"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-rknn
"""

import os
import sys
from rknn.api import RKNN


def convert_onnx_to_rknn_fp16(
    onnx_path,
    rknn_path,
    target_platform="rk3588",
    quantized_dtype="w8a8",
    quantized_method="channel",
    quantized_algorithm="normal",
):
    """Convert LightGlue ONNX model to pure FP16 RKNN model."""
    rknn = RKNN(verbose=False)

    print(f"--> Configuring RKNN for FP16 ({target_platform})...")
    # For non-image inputs, mean_values and std_values should be None
    rknn.config(
        target_platform=target_platform,
        mean_values=None,
        std_values=None,
        quantized_dtype=quantized_dtype,
        quantized_method=quantized_method,
        quantized_algorithm=quantized_algorithm,
    )

    print(f"--> Loading ONNX model from {onnx_path}...")
    if not os.path.exists(onnx_path):
        print(f"Error: ONNX model file not found at {onnx_path}")
        sys.exit(-1)

    ret = rknn.load_onnx(model=onnx_path)
    if ret != 0:
        print("Error: Load ONNX model failed!")
        rknn.release()
        sys.exit(ret)

    print("--> Building RKNN model with FP16 (do_quantization=False)...")
    # Setting do_quantization=False forces the NPU to use FP16 precision
    ret = rknn.build(do_quantization=False)
    if ret != 0:
        print("Error: Build FP16 RKNN model failed!")
        rknn.release()
        sys.exit(ret)

    print(f"--> Exporting RKNN model to {rknn_path}...")
    ret = rknn.export_rknn(rknn_path)
    if ret != 0:
        print("Error: Export RKNN model failed!")
        rknn.release()
        sys.exit(ret)

    print("Success: FP16 RKNN model created successfully.")
    rknn.release()


if __name__ == "__main__":
    onnx_file = "assets/lightglue.onnx"
    rknn_file = "assets/lightglue.rknn"
    platform = "rk1828"  # 'rk3588', 'rk1828'

    convert_onnx_to_rknn_fp16(
        onnx_path=onnx_file,
        rknn_path=rknn_file,
        target_platform=platform,
    )

