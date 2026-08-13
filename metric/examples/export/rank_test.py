"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

from metric.export import (
    test_img_folder_rank,
)

if __name__ == "__main__":
    model_path = "twinnet_inference.onnx"
    csv_path = "experiment/rgb/metadata.csv"
    tiff_path = "experiment/test/output_tiled.tif"
    folder_path = "experiment/rgb"
    collection_name = "test_tiff"

    test_img_folder_rank(
        csv_path=csv_path,
        folder_path=folder_path,
        tif_path=tiff_path,
        model_path=model_path,
        collection_name=collection_name,
        input_size=256,
        scale=1.0,
        visualize_top_k=10,
        output_dir="png_search_visualizations",
    )
