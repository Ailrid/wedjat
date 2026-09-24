"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

from typing import List, Tuple
import torch
import torch.nn.functional as F
from metric.core.structs import RankMetric


class MetricEvaluator:

    def __init__(self, device: str = "cuda"):
        self.default_device = device
        self.reset()

    def reset(self):
        """Resets all internal storage for a new evaluation epoch."""
        self.anchors_list: List[torch.Tensor] = []
        self.samples_list: List[torch.Tensor] = []
        self.labels_list: List[torch.Tensor] = []

    @torch.no_grad()
    def update(
        self,
        feat_anchor: torch.Tensor,
        feat_true_sample: torch.Tensor,
        labels: torch.Tensor,
    ):
        """Accumulates embeddings and labels from mini-batches during evaluation.

        Inputs:
        feat_anchor: [B * S, D] - Batch of anchor features
        feat_true_sample: [B * S, N, D] or [B * S, D] - Batch of positive target sample
        labels: [B] or [B * S]
        """
        if feat_true_sample.dim() == 2:
            feat_true_sample = feat_true_sample.unsqueeze(1)

        labels_flat = labels.detach().cpu().reshape(-1)
        feat_anchor_cpu = feat_anchor.detach().cpu()
        feat_true_sample_cpu = feat_true_sample.detach().cpu()

        num_anchors = feat_anchor_cpu.shape[0]
        num_labels = labels_flat.shape[0]

        if num_anchors != num_labels:
            if num_anchors % num_labels == 0:
                scale = num_anchors // num_labels
                labels_flat = labels_flat.repeat_interleave(scale)
            else:
                raise ValueError(
                    f"Label count ({num_labels}) cannot be aligned with anchor count ({num_anchors})."
                )

        self.anchors_list.append(feat_anchor_cpu)
        self.samples_list.append(feat_true_sample_cpu)
        self.labels_list.append(labels_flat)

    @torch.no_grad()
    def compute(self) -> Tuple[RankMetric, str]:
        """Calculates global Rank-1 to Rank-5 metrics and returns evaluation summary."""
        if not self.anchors_list or not self.samples_list:
            raise ValueError("No batch samples were recorded.")

        all_anchors = torch.cat(self.anchors_list, dim=0)
        all_samples = torch.cat(self.samples_list, dim=0)
        all_labels = torch.cat(self.labels_list, dim=0).to(self.default_device)

        total_queries, num_pos_samples, dim = all_samples.shape

        all_anchors = F.normalize(all_anchors, p=2, dim=-1)
        all_samples = F.normalize(all_samples, p=2, dim=-1)

        all_samples_flat = all_samples.reshape(-1, dim)

        anchors_dev = all_anchors.to(self.default_device)
        samples_flat_dev = all_samples_flat.to(self.default_device)

        query_labels = all_labels
        gallery_labels = all_labels.repeat_interleave(num_pos_samples)

        sim_matrix = torch.matmul(anchors_dev, samples_flat_dev.T)

        k_max = min(5, sim_matrix.size(1))
        top5_indices = torch.topk(sim_matrix, k=k_max, dim=1).indices

        pred_labels = gallery_labels[top5_indices]
        correct_matches = pred_labels == query_labels.unsqueeze(1)

        rank_hits = (
            torch.cummax(correct_matches, dim=1).values.float().mean(dim=0) * 100.0
        )
        rank_accuracies = rank_hits.tolist()

        while len(rank_accuracies) < 5:
            last_val = rank_accuracies[-1] if rank_accuracies else 0.0
            rank_accuracies.append(last_val)

        metric = RankMetric(
            rank1=rank_accuracies[0],
            rank2=rank_accuracies[1],
            rank3=rank_accuracies[2],
            rank4=rank_accuracies[3],
            rank5=rank_accuracies[4],
        )

        summary_lines = [
            "=" * 45,
            "      CROSS-VIEW RETRIEVAL EVALUATION       ",
            "=" * 45,
            f"Total Query Features    : {total_queries}",
            f"Total Gallery Samples   : {all_samples_flat.size(0)}",
            "-" * 45,
            f"  Rank-1 Accuracy : {metric.rank1:6.2f}%",
            f"  Rank-2 Accuracy : {metric.rank2:6.2f}%",
            f"  Rank-3 Accuracy : {metric.rank3:6.2f}%",
            f"  Rank-4 Accuracy : {metric.rank4:6.2f}%",
            f"  Rank-5 Accuracy : {metric.rank5:6.2f}%",
            "=" * 45,
        ]

        report_str = "\n".join(summary_lines)
        return metric, report_str
