"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

import torch
import torch.nn.functional as F
from ..structs import Metric
import torch
import torch.nn.functional as F


class MetricEvaluator:

    def __init__(self, steps: int = 100, device: str = "cuda"):
        # Store thresholds tensor
        self.steps = steps
        self.default_device = device
        self.thresholds = torch.linspace(0.0, 1.0, steps=steps)
        self.reset()

    def reset(self):
        """Resets all internal accumulators for a new evaluation epoch."""
        self.total_pos = 0
        self.total_neg = 0
        self.tp_counts = torch.zeros(self.steps, dtype=torch.long)
        self.tn_counts = torch.zeros(self.steps, dtype=torch.long)

    @torch.no_grad()
    def update(self, feat_anchor: torch.Tensor, feat_true_sample: torch.Tensor):
        """Inputs:

        feat_anchor: [B_total, D]
        feat_true_sample: [B_total, N, D]
        """
        b_total, num_pos_samples, dim = feat_true_sample.shape
        device = feat_anchor.device

        # Dynamically align thresholds to input device
        if self.thresholds.device != device:
            self.thresholds = self.thresholds.to(device)

        # L2 normalize features to calculate cosine similarity safely
        feat_anchor = F.normalize(feat_anchor, p=2, dim=-1)
        feat_true_sample = F.normalize(feat_true_sample, p=2, dim=-1)

        # Flatten positive samples to 2D tensor: [B_total * N, D]
        feat_true_flat = feat_true_sample.reshape(-1, dim)

        # Calculate all-to-all similarity matrix: [B_total, B_total * N]
        sim_matrix = torch.matmul(feat_anchor, feat_true_flat.T)

        # Create a boolean mask for positive pairs using in-batch indices
        row_indices = torch.arange(b_total, device=device).unsqueeze(1)
        col_indices = torch.arange(b_total * num_pos_samples, device=device).unsqueeze(
            0
        )
        pos_mask = (col_indices // num_pos_samples) == row_indices

        # Extract positive and in-batch negative similarity values
        sim_pos = sim_matrix[pos_mask]
        sim_neg = sim_matrix[~pos_mask]

        # Track total counts across validation batches
        self.total_pos += sim_pos.numel()
        self.total_neg += sim_neg.numel()

        # Vectorized threshold matching without giant meshgrid allocation
        # Broadcasting logic fixed with device matching
        tp = (sim_pos.unsqueeze(-1) >= self.thresholds.view(1, -1)).sum(dim=0)
        tn = (sim_neg.unsqueeze(-1) < self.thresholds.view(1, -1)).sum(dim=0)

        # Accumulate metrics (keep on CPU to reduce VRAM consumption)
        self.tp_counts += tp.cpu()
        self.tn_counts += tn.cpu()

    def compute(self):
        """Calculates final curves and identifies the peak operational metrics."""
        if self.total_pos == 0 or self.total_neg == 0:
            raise ValueError("No positive or negative samples were found.")

        # Calculate metrics for all thresholds simultaneously
        tpr = self.tp_counts.float() / self.total_pos
        fpr = (self.total_neg - self.tn_counts.float()) / self.total_neg
        accuracy = (self.tp_counts + self.tn_counts).float() / (
            self.total_pos + self.total_neg
        )

        # Identify the peak accuracy and its corresponding threshold index
        best_idx = torch.argmax(accuracy)

        return Metric(
            accuracy[best_idx].item(),
            self.thresholds[best_idx].item(),
            tpr[best_idx].item(),
            fpr[best_idx].item(),
            self.thresholds.cpu().numpy().tolist(),
            accuracy.cpu().numpy().tolist(),
            tpr.cpu().numpy().tolist(),
            fpr.cpu().numpy().tolist(),
        )

    def print_metrics(self) -> tuple[Metric, str]:
        """
        Computes metrics and generates a styled ASCII evaluation report string.
        Returns a tuple containing the Metric dataclass instance and the report string.
        """
        metrics: Metric = self.compute()
        report_lines = []

        # Report Header
        report_lines.append("\n" + "=" * 66)
        report_lines.append(" METRIC LEARNING VERIFICATION REPORT ".center(66, "="))
        report_lines.append("=" * 66)

        # Summary Table Components
        top_line = "┌" + "─" * 24 + "┬" + "─" * 38 + "┐"
        mid_divider = "├" + "─" * 24 + "┼" + "─" * 38 + "┤"
        bottom_line = "└" + "─" * 24 + "┴" + "─" * 38 + "┘"

        report_lines.append(top_line)
        report_lines.append(
            f"│ {'Operational Metric':^22} │ {'Optimal Operational Value'::^22} │"
        )
        report_lines.append(mid_divider)

        # Highlight Peak Accuracy with standard ANSI bold green color formatting
        report_lines.append(
            f"│ {'Max Accuracy':<22} │ \033[1;32m{metrics.max_accuracy * 100::<22} %\033[0m │"
        )
        report_lines.append(
            f"│ {'Best Threshold (Tau)':<22} │ {metrics.best_threshold::<22}   │"
        )
        report_lines.append(
            f"│ {'TPR at Best Threshold':<22} │ {metrics.tpr_at_best * 100::<22} % │"
        )
        report_lines.append(
            f"│ {'FPR at Best Threshold':<22} │ {metrics.fpr_at_best * 100::<22} % │"
        )
        report_lines.append(bottom_line)

        # Curve Sampling Section (Sample 10 checkpoints across the full sweep)
        report_lines.append("\n" + "=" * 66)
        report_lines.append(" THRESHOLD SWEEP CURVE SAMPLE ".center(66, "="))
        report_lines.append("=" * 66)

        curve_top = (
            "┌" + "─" * 14 + "┬" + "─" * 14 + "┬" + "─" * 14 + "┬" + "─" * 16 + "┐"
        )
        curve_mid = (
            "├" + "─" * 14 + "┼" + "─" * 14 + "┼" + "─" * 14 + "┼" + "─" * 16 + "┤"
        )
        curve_bottom = (
            "└" + "─" * 14 + "┴" + "─" * 14 + "┴" + "─" * 14 + "┴" + "─" * 16 + "┘"
        )

        report_lines.append(curve_top)
        report_lines.append(
            f"│ {'Threshold':^12} │ {'Accuracy (%)':^12} │ {'TPR (%)':^12} │ {'FPR (%)':^14} │"
        )
        report_lines.append(curve_mid)

        total_steps = len(metrics.thresholds)
        # Sample up to 10 points uniformly across the curve to keep log highly scannable
        sample_stride = max(1, total_steps // 10)
        sample_indices = list(range(0, total_steps, sample_stride))

        # Ensure the final boundary threshold is always captured in report
        if (total_steps - 1) not in sample_indices:
            sample_indices.append(total_steps - 1)

        for idx in sample_indices:
            t_val = metrics.thresholds[idx]
            acc_val = metrics.accuracy_curve[idx] * 100
            tpr_val = metrics.tpr_curve[idx] * 100
            fpr_val = metrics.fpr_curve[idx] * 100

            # Dim boundaries where thresholds are 0.0 or 1.0 using dark gray ANSI styling
            if idx == 0 or idx == total_steps - 1:
                report_lines.append(
                    f"│ \033[90m{t_val:^12.2f}\033[0m │ "
                    f"\033[90m{acc_val:^12.2f}\033[0m │ "
                    f"\033[90m{tpr_val:^12.2f}\033[0m │ "
                    f"\033[90m{fpr_val:^14.2f}\033[0m │"
                )
            else:
                report_lines.append(
                    f"│ {t_val:^12.2f} │ "
                    f"{acc_val:^12.2f} │ "
                    f"{tpr_val:^12.2f} │ "
                    f"{fpr_val:^14.2f} │"
                )

        report_lines.append(curve_bottom)
        report_lines.append("=" * 66 + "\n")

        report_str = "\n".join(report_lines)
        return metrics, report_str
