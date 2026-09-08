"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
import torch.nn as nn
import torch.nn.functional as F


class LearnableFourierPositionalEncoding(nn.Module):

    def __init__(
        self, M: int, descriptor_dim: int, num_heads: int, gamma: float = 1.0
    ) -> None:
        super().__init__()
        self.num_heads = num_heads
        head_dim = descriptor_dim // num_heads
        self.Wr = nn.Linear(M, head_dim // 2, bias=False)
        self.gamma = gamma
        nn.init.normal_(self.Wr.weight.data, mean=0, std=self.gamma**-2)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Encode position vector using stack and cat to avoid Tile op on NPU."""
        projected = self.Wr(x)
        cosines, sines = torch.cos(projected), torch.sin(projected)
        emb = torch.stack([cosines, sines])  # Shape: (2, B, N, head_dim // 2)

        # Interleave cosines and sines at the last dimension
        emb = torch.stack([emb, emb], dim=-1).reshape(
            emb.shape[0], emb.shape[1], emb.shape[2], -1
        )  # Shape: (2, B, N, head_dim)

        # Use torch.cat instead of repeat to eliminate ONNX Tile node
        emb = torch.cat([emb] * self.num_heads, dim=-1).unsqueeze(-1)
        return emb


class SelfBlock(nn.Module):
    def __init__(self, embed_dim: int, num_heads: int, bias: bool = True) -> None:
        super().__init__()
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.head_dim = embed_dim // num_heads
        self.Wqkv = nn.Linear(embed_dim, 3 * embed_dim, bias=bias)
        self.out_proj = nn.Linear(embed_dim, embed_dim, bias=bias)
        self.ffn = nn.Sequential(
            nn.Linear(2 * embed_dim, 2 * embed_dim),
            nn.LayerNorm(2 * embed_dim, elementwise_affine=True),
            nn.GELU(),
            nn.Linear(2 * embed_dim, embed_dim),
        )

    def rotate_half(self, qk: torch.Tensor) -> torch.Tensor:
        b, n, _, _ = qk.shape
        qk = qk.reshape((b, n, self.num_heads, self.head_dim // 2, 2, 2))
        qk = torch.stack((-qk[..., 1, :], qk[..., 0, :]), dim=4)
        qk = qk.reshape((b, n, self.embed_dim, 2))
        return qk

    def apply_cached_rotary_emb(
        self, encoding: torch.Tensor, qk: torch.Tensor
    ) -> torch.Tensor:
        return qk * encoding[0] + self.rotate_half(qk) * encoding[1]

    def forward(self, x: torch.Tensor, encoding: torch.Tensor) -> torch.Tensor:
        b, n, _ = x.shape
        qkv: torch.Tensor = self.Wqkv(x)
        qkv = qkv.reshape((b, n, self.embed_dim, 3))
        qk, v = qkv[..., :2], qkv[..., 2]
        qk = self.apply_cached_rotary_emb(encoding, qk)
        q, k = qk[..., 0], qk[..., 1]

        head_dim = self.embed_dim // self.num_heads
        q = q.reshape(b, n, self.num_heads, head_dim).transpose(1, 2)
        k = k.reshape(b, n, self.num_heads, head_dim).transpose(1, 2)
        v = v.reshape(b, n, self.num_heads, head_dim).transpose(1, 2)

        context = (
            F.scaled_dot_product_attention(q, k, v)
            .transpose(1, 2)
            .reshape(b, n, self.embed_dim)
        )
        message = self.out_proj(context)
        return x + self.ffn(torch.cat([x, message], dim=2))


class CrossBlock(nn.Module):
    def __init__(self, embed_dim: int, num_heads: int, bias: bool = True) -> None:
        super().__init__()
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.head_dim = embed_dim // num_heads
        self.to_qk = nn.Linear(embed_dim, embed_dim, bias=bias)
        self.to_v = nn.Linear(embed_dim, embed_dim, bias=bias)
        self.to_out = nn.Linear(embed_dim, embed_dim, bias=bias)
        self.ffn = nn.Sequential(
            nn.Linear(2 * embed_dim, 2 * embed_dim),
            nn.LayerNorm(2 * embed_dim, elementwise_affine=True),
            nn.GELU(),
            nn.Linear(2 * embed_dim, embed_dim),
        )

    def forward(
        self, desc0: torch.Tensor, desc1: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor]:
        """Cross attention between two independent image descriptor streams."""
        num_keypoints = desc0.shape[1]
        qk0, v0 = self.to_qk(desc0), self.to_v(desc0)
        qk1, v1 = self.to_qk(desc1), self.to_v(desc1)

        head_dim = self.embed_dim // self.num_heads

        # Stream 0 attends to Stream 1
        q0 = qk0.reshape(1, num_keypoints, self.num_heads, head_dim).transpose(1, 2)
        k1 = qk1.reshape(1, num_keypoints, self.num_heads, head_dim).transpose(1, 2)
        v1_heads = v1.reshape(1, num_keypoints, self.num_heads, head_dim).transpose(
            1, 2
        )

        m0 = (
            F.scaled_dot_product_attention(q0, k1, v1_heads)
            .transpose(1, 2)
            .reshape(1, num_keypoints, self.embed_dim)
        )
        m0 = self.to_out(m0)
        desc0_out = desc0 + self.ffn(torch.cat([desc0, m0], dim=2))

        # Stream 1 attends to Stream 0
        q1 = qk1.reshape(1, num_keypoints, self.num_heads, head_dim).transpose(1, 2)
        k0 = qk0.reshape(1, num_keypoints, self.num_heads, head_dim).transpose(1, 2)
        v0_heads = v0.reshape(1, num_keypoints, self.num_heads, head_dim).transpose(
            1, 2
        )

        m1 = (
            F.scaled_dot_product_attention(q1, k0, v0_heads)
            .transpose(1, 2)
            .reshape(1, num_keypoints, self.embed_dim)
        )
        m1 = self.to_out(m1)
        desc1_out = desc1 + self.ffn(torch.cat([desc1, m1], dim=2))

        return desc0_out, desc1_out


class TransformerLayer(nn.Module):
    def __init__(self, embed_dim: int, num_heads: int) -> None:
        super().__init__()
        self.self_attn = SelfBlock(embed_dim, num_heads)
        self.cross_attn = CrossBlock(embed_dim, num_heads)

    def forward(
        self,
        desc0: torch.Tensor,
        desc1: torch.Tensor,
        enc0: torch.Tensor,
        enc1: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor]:
        desc0 = self.self_attn(desc0, enc0)
        desc1 = self.self_attn(desc1, enc1)
        desc0, desc1 = self.cross_attn(desc0, desc1)
        return desc0, desc1


def sigmoid_log_double_softmax_static(
    similarities: torch.Tensor, z0: torch.Tensor, z1: torch.Tensor
) -> torch.Tensor:
    """Compute assignment matrix without log operations."""
    scores0 = F.softmax(similarities, dim=2)
    scores1 = F.softmax(similarities, dim=1)
    certainties = torch.sigmoid(z0) * torch.sigmoid(z1).transpose(1, 2)
    return scores0 * scores1 * certainties


class MatchAssignmentStatic(nn.Module):
    def __init__(self, dim: int) -> None:
        super().__init__()
        self.scale = dim**0.25
        self.final_proj = nn.Linear(dim, dim, bias=True)
        self.matchability = nn.Linear(dim, 1, bias=True)

    def forward(self, desc0: torch.Tensor, desc1: torch.Tensor) -> torch.Tensor:
        """Build assignment matrix directly from stream inputs."""
        mdesc0 = self.final_proj(desc0) / self.scale
        mdesc1 = self.final_proj(desc1) / self.scale

        similarities = mdesc0 @ mdesc1.transpose(1, 2)

        z0 = self.matchability(desc0)
        z1 = self.matchability(desc1)

        scores = sigmoid_log_double_softmax_static(similarities, z0, z1)
        return scores


class LightGlue(nn.Module):
    def __init__(
        self,
        url: str = "https://github.com/cvg/LightGlue/releases/download/v0.1_arxiv/superpoint_lightglue.pth",
        input_dim: int = 256,
        descriptor_dim: int = 256,
        num_heads: int = 4,
        n_layers: int = 9,
    ) -> None:
        super().__init__()

        self.descriptor_dim = descriptor_dim
        self.num_heads = num_heads
        self.n_layers = n_layers

        if input_dim != self.descriptor_dim:
            self.input_proj = nn.Linear(input_dim, self.descriptor_dim, bias=True)
        else:
            self.input_proj = nn.Identity()

        self.posenc = LearnableFourierPositionalEncoding(
            2, self.descriptor_dim, self.num_heads
        )

        d, h, n = self.descriptor_dim, self.num_heads, self.n_layers
        self.transformers = nn.ModuleList([TransformerLayer(d, h) for _ in range(n)])
        self.log_assignment = nn.ModuleList(
            [MatchAssignmentStatic(d) for _ in range(n)]
        )

        # Load weights directly without structural mismatch
        state_dict = torch.hub.load_state_dict_from_url(url)
        for i in range(n):
            pattern = f"self_attn.{i}", f"transformers.{i}.self_attn"
            state_dict = {k.replace(*pattern): v for k, v in state_dict.items()}
            pattern = f"cross_attn.{i}", f"transformers.{i}.cross_attn"
            state_dict = {k.replace(*pattern): v for k, v in state_dict.items()}
        self.load_state_dict(state_dict, strict=False)

    def forward(
        self,
        kpts0: torch.Tensor,  # Shape: (1, N, 2)
        kpts1: torch.Tensor,  # Shape: (1, N, 2)
        descs0: torch.Tensor,  # Shape: (1, N, 256)
        descs1: torch.Tensor,  # Shape: (1, N, 256)
    ) -> torch.Tensor:
        """Forward pass taking 4 independent inputs with batch size 1."""
        descs0 = self.input_proj(descs0)
        descs1 = self.input_proj(descs1)

        enc0 = self.posenc(kpts0)
        enc1 = self.posenc(kpts1)

        for i in range(self.n_layers):
            descs0, descs1 = self.transformers[i](descs0, descs1, enc0, enc1)

        scores = self.log_assignment[-1](descs0, descs1)
        return scores
