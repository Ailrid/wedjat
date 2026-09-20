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
class RankMetric:
    rank1: float = 0.0
    rank2: float = 0.0
    rank3: float = 0.0
    rank4: float = 0.0
    rank5: float = 0.0


@dataclass
class Metric:
    metric: list[RankMetric] = field(default_factory=lambda: list())
