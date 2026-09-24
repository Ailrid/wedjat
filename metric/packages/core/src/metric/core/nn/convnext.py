"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
from torchvision import models
from .resnet import GeM, weights_init_kaiming

class ConvNeXtTiny(nn.Module):

    def __init__(self, embed_dim: int = 512, pretrained: bool = True):
        super(ConvNeXtTiny, self).__init__()

        weights = models.ConvNeXt_Tiny_Weights.IMAGENET1K_V1 if pretrained else None
        convnext = models.convnext_tiny(weights=weights)

        self.features = convnext.features
        in_features = 768  # ConvNeXt-Tiny 的默认输出通道数

        self.gem = GeM()

        self.projection = nn.Linear(in_features, embed_dim, bias=False)
 
        self.bn = nn.BatchNorm1d(embed_dim)
        self.bn.apply(weights_init_kaiming)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # [B, 3, H, W] -> [B, 768, H/32, W/32]
        x = self.features(x)

        # [B, 768, H/32, W/32] -> [B, 768, 1, 1]
        x = self.gem(x)
        x = torch.flatten(x, 1)  # [B, 768]

        x = self.projection(x)  # [B, 512]
        feat = self.bn(x)

        f_norm = F.normalize(feat, p=2, dim=1)
        return f_norm
