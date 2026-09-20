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
        # self.avgpool = GeM()
        self.avgpool = nn.AdaptiveAvgPool2d((1, 1))

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


class BottleneckV2(nn.Module):
    """ResNet50 V2 Bottleneck Block (Pre-activation)."""

    expansion = 4

    def __init__(
        self,
        in_planes: int,
        planes: int,
        stride: int = 1,
        downsample: nn.Module = None, # type: ignore
    ):
        super().__init__()
        self.bn1 = nn.BatchNorm2d(in_planes)
        self.conv1 = nn.Conv2d(in_planes, planes, kernel_size=1, bias=False)

        self.bn2 = nn.BatchNorm2d(planes)
        self.conv2 = nn.Conv2d(
            planes, planes, kernel_size=3, stride=stride, padding=1, bias=False
        )

        self.bn3 = nn.BatchNorm2d(planes)
        self.conv3 = nn.Conv2d(
            planes, planes * self.expansion, kernel_size=1, bias=False
        )

        self.relu = nn.ReLU(inplace=True)
        self.downsample = downsample

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        # Pre-activation
        out = self.relu(self.bn1(x))

        # Shortcut path
        shortcut = self.downsample(out) if self.downsample is not None else x

        out = self.conv1(out)
        out = self.conv2(self.relu(self.bn2(out)))
        out = self.conv3(self.relu(self.bn3(out)))

        return out + shortcut


class ResNet50V2(nn.Module):
    """Standard ResNet50 V2 Architecture."""

    def __init__(self, num_classes: int = 1000):
        super().__init__()
        self.in_planes = 64

        # Stage 1: Stem
        self.conv1 = nn.Conv2d(3, 64, kernel_size=7, stride=2, padding=3, bias=False)
        self.maxpool = nn.MaxPool2d(kernel_size=3, stride=2, padding=1)

        # Stages 2-5: Residual Blocks
        self.layer1 = self._make_layer(BottleneckV2, 64, 3, stride=1)
        self.layer2 = self._make_layer(BottleneckV2, 128, 4, stride=2)
        self.layer3 = self._make_layer(BottleneckV2, 256, 6, stride=2)
        self.layer4 = self._make_layer(BottleneckV2, 512, 3, stride=2)

        # Final Pre-activation & Classifier Head
        self.bn_final = nn.BatchNorm2d(512 * BottleneckV2.expansion)
        self.relu = nn.ReLU(inplace=True)
        self.avgpool = nn.AdaptiveAvgPool2d((1, 1))
        self.fc = nn.Linear(512 * BottleneckV2.expansion, num_classes)

    def _make_layer(
        self, block: type[BottleneckV2], planes: int, blocks: int, stride: int
    ) -> nn.Sequential:
        downsample = None
        out_planes = planes * block.expansion

        if stride != 1 or self.in_planes != out_planes:
            downsample = nn.Conv2d(
                self.in_planes, out_planes, kernel_size=1, stride=stride, bias=False
            )

        layers = []
        layers.append(block(self.in_planes, planes, stride, downsample)) # type: ignore
        self.in_planes = out_planes

        for _ in range(1, blocks):
            layers.append(block(self.in_planes, planes))

        return nn.Sequential(*layers)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = self.conv1(x)
        x = self.maxpool(x)

        x = self.layer1(x)
        x = self.layer2(x)
        x = self.layer3(x)
        x = self.layer4(x)

        x = self.relu(self.bn_final(x))
        x = self.avgpool(x)
        x = torch.flatten(x, 1)
        x = self.fc(x)

        return x
