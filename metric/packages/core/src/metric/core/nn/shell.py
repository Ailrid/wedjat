"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from dataclasses import asdict
import json
import os

import torch
from .interface import Network
from ..structs import Metric, Tensor4D, Tensor5D, Tensor6D

class Shell(Network):
    def __init__(self, model: torch.nn.Module):
        super(Shell, self).__init__()
        self.model = model

    def forward(
            self, anchor: torch.Tensor, true_samples: torch.Tensor
        ) -> tuple[torch.Tensor, torch.Tensor]:
            """Input Shapes:

            anchor:        [B, S, C, H, W]       (S = samples_per_yield)
            true_samples:  [B, S, N, C, H, W]    (N = true_sample_number)

            Output Shapes:
                feat_anchor:       [B * S, D]         (D = embedding dimension)
                feat_true_sample:  [B * S, N, D]
            """
            batch_size, samples_per_yield, channels, height, width = anchor.shape
            num_pos_samples = true_samples.shape[2]
            total_batch = batch_size * samples_per_yield

            # Flatten anchor images to 4D tensor: [B * S, C, H, W]
            anchor_flat = anchor.reshape(total_batch, channels, height, width)
            feat_anchor = self.model(anchor_flat)  # Output shape: [B * S, D]

            # Flatten positive samples to 4D tensor: [B * S * N, C, H, W]
            true_samples_flat = true_samples.reshape(
                total_batch * num_pos_samples, channels, height, width
            )
            feat_true_flat = self.model(
                true_samples_flat
            )  # Output shape: [B * S * N, D]

            # Restore positive features to tensor structure: [B * S, N, D]
            feat_true_sample = feat_true_flat.reshape(
                total_batch, num_pos_samples, -1
            )

            return feat_anchor, feat_true_sample

    @torch.no_grad()
    def refer(self, anchor: Tensor4D, true_samples: Tensor6D):
        return self.forward(anchor, true_samples)

    def save_checkpoint(self, path: str, metric: Metric):

        os.makedirs(path, exist_ok=True)

        components = {
            "model": self.model,
        }

        for name, sub_module in components.items():
            file_path = os.path.join(path, f"{name}.pth")
            state_dict = sub_module.state_dict()
            torch.save(state_dict, file_path)

            # 核验文件是否真正成功写入且大小正常
            if not (os.path.exists(file_path) and os.path.getsize(file_path) > 0):
                raise IOError(
                    f"The weight file of submodule [{name}] failed to save or the file is empty!"
                )

        with open(os.path.join(path, "metric.json"), "w") as f:
            f.write(json.dumps(asdict(metric), indent=4, ensure_ascii=False))

    def load_checkpoint(self, path: str):

        components = {
            "model": self.model,
        }

        # 基础物理文件完整性核验
        for name in components.keys():
            file_path = os.path.join(path, f"{name}.pth")
            if not os.path.exists(file_path):
                raise FileNotFoundError(
                    f"\nWeight loading intercepted! Missing key sub component weight file:\n"
                    f"   ➔ Expected file path: {file_path}\n"
                    f"   Please check if the experimental folder or model architecture definition matches."
                )

        # Keys 结构与 Shape 尺寸
        for name, sub_module in components.items():
            file_path = os.path.join(path, f"{name}.pth")

            # 先加载到 CPU
            loaded_state_dict = torch.load(file_path, map_location="cpu")
            current_state_dict = sub_module.state_dict()

            loaded_keys = set(loaded_state_dict.keys())
            current_keys = set(current_state_dict.keys())

            # 计算差异键
            missing_keys = current_keys - loaded_keys
            unexpected_keys = loaded_keys - current_keys
            shape_mismatches = []

            # 核验交集 Key 的张量 Shape 是否对齐
            for key in current_keys & loaded_keys:
                if current_state_dict[key].shape != loaded_state_dict[key].shape:
                    shape_mismatches.append(
                        f"      ➔ Attribute '{key}':\n"
                        f"          Runtime model dimension: {list(current_state_dict[key].shape)}\n"
                        f"          File save weight dimension: {list(loaded_state_dict[key].shape)}"
                    )

            # 如果该组件存在任何不一致，立刻抛出详细的崩溃报告，绝不带病运行
            if missing_keys or unexpected_keys or shape_mismatches:
                error_title = (
                    f"\nWeight Dimension Mismatch inside sub-component [{name}]!"
                )
                error_details = []

                if missing_keys:
                    error_details.append(
                        f"   The key that exists in the current model but is missing in the weight file is:\n      {list(missing_keys)}"
                    )
                if unexpected_keys:
                    error_details.append(
                        f"   The weight file contains redundant keys in the current model:\n      {list(unexpected_keys)}"
                    )
                if shape_mismatches:
                    error_details.append(
                        f"   Geometric Dimension Conflict (modified d_model/head/num_classes):\n"
                        + "\n".join(shape_mismatches)
                    )

                raise ValueError(
                    f"{error_title}\n"
                    + "\n\n".join(error_details)
                    + f"\n\nPlease clean up conflicting historical experiment folders or correct the network hyperparameter configuration in 'train. py'."
                )

            sub_module.load_state_dict(loaded_state_dict)
