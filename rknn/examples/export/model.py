"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-rknn
"""

import os
import sys
from rknn.api import RKNN


def init_and_config_rknn(
    target_platform,
    mean_values,
    std_values,
    quantized_dtype="asymmetric_quantized-8",
    quantized_method="channel",
    quantized_algorithm="normal",
):
    """Initialize RKNN object and configure preprocessing parameters."""
    print("--> Initializing and configuring RKNN...")
    rknn = RKNN(verbose=False)
    rknn.config(
        target_platform=target_platform,
        mean_values=mean_values,
        std_values=std_values,
        quantized_dtype=quantized_dtype,
        quantized_method=quantized_method,
        quantized_algorithm=quantized_algorithm,
    )
    return rknn


def load_onnx_model(rknn, path):
    """Load the source ONNX model into RKNN memory."""
    print(f"--> Loading ONNX model from: {path}")
    if not os.path.exists(path):
        print(f"Error: ONNX model file not found at {path}")
        rknn.release()
        sys.exit(-1)

    ret = rknn.load_onnx(model=path)
    if ret != 0:
        print("Error: Load ONNX model failed! Please check model operators.")
        rknn.release()
        sys.exit(ret)


def build_rknn_model(rknn, do_quantization=True, dataset_path=None):
    """Build RKNN model with full quantization or FP16 mode."""
    print(f"--> Building RKNN model (do_quantization={do_quantization})...")
    if do_quantization:
        if not dataset_path or not os.path.exists(dataset_path):
            print(f"Error: Dataset file not found at {dataset_path} for quantization.")
            rknn.release()
            sys.exit(-1)
        ret = rknn.build(do_quantization=True, dataset=dataset_path)
    else:
        ret = rknn.build(do_quantization=False)

    if ret != 0:
        print("Error: Build RKNN model failed!")
        rknn.release()
        sys.exit(ret)
    print("Success: RKNN model built successfully.")


def run_accuracy_analysis_on_folder(rknn, folder_path, output_dir):
    """Scan folder and perform layer-by-layer accuracy analysis using all found images."""
    print(f"--> Scanning folder for test images: {folder_path}")
    if not os.path.isdir(folder_path):
        print(f"Error: Provided path is not a valid directory: {folder_path}")
        rknn.release()
        sys.exit(-1)

    valid_extensions = (
        ".jpg",
        ".jpeg",
        ".png",
        ".bmp",
        ".JPG",
        ".JPEG",
        ".PNG",
        ".BMP",
    )

    image_list = [
        os.path.join(folder_path, f)
        for f in os.listdir(folder_path)
        if f.endswith(valid_extensions)
    ]

    if not image_list:
        print(f"Error: No valid images found in folder: {folder_path}")
        rknn.release()
        sys.exit(-1)

    print(f"--> Found {len(image_list)} images. Starting batch accuracy analysis...")

    ret = rknn.accuracy_analysis(inputs=image_list, output_dir=output_dir, target=None)
    if ret != 0:
        print("Error: Accuracy analysis failed!")
        rknn.release()
        sys.exit(ret)
    print(
        f"Success: Accuracy analysis reports saved to '{output_dir}/error_analysis.txt'"
    )


def export_rknn_model(rknn, path):
    """Export the compiled and verified RKNN model to disk."""
    print(f"--> Exporting quantized RKNN model to: {path}")
    ret = rknn.export_rknn(path)
    if ret != 0:
        print("Error: Export RKNN model failed!")
        rknn.release()
        sys.exit(ret)
    print("Success: Final RKNN model generated successfully.")


def main(
    do_quantization,
    quantized_dtype,
    model_path,
    output_path,
    dataset_path,
    image_folder_path,
    target_platform,
    analysis_output_dir,
    mean_values,
    std_values,
):
    """Execute main flow for RKNN model building."""
    rknn = init_and_config_rknn(
        target_platform=target_platform,
        mean_values=mean_values,
        std_values=std_values,
        quantized_dtype=quantized_dtype,
    )
    try:
        load_onnx_model(rknn, model_path)
        build_rknn_model(
            rknn, do_quantization=do_quantization, dataset_path=dataset_path
        )

        # Optional: Run accuracy analysis after building
        if os.path.exists(image_folder_path):
            run_accuracy_analysis_on_folder(
                rknn, image_folder_path, analysis_output_dir
            )

        export_rknn_model(rknn, output_path)
    except Exception as e:
        print(f"An unexpected error occurred: {e}")
    finally:
        print("--> Releasing RKNN resources...")
        rknn.release()


if __name__ == "__main__":
    # Settings:
    # 1. Full UINT8/INT8 Quantization: do_quantization = True
    # 2. Pure FP16 Mode: do_quantization = False
    do_quantization = True

    # Quantization data type options:
    # - "asymmetric_quantized-8" (standard asymmetric int8/uint8)
    # - "w8a8"
    quantized_dtype = "w8a8"

    model_path = "assets/super.onnx"
    output_path = "assets/model.rknn"
    dataset_path = "./test_images/dataset.txt"
    image_folder_path = "./test_images"
    target_platform = "rk1828"
    analysis_output_dir = "./snapshot"

    mean_values = [[123.675, 116.28, 103.53]]
    std_values = [[58.395, 57.12, 57.375]]

    main(
        do_quantization=do_quantization,
        quantized_dtype=quantized_dtype,
        model_path=model_path,
        output_path=output_path,
        dataset_path=dataset_path,
        image_folder_path=image_folder_path,
        target_platform=target_platform,
        analysis_output_dir=analysis_output_dir,
        mean_values=mean_values,
        std_values=std_values,
    )
