"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

from typing import Optional
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
        self.ce_loss = nn.CrossEntropyLoss(label_smoothing=0.1)
        self.alpha = alpha
        self.beta = beta
        self.base = base
        self.eps = eps

    def forward(
        self,
        feat_anchor: torch.Tensor,
        feat_true_sample: torch.Tensor,
        feat_class: Optional[torch.Tensor] = None,
        labels: Optional[torch.Tensor] = None,
    ) -> torch.Tensor:
        """
        Input Shapes:
            feat_anchor:       [B_total, D]
            feat_true_sample:  [B_total, N, D]
            feat_class:     [B_total, Num_Classes] or None
            labels:            [B, S] or None
        """
        if labels is not None:
            labels = labels.to(feat_anchor.device)

        b_total, d = feat_anchor.shape
        num_pos = feat_true_sample.shape[1]

        # Normalize features for metric loss calculation
        feat_anchor = F.normalize(feat_anchor, p=2, dim=-1)
        feat_true_sample = F.normalize(feat_true_sample, p=2, dim=-1)

        # Compute positive similarities
        sim_pos = torch.bmm(
            feat_anchor.unsqueeze(1),
            feat_true_sample.transpose(1, 2),
        ).squeeze(1)

        # Compute cross-view negative similarities
        drone_flat = feat_true_sample.view(b_total * num_pos, d)
        sim_matrix = torch.matmul(feat_anchor, drone_flat.T)

        # Exclude matching positive pairs from negative matrix
        pos_mask = torch.zeros(
            b_total,
            b_total * num_pos,
            dtype=torch.bool,
            device=feat_anchor.device,
        )
        for i in range(b_total):
            pos_mask[i, i * num_pos : (i + 1) * num_pos] = True

        sim_neg = sim_matrix.masked_fill(pos_mask, float("-inf"))

        # Select hard pairs for multi-similarity loss
        max_neg, _ = torch.max(sim_neg, dim=1, keepdim=True)
        min_pos, _ = torch.min(sim_pos, dim=1, keepdim=True)

        mask_pos_selected = (sim_pos < max_neg + self.eps).float()
        mask_neg_selected = (sim_neg > min_pos - self.eps).float()
        mask_neg_selected = mask_neg_selected.masked_fill(pos_mask, 0.0)

        # Compute exponential terms with numerical stability clamp
        pos_exp = torch.exp(-self.alpha * (sim_pos - self.base)) * mask_pos_selected
        neg_val = torch.clamp(self.beta * (sim_neg - self.base), max=80.0)
        neg_exp = torch.exp(neg_val) * mask_neg_selected

        # Compute multi-similarity loss
        pos_loss = (1.0 / self.alpha) * torch.log(1.0 + torch.sum(pos_exp, dim=1))
        neg_loss = (1.0 / self.beta) * torch.log(1.0 + torch.sum(neg_exp, dim=1))
        ms_loss = (pos_loss + neg_loss).mean()

        # Add cross entropy loss if classification logits and labels are provided
        if feat_class is not None and labels is not None:
            ce = self.ce_loss(feat_class, labels.reshape(-1))
            return ms_loss + ce

        return ms_loss
