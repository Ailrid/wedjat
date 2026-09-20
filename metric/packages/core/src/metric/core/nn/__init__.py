"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""


from .interface import *
from .loss import *
from .vit import *
from .shell import *
from .resnet import *
from .convnext import *


class Classify(nn.Module):

    def __init__(
        self,
        in_features: int,
        num_classes: int,
        dropout: float = 0.0,
    ):
        super(Classify, self).__init__()
        if dropout > 0.0:
            self.head = nn.Sequential(
                nn.Dropout(p=dropout),
                nn.Linear(in_features, num_classes),
            )
        else:
            self.head = nn.Linear(in_features, num_classes)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.head(x)
