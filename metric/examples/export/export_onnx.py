"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch

from metric.core import Shell, ResNet50V2, ViT, ConvNeXtTiny, Classify

checkpoint_path = "checkpoints/2026-09-20-15-29-22"

out_dim = 512
class_num = 861

shell = Shell(ViT(out_dim), Classify(out_dim, class_num))
shell.load_checkpoint(checkpoint_path)

model = shell.model

dummy_input = torch.randn(1, 3, 224, 224)
model.eval()
# Export to ONNX
torch.onnx.export(
    model,
    dummy_input,  # type: ignore
    "assets/vit.onnx",
    opset_version=14,
    input_names=["input"],
    output_names=["output"],
)
