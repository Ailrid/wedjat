"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""
import torch.nn as nn
import torch.nn.functional as F
import timm


class ViT(nn.Module):

    def __init__(self, embed_dim=512, img_size=224, drop_path_rate=0.1):
        super(ViT, self).__init__()

        self.backbone = timm.create_model(
            "vit_base_patch16_224",
            pretrained=True,
            num_classes=0,
            img_size=img_size,
            drop_path_rate=drop_path_rate,
        )

        in_features = self.backbone.num_features  # 768

        self.bottleneck = nn.Linear(in_features, embed_dim, bias=False)  # type: ignore
        self.bn = nn.BatchNorm1d(embed_dim)

        nn.init.kaiming_normal_(self.bottleneck.weight, mode="fan_out")
        nn.init.normal_(self.bn.weight, 1.0, 0.02)
        nn.init.constant_(self.bn.bias, 0.0)

    def forward(self, x):
        feat = self.backbone(x)
        feat = self.bottleneck(feat)
        feat = self.bn(feat)
        return F.normalize(feat, p=2, dim=1)
