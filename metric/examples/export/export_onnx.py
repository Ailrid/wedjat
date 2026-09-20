"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch

from metric.core import Shell, ResNet50V2, ViT, ConvNeXtTiny

checkpoint_path = "assets/vit.pth"

out_dim = 512
# shell = Shell(ResNet50V2(out_dim))
shell = Shell(ViT(out_dim))
# shell = Shell(ConvNeXtTiny(out_dim))

# shell.load_checkpoint(checkpoint_path)
model = ViT(out_dim)
# model.load_state_dict(torch.load(checkpoint_path)["model_state_dict"])

dummy_input = torch.randn(1, 3, 224, 224)
model.eval()
# Export to ONNX
torch.onnx.export(
    model,
    dummy_input,  # type: ignore
    "assets/vit.pth",
    opset_version=14,
    input_names=["input"],
    output_names=["output"],
)
