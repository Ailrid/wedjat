"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
from typing import Annotated, TypeAlias
from dataclasses import dataclass, field

Tensor6D: TypeAlias = Annotated[torch.Tensor, "Shape: (B, N, M, C, H, W)"]
Tensor5D: TypeAlias = Annotated[torch.Tensor, "Shape: (B, N, C, H, W)"]
Tensor4D: TypeAlias = Annotated[torch.Tensor, "Shape: (B, C, H, W)"]
Tensor3D: TypeAlias = Annotated[torch.Tensor, "Shape: (B, D, S)"]
Tensor2D: TypeAlias = Annotated[torch.Tensor, "Shape: (B, D)"]
Tensor1D: TypeAlias = Annotated[torch.Tensor, "Shape: (B,)"]


@dataclass
class Metric:
    max_accuracy: float = 0
    best_threshold: float = 0
    tpr_at_best: float = 0
    fpr_at_best: float = 0
    thresholds: list[float] = field(default_factory=list)
    accuracy_curve: list[float] = field(default_factory=list)
    tpr_curve: list[float] = field(default_factory=list)
    fpr_curve: list[float] = field(default_factory=list)
