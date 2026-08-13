"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.nn import init
from torchvision import models


class GeM(nn.Module):
    """
    Generalized Mean Pooling Layer
    Computes generalized mean pooling with numerical stability for FP16 training.
    """

    def __init__(self, p: float = 3.0, eps: float = 1e-6):
        super(GeM, self).__init__()
        self.p = nn.Parameter(torch.ones(1) * p)
        self.eps = eps

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        orig_dtype = x.dtype
        x_f32 = x.float().clamp(min=self.eps)
        pooled = F.adaptive_avg_pool2d(x_f32.pow(self.p), (1, 1)).pow(1.0 / self.p)
        return pooled.to(orig_dtype)


def weights_init_kaiming(m):
    """
    Kaiming initialization without accessing private data attributes.
    """
    classname = m.__class__.__name__
    with torch.no_grad():
        if "Conv" in classname:
            init.kaiming_normal_(m.weight, a=0, mode="fan_in")
            if m.bias is not None:
                init.constant_(m.bias, 0.0)
        elif "Linear" in classname:
            init.kaiming_normal_(m.weight, a=0, mode="fan_out")
            if m.bias is not None:
                init.constant_(m.bias, 0.0)
        elif "BatchNorm" in classname:
            if m.weight is not None:
                init.normal_(m.weight, 1.0, 0.02)
            if m.bias is not None:
                init.constant_(m.bias, 0.0)


class ResNet50(nn.Module):

    def __init__(
        self,
        feat_dim: int = 512,
        stride: int = 1,
    ):
        super(ResNet50, self).__init__()

        model_ft = models.resnet50(weights=models.ResNet50_Weights.IMAGENET1K_V1)

        if stride == 1:
            model_ft.layer4[0].conv2.stride = (1, 1)  # type: ignore
            model_ft.layer4[0].downsample[0].stride = (1, 1)  # type: ignore
            for block in model_ft.layer4:
                block.conv2.dilation = (2, 2)  # type: ignore
                block.conv2.padding = (2, 2)  # type: ignore

        self.model = model_ft
        self.avgpool = GeM()

        # Linear projection layer for setting custom feature dimension
        self.fc = nn.Linear(2048, feat_dim, bias=False)
        self.fc.apply(weights_init_kaiming)

        # Batch Normalization neck updated to fit the new feature dimension
        self.bn = nn.BatchNorm1d(feat_dim)
        self.bn.bias.requires_grad_(False)
        self.bn.apply(weights_init_kaiming)

    def forward(self, x):
        x = self.model.conv1(x)
        x = self.model.bn1(x)
        x = self.model.relu(x)
        x = self.model.maxpool(x)

        x = self.model.layer1(x)
        x = self.model.layer2(x)
        x = self.model.layer3(x)
        x = self.model.layer4(x)

        x = self.avgpool(x)
        x = x.view(x.size(0), -1)

        # Feature dimension reduction and BNNeck processing
        feat = self.fc(x)
        feat = self.bn(feat)

        # L2 normalization for distance-based loss computation
        f_norm = F.normalize(feat, p=2, dim=1)
        return f_norm
