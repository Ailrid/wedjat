"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
import torch.nn as nn
import torch.nn.functional as F


class MSLoss(nn.Module):
    def __init__(
        self,
        alpha: float = 2.0,
        beta: float = 50.0,
        base: float = 0.5,
        eps: float = 0.1,
    ):
        super(MSLoss, self).__init__()
        self.alpha = alpha
        self.beta = beta
        self.base = base
        self.eps = eps

    def forward(
        self, feat_anchor: torch.Tensor, feat_true_sample: torch.Tensor
    ) -> torch.Tensor:
        """
        Input Shapes:
            feat_anchor:       [B_total, D]        (where B_total = B * S)
            feat_true_sample:  [B_total, N, D]     (N = positive samples per anchor)

        Output Shape:
            loss: Scaled scalar tensor
        """
        b_total, d = feat_anchor.shape

        # 1. L2 normalize features
        feat_anchor = F.normalize(feat_anchor, p=2, dim=-1)
        feat_true_sample = F.normalize(feat_true_sample, p=2, dim=-1)

        # 2. Compute positive similarities: [B_total, N]
        sim_pos = torch.bmm(
            feat_anchor.unsqueeze(1), feat_true_sample.transpose(1, 2)
        ).squeeze(1)

        # 3. Compute negative similarities against all other anchors
        # sim_neg_full: [B_total, B_total]
        sim_neg = torch.matmul(feat_anchor, feat_anchor.T)

        # Mask out self-similarity
        diag_mask = torch.eye(b_total, dtype=torch.bool, device=feat_anchor.device)
        sim_neg = sim_neg.masked_fill(diag_mask, float("-inf"))

        # 4. Hard pair mining per anchor element
        max_neg, _ = torch.max(sim_neg, dim=1, keepdim=True)  # [B_total, 1]
        min_pos, _ = torch.min(sim_pos, dim=1, keepdim=True)  # [B_total, 1]

        # Multi-Similarity mining masks
        mask_pos_selected = (sim_pos < max_neg + self.eps).float()
        mask_neg_selected = (sim_neg > min_pos - self.eps).float()

        # Re-apply diagonal mask to ensure self-similarity is excluded
        mask_neg_selected = mask_neg_selected.masked_fill(diag_mask, 0.0)

        # 5. Calculate loss terms using exponential formulations
        pos_exp = torch.exp(-self.alpha * (sim_pos - self.base)) * mask_pos_selected
        neg_exp = torch.exp(self.beta * (sim_neg - self.base)) * mask_neg_selected

        # 6. Compute log(1 + sum(exp)) safely
        pos_loss = (1.0 / self.alpha) * torch.log(1.0 + torch.sum(pos_exp, dim=1))
        neg_loss = (1.0 / self.beta) * torch.log(1.0 + torch.sum(neg_exp, dim=1))

        return (pos_loss + neg_loss).mean()
