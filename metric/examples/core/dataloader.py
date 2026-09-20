"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import time
from torch.utils.data import DataLoader
from metric.core.dataloader import TiffLoader, get_cross_view_dataloader


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


def benchmark_cross_view_dataloader():
    # Dataset configuration matching your environment
    dataset_dir = "dataset/cross_view/train"  # Path containing 'drone' and 'satellite' subdirectories
    input_size = (224, 224)
    batch_size = 4
    drone_samples_per_location = 2  # Number of drone images per location
    num_workers = 4
    is_train = True

    print("Initializing CrossView DataLoader...")
    dataloader = get_cross_view_dataloader(
        root_dir=dataset_dir,
        input_size=input_size,
        batch_size=batch_size,
        drone_samples_per_location=drone_samples_per_location,
        is_train=is_train,
        num_workers=num_workers,
    )

    print(f"Starting benchmark with num_workers={num_workers}...")
    start_time = time.time()
    batch_count = 0
    total_images_processed = 0

    # Calculate total images in a single sample pair (1 satellite + N drone images)
    images_per_sample = 1 + drone_samples_per_location

    iterator = iter(dataloader)
    try:
        # Warm-up step: PyTorch worker processes start-up time
        sat_batch, drone_batch, ids = next(iterator)
        batch_count += 1

        print("--- Shape Verification ---")
        print("Satellite batch shape :", sat_batch.shape)  # [B, C, H, W]
        print(
            "Drone batch shape     :", drone_batch.shape
        )  # [B, K, C, H, W] or [B, C, H, W]
        print("Location IDs sample   :", ids[: min(4, len(ids))])

        current_batch_size = sat_batch.size(0)
        total_images_processed += current_batch_size * images_per_sample
        print("Warm-up completed. Iterating through dataset...\n")

    except StopIteration:
        print(
            f"No valid dataset found at '{dataset_dir}'. Please verify directory structure."
        )
        return

    # Benchmark dataset iteration speed
    for sat_batch, drone_batch, ids in iterator:
        batch_count += 1
        current_batch_size = sat_batch.size(0)
        total_images_processed += current_batch_size * images_per_sample

        if batch_count % 10 == 0:
            elapsed = time.time() - start_time
            print(
                f"Processed {batch_count} batches ({total_images_processed} images) in {elapsed:.2f}s..."
            )

    end_time = time.time()
    total_time = end_time - start_time

    batches_per_second = batch_count / total_time
    images_per_second = total_images_processed / total_time

    print("\n================ BENCHMARK RESULTS ================")
    print(f"Total Time Elapsed  : {total_time:.2f} seconds")
    print(f"Total Batches Read  : {batch_count}")
    print(f"Total Images Loaded : {total_images_processed} (Satellite + Drone)")
    print(f"Throughput (Batch)  : {batches_per_second:.2f} batches/sec")
    print(f"Throughput (Image)  : {images_per_second:.2f} images/sec")
    print("===================================================")


if __name__ == "__main__":
    # benchmark_dataloader()
    benchmark_cross_view_dataloader()
