"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import logging
from logging import getLogger
import re
import textwrap
import os
import json
from dataclasses import asdict
from .components import LightParameters, TrainingState
import matplotlib.pyplot as plt
import numpy as np
import seaborn as sns


RESET = "\x1b[0m"
BOLD = "\x1b[1m"
RED = "\x1b[31m"
GREEN = "\x1b[32m"
YELLOW = "\x1b[33m"
MAGENTA = "\x1b[35m"
CYAN = "\x1b[36m"
GRAY = "\x1b[90m"


ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*m")


class ViridConsoleFormatter(logging.Formatter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    def _get_prefix(self, record):
        if record.levelno == logging.INFO:
            return f"{GREEN}{BOLD}[INFO]{RESET} ", 7
        elif record.levelno == logging.WARNING:
            return f"{YELLOW}{BOLD}[WARN]{RESET} ", 7
        elif record.levelno == logging.ERROR:
            if "context" in getattr(record, "msg_type", ""):
                return f"{MAGENTA}{BOLD}✖ [ERR_CTX]{RESET} ", 11
            else:
                return f"{RED}{BOLD}✖ [ERROR]{RESET} ", 9
        return "", 0

    def format(self, record):
        raw_msg = record.getMessage()
        record.message = raw_msg

        # 消除首尾换行与代码缩进污染
        msg_str = textwrap.dedent(raw_msg.strip("\n"))

        prefix, _ = self._get_prefix(record)

        lines = msg_str.splitlines()
        if len(lines) > 1:
            indent = " " * 4
            formatted_lines = [f"{prefix}\n{lines[0]}"]
            for line in lines[1:]:
                formatted_lines.append(f"{indent}{line}")
            return "\n".join(formatted_lines)
        else:
            return f"{prefix}\n{msg_str}"


class ViridFileFormatter(logging.Formatter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    def _get_prefix(self, record):
        if record.levelno == logging.INFO:
            return "[INFO] ", 7
        elif record.levelno == logging.WARNING:
            return "[WARN] ", 7
        elif record.levelno == logging.ERROR:
            if "context" in getattr(record, "msg_type", ""):
                return "✖ [ERR_CTX] ", 11
            else:
                return "✖ [ERROR] ", 9
        return "", 0

    def format(self, record):
        raw_msg = record.getMessage()
        record.message = raw_msg

        # 消除多行日志的换行与环境缩进污染
        msg_str = textwrap.dedent(raw_msg.strip("\n"))

        # 把可能混进文件日志里的 ANSI 颜色代码通通清洗掉！
        msg_str = ANSI_ESCAPE.sub("", msg_str)

        time_str = self.formatTime(record, "%Y-%m-%d %H:%M:%S")
        time_prefix = f"[{time_str}] "
        level_prefix, _ = self._get_prefix(record)
        full_prefix = f"{time_prefix}{level_prefix}\n"

        lines = msg_str.splitlines()
        if len(lines) > 1:
            indent = " " * 4
            file_lines = [f"{full_prefix}{lines[0]}"]
            for line in lines[1:]:
                file_lines.append(f"{indent}{line}")
            return "\n".join(file_lines)
        else:
            return f"{full_prefix}{msg_str}"


def create_logger(log_file_path: str = "training.txt") -> logging.Logger:
    logger = getLogger("train_logger")
    logger.setLevel(logging.INFO)

    if not logger.handlers:
        console_handler = logging.StreamHandler()
        console_handler.setLevel(logging.INFO)
        console_handler.setFormatter(ViridConsoleFormatter())
        logger.addHandler(console_handler)

        file_handler = logging.FileHandler(
            log_file_path, mode="a", encoding="utf-8", delay=True
        )
        file_handler.setLevel(logging.INFO)
        file_handler.setFormatter(ViridFileFormatter())
        logger.addHandler(file_handler)

    return logger


def save_light_params(
    path: str,
    light_params: LightParameters,
):
    """
    将模型、数据集和环境配置序列化并保存到指定文件夹下的 params.json 文件中
    """
    os.makedirs(path, exist_ok=True)

    combined_dict = {
        "light_params": asdict(light_params),
    }

    file_path = os.path.join(path, "params.json")
    with open(file_path, "w", encoding="utf-8") as f:
        json.dump(combined_dict, f, indent=4, ensure_ascii=False)


def confirm_light_params(
    path: str,
    light_params: LightParameters,
):

    file_path = os.path.join(path, "params.json")
    if not os.path.exists(file_path):
        raise FileNotFoundError(
            f"The historical parameter configuration file was not found in the folder: {file_path}"
        )

    with open(file_path, "r", encoding="utf-8") as f:
        # 直接定位到内部的 light_params 字典
        saved_config = json.load(f).get("light_params", {})

    current_config = asdict(light_params)

    # 只有写入这个字典的字段，才会被严格比对
    required_checks = {
        "model_params": ["model_type", "out_dims", "dropout"],
    }

    mismatches = []
    matches = []

    for section_name, checked_keys in required_checks.items():
        saved_section = saved_config.get(section_name, {})
        current_section = current_config.get(section_name, {})

        for key in checked_keys:
            v_saved = saved_section.get(key)
            v_current = current_section.get(key)

            # Python 的 != 会自动递归比较嵌套的 voxel_params 字典
            if v_saved != v_current:
                mismatches.append(
                    f"  ➔ [{section_name}] -> property '{key}':\n"
                    f"      Historical saved values: {v_saved}\n"
                    f"      Current input value: {v_current}"
                )
            else:
                matches.append(
                    f"  ➔ [{section_name}] -> property '{key}':\n"
                    f"      Historical saved values: {v_saved}\n"
                    f"      Current input value: {v_current}\n"
                )

    if mismatches:
        error_title = f"\nParameter Mismatch! The current running configuration is inconsistent with the historically saved configuration."
        error_details = "\n".join(mismatches)
        raise ValueError(
            f"{error_title}\n{error_details}\nPlease check your configuration file or clean up the experiment folder."
        )

    return "".join(matches)


# def plot_training_state(state: TrainingState):
#     """Generates a 3x2 visualization dashboard for In-Batch metric learning

#     using exact TrainingState dataclass structure.
#     """
#     try:
#         plt.style.use("seaborn-v0_8-whitegrid")
#     except Exception:
#         plt.style.use("default")

#     fig = plt.figure(figsize=(16, 18), dpi=100)
#     gs = fig.add_gridspec(3, 2, height_ratios=[1, 1, 1])

#     cur_m = state.current_metrics
#     best_m = state.best_metrics
#     history_list = state.metrics_history.metric

#     # Global Summary Title
#     title_str = (
#         f"In-Batch Metric Learning Dashboard [Epoch {state.current_epoch}]\n"
#         f"Current: MRR={cur_m.in_batch_mrr:.4f} | Recall@1={cur_m.in_batch_recall_1 * 100:.2f}%  ||  "
#         f"Best Hist: MRR={best_m.in_batch_mrr:.4f} | Recall@1={best_m.in_batch_recall_1 * 100:.2f}%"
#     )
#     fig.suptitle(title_str, fontsize=16, fontweight="bold", y=0.98, va="top")

#     # ROW 1: Loss and Ranking Metrics History

#     # Subplot: Training Loss Curve
#     ax_loss = fig.add_subplot(gs[0, 0])
#     losses = state.train_loss
#     if len(losses) > 0:
#         epochs_loss = range(1, len(losses) + 1)
#         ax_loss.plot(
#             epochs_loss,
#             losses,
#             color="#E64B35",
#             alpha=0.6,
#             linewidth=1.5,
#             label="Batch Loss",
#         )
#         if len(losses) >= 5:
#             window = max(2, len(losses) // 10)
#             smooth_loss = np.convolve(losses, np.ones(window) / window, mode="valid")
#             ax_loss.plot(
#                 range(window, len(losses) + 1),
#                 smooth_loss,
#                 color="#7E2F23",
#                 linewidth=2.5,
#                 label=f"Moving Avg (w={window})",
#             )
#         ax_loss.set_title("Training Loss History", fontsize=12, fontweight="bold")
#         ax_loss.set_xlabel("Epochs")
#         ax_loss.set_ylabel("Loss")
#         ax_loss.legend(loc="upper right")
#     else:
#         ax_loss.text(
#             0.5,
#             0.5,
#             "No Loss Data Available",
#             ha="center",
#             va="center",
#             fontsize=12,
#         )

#     # Subplot: In-Batch Ranking Metrics History
#     ax_rank = fig.add_subplot(gs[0, 1])
#     if len(history_list) > 0:
#         epochs_hist = range(1, len(history_list) + 1)
#         r1_list = [m.in_batch_recall_1 * 100 for m in history_list]
#         r5_list = [m.in_batch_recall_5 * 100 for m in history_list]
#         mrr_list = [m.in_batch_mrr for m in history_list]

#         ax_rank.plot(
#             epochs_hist,
#             r1_list,
#             color="#E64B35",
#             marker="o",
#             markersize=4,
#             linewidth=2,
#             label="Recall@1 (%)",
#         )
#         ax_rank.plot(
#             epochs_hist,
#             r5_list,
#             color="#4DBBD5",
#             marker="s",
#             markersize=4,
#             linewidth=2,
#             label="Recall@5 (%)",
#         )

#         ax_rank.set_title(
#             "In-Batch Ranking Metrics History", fontsize=12, fontweight="bold"
#         )
#         ax_rank.set_xlabel("Epochs")
#         ax_rank.set_ylabel("Recall Score (%)")
#         ax_rank.set_ylim(-2, 102)

#         # Secondary Y-axis for MRR score
#         ax_mrr = ax_rank.twinx()
#         ax_mrr.plot(
#             epochs_hist,
#             mrr_list,
#             color="#00A087",
#             linestyle="--",
#             linewidth=2.5,
#             label="MRR",
#         )
#         ax_mrr.set_ylabel("MRR Score", color="#00A087")
#         ax_mrr.set_ylim(-0.02, 1.02)
#         ax_mrr.grid(False)

#         lines_1, labels_1 = ax_rank.get_legend_handles_labels()
#         lines_2, labels_2 = ax_mrr.get_legend_handles_labels()
#         ax_rank.legend(lines_1 + lines_2, labels_1 + labels_2, loc="lower right")
#     else:
#         ax_rank.text(
#             0.5,
#             0.5,
#             "No Metric History Available",
#             ha="center",
#             va="center",
#             fontsize=12,
#         )

#     # ROW 2: Separability Index and Similarity Distribution Dynamics

#     # Subplot: Feature Separability Index (d') and Margin History
#     ax_sep = fig.add_subplot(gs[1, 0])
#     if len(history_list) > 0:
#         d_prime_list = [m.d_prime for m in history_list]
#         margin_list = [m.margin for m in history_list]

#         ax_sep.plot(
#             epochs_hist, # type: ignore
#             d_prime_list,
#             color="#3C5488",
#             marker="^",
#             markersize=4,
#             linewidth=2,
#             label="Separability Index (d')",
#         )
#         ax_sep.set_title(
#             "Feature Separability & Margin History",
#             fontsize=12,
#             fontweight="bold",
#         )
#         ax_sep.set_xlabel("Epochs")
#         ax_sep.set_ylabel("d' Index", color="#3C5488")

#         ax_margin = ax_sep.twinx()
#         ax_margin.plot(
#             epochs_hist,  # type: ignore
#             margin_list,
#             color="#F39B7F",
#             linestyle="-.",
#             linewidth=2,
#             label="Similarity Margin",
#         )
#         ax_margin.set_ylabel("Margin (Mean Pos - Mean Neg)", color="#F39B7F")
#         ax_margin.grid(False)

#         lines_1, labels_1 = ax_sep.get_legend_handles_labels()
#         lines_2, labels_2 = ax_margin.get_legend_handles_labels()
#         ax_sep.legend(lines_1 + lines_2, labels_1 + labels_2, loc="upper left")
#     else:
#         ax_sep.text(
#             0.5,
#             0.5,
#             "No Separability Data Available",
#             ha="center",
#             va="center",
#             fontsize=12,
#         )

#     # Subplot: Positive and Negative Cosine Similarity Dynamics
#     ax_sim = fig.add_subplot(gs[1, 1])
#     if len(history_list) > 0:
#         pos_mean = np.array([m.mean_pos_sim for m in history_list])
#         pos_std = np.array([m.std_pos_sim for m in history_list])
#         neg_mean = np.array([m.mean_neg_sim for m in history_list])
#         neg_std = np.array([m.std_neg_sim for m in history_list])

#         ax_sim.plot(
#             epochs_hist,  # type: ignore
#             pos_mean,
#             color="#00A087",
#             linewidth=2,
#             label="Positive Sim Mean",
#         )
#         ax_sim.fill_between(
#             epochs_hist,  # type: ignore
#             pos_mean - pos_std,
#             pos_mean + pos_std,
#             color="#00A087",
#             alpha=0.15,
#         )

#         ax_sim.plot(
#             epochs_hist,  # type: ignore
#             neg_mean,
#             color="#E64B35",
#             linewidth=2,
#             label="Negative Sim Mean",
#         )
#         ax_sim.fill_between(
#             epochs_hist,  # type: ignore
#             neg_mean - neg_std,
#             neg_mean + neg_std,
#             color="#E64B35",
#             alpha=0.15,
#         )

#         ax_sim.set_title(
#             "Cosine Similarity Dynamics (Mean ± Std)",
#             fontsize=12,
#             fontweight="bold",
#         )
#         ax_sim.set_xlabel("Epochs")
#         ax_sim.set_ylabel("Cosine Similarity")
#         ax_sim.set_ylim(-1.05, 1.05)
#         ax_sim.legend(loc="center right")
#     else:
#         ax_sim.text(
#             0.5,
#             0.5,
#             "No Similarity Distribution Data",
#             ha="center",
#             va="center",
#             fontsize=12,
#         )

#     # ROW 3: Current vs Best Model Comparison

#     # Subplot: Bar Chart Comparing Current vs Best Metrics
#     ax_bar = fig.add_subplot(gs[2, 0])
#     metrics_names = ["Recall@1 (%)", "Recall@5 (%)", "MRR (*100)", "d' Index"]

#     cur_values = [
#         cur_m.in_batch_recall_1 * 100,
#         cur_m.in_batch_recall_5 * 100,
#         cur_m.in_batch_mrr * 100,
#         cur_m.d_prime,
#     ]
#     best_values = [
#         best_m.in_batch_recall_1 * 100,
#         best_m.in_batch_recall_5 * 100,
#         best_m.in_batch_mrr * 100,
#         best_m.d_prime,
#     ]

#     x = np.arange(len(metrics_names))
#     width = 0.35

#     ax_bar.bar(
#         x - width / 2,
#         cur_values,
#         width,
#         label="Current Epoch",
#         color="#4DBBD5",
#         alpha=0.85,
#     )
#     ax_bar.bar(
#         x + width / 2,
#         best_values,
#         width,
#         label="Best Epoch",
#         color="#00A087",
#         alpha=0.85,
#     )

#     ax_bar.set_title(
#         "Current vs Best Metric Performance Comparison",
#         fontsize=12,
#         fontweight="bold",
#     )
#     ax_bar.set_xticks(x)
#     ax_bar.set_xticklabels(metrics_names)
#     ax_bar.legend(loc="upper left")

#     for i in range(len(metrics_names)):
#         ax_bar.text(
#             x[i] - width / 2,
#             cur_values[i] + 1,
#             f"{cur_values[i]:.1f}",
#             ha="center",
#             va="bottom",
#             fontsize=9,
#         )
#         ax_bar.text(
#             x[i] + width / 2,
#             best_values[i] + 1,
#             f"{best_values[i]:.1f}",
#             ha="center",
#             va="bottom",
#             fontsize=9,
#         )

#     # Subplot: Statistical Error Bar Comparison
#     ax_stat = fig.add_subplot(gs[2, 1])

#     categories = [
#         "Current Pos Sim",
#         "Current Neg Sim",
#         "Best Pos Sim",
#         "Best Neg Sim",
#     ]
#     means = [
#         cur_m.mean_pos_sim,
#         cur_m.mean_neg_sim,
#         best_m.mean_pos_sim,
#         best_m.mean_neg_sim,
#     ]
#     stds = [
#         cur_m.std_pos_sim,
#         cur_m.std_neg_sim,
#         best_m.std_pos_sim,
#         best_m.std_neg_sim,
#     ]

#     colors = ["#00A087", "#E64B35", "#3C5488", "#F39B7F"]
#     x_pos = np.arange(len(categories))

#     ax_stat.errorbar(
#         x_pos,
#         means,
#         yerr=stds,
#         fmt="o",
#         ecolor="black",
#         color="darkblue",
#         elinewidth=2,
#         capsize=6,
#         markersize=8,
#     )
#     for i in range(len(categories)):
#         ax_stat.scatter(x_pos[i], means[i], color=colors[i], s=100, zorder=5)
#         ax_stat.text(
#             x_pos[i],
#             means[i] + stds[i] + 0.05,
#             f"{means[i]:.2f}±{stds[i]:.2f}",
#             ha="center",
#             va="bottom",
#             fontsize=9,
#         )

#     ax_stat.set_title(
#         "Similarity Distribution Parameters (Mean ± Std)",
#         fontsize=12,
#         fontweight="bold",
#     )
#     ax_stat.set_xticks(x_pos)
#     ax_stat.set_xticklabels(categories, rotation=15)
#     ax_stat.set_ylabel("Cosine Similarity Value")
#     ax_stat.set_ylim(-1.05, 1.2)

#     plt.tight_layout()
#     fig.subplots_adjust(top=0.92)

#     os.makedirs(state.log_folder, exist_ok=True)
#     save_path = os.path.join(state.log_folder, "training_state.png")
#     plt.savefig(save_path, bbox_inches="tight")
#     plt.close()
