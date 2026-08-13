"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

from metric.export import (
    TiffProcessor,
    QdrantPerformanceTester,
)

if __name__ == "__main__":
    model_path = "twinnet_inference.onnx"
    tiff_path = "experiment/test/output_tiled.tif"
    collection_name = "test_tiff"
    crop_size = 256
    out_dim = 2048

    processor = TiffProcessor(model_path=model_path, crop_size=crop_size, overlap=0.66)
    processor.run_ingestion(tiff_path, collection_name, out_dim)

    tester = QdrantPerformanceTester(
        model_path=model_path,
        tif_path=tiff_path,
        collection_name=collection_name,
        crop_size=crop_size,
    )
    # Global performance and recall test
    tester.test_performance_and_recall(500)
    # Real-time cropping + ONNX inference + Rank-5 search test
    tester.test_realtime_crop_and_rank(200, 3, "test_realtime_crop_and_rank")
