"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import time
from torch.utils.data import DataLoader
from metric.core.dataloader import TiffLoader


def benchmark_dataloader():
    # Define the target TIFF path
    tiff_folder = "experiment/train"

    # Dataset configuration parameters matching your implementation
    input_size = 256
    iter_times = 50  # Number of areas to sample per TIFF file
    true_sample_number = 3  # Positive samples per area
    samples_per_yield = 3  # Negative samples per area
    batch_size = 4  # Batch size for training simulation
    num_workers = 4  # Number of parallel CPU workers

    print("Initializing TiffLoader with tuple output...")
    dataset = TiffLoader(
        tiff_folder=tiff_folder,
        input_size=input_size,
        iter_times=iter_times,
        true_sample_number=true_sample_number,
        samples_per_yield=samples_per_yield,
    )

    # Initialize PyTorch DataLoader with multi-processing
    dataloader = DataLoader(dataset, batch_size=batch_size, num_workers=num_workers)

    print(f"Starting benchmark with num_workers={num_workers}...")
    start_time = time.time()
    batch_count = 0
    total_images_processed = 0

    # Warm up: PyTorch multi-processing initialization takes a moment on the first step
    iterator = iter(dataloader)
    try:
        # Unpack the batch as a tuple instead of a dictionary
        anchors, positives = next(iterator)
        batch_count += 1
        print("actual size:", anchors.shape)
        print("positives size:", positives.shape)

        # Calculate individual image crops per yield: 1 anchor + N positives + M negatives
        images_per_yield = (1 + true_sample_number) * samples_per_yield
        total_images_processed += anchors.size(0) * images_per_yield
        print("Warm-up completed. Iterating through the remaining data...")
    except StopIteration:
        print(
            "No data found. Please verify that the TIFF file exists in the directory."
        )
        return

    # Measure the actual data loading speed across the remaining stream
    for anchors, positives in iterator:
        batch_count += 1
        current_batch_size = anchors.size(0)
        total_images_processed += current_batch_size * images_per_yield

        # Print progress metrics every 10 batches
        if batch_count % 10 == 0:
            elapsed = time.time() - start_time
            print(f"Processed {batch_count} batches in {elapsed:.2f} seconds...")

    end_time = time.time()
    total_time = end_time - start_time

    # Calculate performance and throughput metrics
    batches_per_second = batch_count / total_time
    images_per_second = total_images_processed / total_time

    print("\n================ BENCHMARK RESULTS ================")
    print(f"Total Time Elapsed  : {total_time:.2f} seconds")
    print(f"Total Batches Read  : {batch_count}")
    print(f"Total Images Loaded : {total_images_processed} (including all crops)")
    print(f"Throughput (Batch)  : {batches_per_second:.2f} batches/sec")
    print(f"Throughput (Image)  : {images_per_second:.2f} crops/sec")
    print("===================================================")


if __name__ == "__main__":
    benchmark_dataloader()
