import logging
from logging import getLogger
import re
import textwrap
import os
import json
from dataclasses import asdict
from .components import LightParameters, TrainingState
from typing import Optional
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


def plot_training_state(state: TrainingState):
    """
    Generates a 3x2 mixed training status dashboard for self-supervised
    metric learning and binary verification metrics.
    """
    try:
        plt.style.use("seaborn-v0_8-whitegrid")
    except:
        plt.style.use("default")

    # Create a 3x2 grid layout matching your original aspect standard
    fig = plt.figure(figsize=(16, 18), dpi=100)
    gs = fig.add_gridspec(3, 2, height_ratios=[1, 1, 1])

    # Top global text block summarizing historical progress
    title_str = (
        f"Metric Learning Training Dashboard [Epoch {state.current_epoch}]\n"
        f"Current: Max Acc={state.current_metrics.max_accuracy * 100:.2f} % | "
        f"Best Threshold={state.current_metrics.best_threshold:.4f}\n"
        f"Best Hist: Max Acc={state.best_metrics.max_accuracy * 100:.2f} % | "
        f"Best Threshold={state.best_metrics.best_threshold:.4f}"
    )
    fig.suptitle(title_str, fontsize=16, fontweight="bold", y=0.98, va="top")

    # -------------------------------------------------------------------------
    # ROW 1: HISTORICAL PROGRESS TRACKING
    # -------------------------------------------------------------------------

    # 1. [Top-Left]: Training Loss Curve
    ax_loss = fig.add_subplot(gs[0, 0])
    losses = state.train_loss
    if isinstance(losses, list) and len(losses) > 0:
        ax_loss.plot(
            range(1, len(losses) + 1),
            losses,
            color="#E64B35",
            alpha=0.8,
            linewidth=1.5,
            label="Batch Loss",
        )
        if len(losses) > 10:
            window = max(2, len(losses) // 10)
            smooth_loss = np.convolve(losses, np.ones(window) / window, mode="valid")
            ax_loss.plot(
                range(window, len(losses) + 1),
                smooth_loss,
                color="#7E2F23",
                linewidth=2.5,
                label=f"Moving Avg (w={window})",
            )
        ax_loss.set_title("Training Loss History", fontsize=12, fontweight="bold")
        ax_loss.set_xlabel("Epochs")
        ax_loss.set_ylabel("Loss")
        ax_loss.legend()
    else:
        ax_loss.text(
            0.5, 0.5, "No Loss Data Available", ha="center", va="center", fontsize=12
        )

    # 2. [Top-Right]: Peak Binary Accuracy Curve Over Epochs
    ax_acc_hist = fig.add_subplot(gs[0, 1])
    train_acc = state.train_max_accuracy
    test_acc = state.test_max_accuracy
    if isinstance(train_acc, list) and len(train_acc) > 0:
        epochs_range = range(1, len(train_acc) + 1)
        ax_acc_hist.plot(
            epochs_range,
            train_acc,
            color="#00A087",
            marker="o",
            markersize=4,
            linewidth=2,
            label="Train Max Acc",
        )
        if isinstance(test_acc, list) and len(test_acc) > 0:
            ax_acc_hist.plot(
                range(1, len(test_acc) + 1),
                test_acc,
                color="#3C5488",
                marker="s",
                markersize=4,
                linewidth=2,
                label="Test Max Acc",
            )
        ax_acc_hist.set_title(
            "Max Verification Accuracy History", fontsize=12, fontweight="bold"
        )
        ax_acc_hist.set_xlabel("Epochs")
        ax_acc_hist.set_ylabel("Accuracy Score")
        ax_acc_hist.set_ylim(-0.02, 1.02)
        ax_acc_hist.legend(loc="lower right")
    else:
        ax_acc_hist.text(
            0.5, 0.5, "No Accuracy History Data", ha="center", va="center", fontsize=12
        )

    # -------------------------------------------------------------------------
    # ROW 2: THRESHOLD SWEEP DYNAMICS (ACCURACY, TPR, FPR VS THRESHOLD)
    # -------------------------------------------------------------------------

    # 3. [Middle-Left]: Current Epoch Threshold Sweep
    ax_cur_sweep = fig.add_subplot(gs[1, 0])
    cur_m = state.current_metrics
    if isinstance(cur_m.thresholds, list) and len(cur_m.thresholds) > 0:
        ax_cur_sweep.plot(
            cur_m.thresholds,
            cur_m.accuracy_curve,
            color="#4DBBD5",
            linewidth=2.5,
            label="Accuracy",
        )
        ax_cur_sweep.plot(
            cur_m.thresholds,
            cur_m.tpr_curve,
            color="#00A087",
            linestyle="--",
            label="TPR (Recall)",
        )
        ax_cur_sweep.plot(
            cur_m.thresholds,
            cur_m.fpr_curve,
            color="#E64B35",
            linestyle=":",
            label="FPR",
        )
        ax_cur_sweep.axvline(
            x=cur_m.best_threshold,
            color="#7E2F23",
            linestyle="-.",
            label=f"Best Threshold ({cur_m.best_threshold:.3f})",
        )
        ax_cur_sweep.set_title(
            f"Epoch {state.current_epoch} Threshold Sweep Curve",
            fontsize=12,
            fontweight="bold",
        )
        ax_cur_sweep.set_xlabel("Cosine Similarity Threshold")
        ax_cur_sweep.set_ylabel("Metrics Score")
        ax_cur_sweep.set_ylim(-0.02, 1.02)
        ax_cur_sweep.legend(loc="lower left")
    else:
        ax_cur_sweep.text(
            0.5, 0.5, "No Current Sweep Data", ha="center", va="center", fontsize=12
        )

    # 4. [Middle-Right]: Best Historical Threshold Sweep
    ax_best_sweep = fig.add_subplot(gs[1, 1])
    best_m = state.best_metrics
    if isinstance(best_m.thresholds, list) and len(best_m.thresholds) > 0:
        ax_best_sweep.plot(
            best_m.thresholds,
            best_m.accuracy_curve,
            color="#91D1C2",
            linewidth=2.5,
            label="Accuracy",
        )
        ax_best_sweep.plot(
            best_m.thresholds,
            best_m.tpr_curve,
            color="#00A087",
            linestyle="--",
            label="TPR (Recall)",
        )
        ax_best_sweep.plot(
            best_m.thresholds,
            best_m.fpr_curve,
            color="#E64B35",
            linestyle=":",
            label="FPR",
        )
        ax_best_sweep.axvline(
            x=best_m.best_threshold,
            color="#7E2F23",
            linestyle="-.",
            label=f"Best Threshold ({best_m.best_threshold:.3f})",
        )
        ax_best_sweep.set_title(
            "Best History Threshold Sweep Curve", fontsize=12, fontweight="bold"
        )
        ax_best_sweep.set_xlabel("Cosine Similarity Threshold")
        ax_best_sweep.set_ylabel("Metrics Score")
        ax_best_sweep.set_ylim(-0.02, 1.02)
        ax_best_sweep.legend(loc="lower left")
    else:
        ax_best_sweep.text(
            0.5, 0.5, "No Best Sweep Data", ha="center", va="center", fontsize=12
        )

    # -------------------------------------------------------------------------
    # ROW 3: ROC CURVES SPACE (FPR VS TPR VISUALIZATION)
    # -------------------------------------------------------------------------

    # 5. [Bottom-Left]: Current Epoch ROC Curve
    ax_cur_roc = fig.add_subplot(gs[2, 0])
    if isinstance(cur_m.fpr_curve, list) and len(cur_m.fpr_curve) > 0:
        ax_cur_roc.plot(
            cur_m.fpr_curve,
            cur_m.tpr_curve,
            color="#3C5488",
            linewidth=2.5,
            label="ROC Curve",
        )
        ax_cur_roc.plot(
            [0, 1], [0, 1], color="gray", linestyle="--", alpha=0.5, label="Baseline"
        )
        ax_cur_roc.scatter(
            cur_m.fpr_at_best,
            cur_m.tpr_at_best,
            color="#E64B35",
            s=60,
            zorder=5,
            label=f"Optimal Operational Point",
        )
        ax_cur_roc.set_title(
            f"Epoch {state.current_epoch} ROC Curve", fontsize=12, fontweight="bold"
        )
        ax_cur_roc.set_xlabel("False Positive Rate (FPR)")
        ax_cur_roc.set_ylabel("True Positive Rate (TPR)")
        ax_cur_roc.set_xlim(-0.02, 1.02)
        ax_cur_roc.set_ylim(-0.02, 1.02)
        ax_cur_roc.legend(loc="lower right")
    else:
        ax_cur_roc.text(
            0.5,
            0.5,
            "No Current ROC Data Available",
            ha="center",
            va="center",
            fontsize=12,
        )

    # 6. [Bottom-Right]: Best Historical ROC Curve
    ax_best_roc = fig.add_subplot(gs[2, 1])
    if isinstance(best_m.fpr_curve, list) and len(best_m.fpr_curve) > 0:
        ax_best_roc.plot(
            best_m.fpr_curve,
            best_m.tpr_curve,
            color="#A1A9D0",
            linewidth=2.5,
            label="ROC Curve",
        )
        ax_best_roc.plot(
            [0, 1], [0, 1], color="gray", linestyle="--", alpha=0.5, label="Baseline"
        )
        ax_best_roc.scatter(
            best_m.fpr_at_best,
            best_m.tpr_at_best,
            color="#E64B35",
            s=60,
            zorder=5,
            label=f"Optimal Operational Point",
        )
        ax_best_roc.set_title("Best History ROC Curve", fontsize=12, fontweight="bold")
        ax_best_roc.set_xlabel("False Positive Rate (FPR)")
        ax_best_roc.set_ylabel("True Positive Rate (TPR)")
        ax_best_roc.set_xlim(-0.02, 1.02)
        ax_best_roc.set_ylim(-0.02, 1.02)
        ax_best_roc.legend(loc="lower right")
    else:
        ax_best_roc.text(
            0.5,
            0.5,
            "No Best ROC Data Available",
            ha="center",
            va="center",
            fontsize=12,
        )

    # Standard alignment adjustments
    plt.tight_layout()
    fig.subplots_adjust(top=0.92)

    save_path = os.path.join(state.log_folder, "training_state.png")
    plt.savefig(save_path, bbox_inches="tight")
    plt.close()
