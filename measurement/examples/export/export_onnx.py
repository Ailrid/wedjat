"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

import torch

from measurement.core import Shell, ResNet50, ViT, ConvNeXtTiny

checkpoint_path = "checkpoints/2026-07-03-23-02-13"

out_dim = 512
shell = Shell(ResNet50(out_dim))
# shell = Shell(ViT(out_dim))
# shell = Shell(ConvNeXtTiny(out_dim))

shell.load_checkpoint(checkpoint_path)
model = shell.model

dummy_input = torch.randn(1, 3, 256, 256)
model.eval()
# Export to ONNX
torch.onnx.export(
    model,
    dummy_input,  # type: ignore
    "model.onnx",
    opset_version=12,
    input_names=["input"],
    output_names=["output"],
)
