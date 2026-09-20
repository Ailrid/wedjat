"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import os
import matplotlib.pyplot as plt
import numpy as np
import torch
from metric.core import get_cross_view_dataloader


def denormalize(tensor: torch.Tensor) -> np.ndarray:
    """Convert normalized Image Tensor [C, H, W] back to unnormalized RGB image [H, W, C]."""
    mean = np.array([0.485, 0.456, 0.406]).reshape(1, 1, 3)
    std = np.array([0.229, 0.224, 0.225]).reshape(1, 1, 3)

    img = tensor.permute(1, 2, 0).cpu().numpy()
    img = img * std + mean
    img = np.clip(img * 255.0, 0, 255).astype(np.uint8)
    return img


def run_cross_view_dataloader_test(
    dataset_dir: str,
    batch_size: int = 4,
    drone_samples_per_location: int = 2,
    input_size: int = 256,
):
    if not os.path.exists(dataset_dir):
        print(f"Directory not found: {dataset_dir}")
        return

    print(f"Loading Cross-View dataset from directory: {dataset_dir}")

    dataloader = get_cross_view_dataloader(
        root_dir=dataset_dir,
        input_size=(input_size, input_size),
        batch_size=batch_size,
        drone_samples_per_location=drone_samples_per_location,
        is_train=True,
        num_workers=0,
    )

    for batch_idx, (sat_batch, drone_batch, ids) in enumerate(dataloader):
        print("\n" + "=" * 50)
        print(f"Batch {batch_idx + 1} successfully loaded. Tensor validation:")
        print(f"Satellite batch shape [B, C, H, W]        : {sat_batch.shape}")
        if drone_samples_per_location > 1:
            print(f"Drone batch shape     [B, K, C, H, W]     : {drone_batch.shape}")
        else:
            print(f"Drone batch shape     [B, C, H, W]        : {drone_batch.shape}")
        print(f"Location IDs                              : {ids}")

        print(
            f"Satellite value range                     : [{sat_batch.min():.2f}, {sat_batch.max():.2f}]"
        )
        print(
            f"Drone value range                         : [{drone_batch.min():.2f}, {drone_batch.max():.2f}]"
        )
        print("=" * 50)

        # Strict shape verification assertions
        expected_sat_shape = (batch_size, 3, input_size, input_size)
        assert (
            sat_batch.shape == expected_sat_shape
        ), f"Expected satellite shape {expected_sat_shape}, got {sat_batch.shape}"

        if drone_samples_per_location > 1:
            expected_drone_shape = (
                batch_size,
                drone_samples_per_location,
                3,
                input_size,
                input_size,
            )
            assert (
                drone_batch.shape == expected_drone_shape
            ), f"Expected drone shape {expected_drone_shape}, got {drone_batch.shape}"
        else:
            expected_drone_shape = (batch_size, 3, input_size, input_size)
            assert (
                drone_batch.shape == expected_drone_shape
            ), f"Expected drone shape {expected_drone_shape}, got {drone_batch.shape}"

        # Plotting & Visualization setup
        total_cols = 1 + drone_samples_per_location
        fig, axes = plt.subplots(
            batch_size, total_cols, figsize=(3.5 * total_cols, 3.5 * batch_size)
        )

        # Standardize 2D axes array structure if batch_size == 1
        if batch_size == 1:
            axes = np.expand_dims(axes, axis=0)

        for row_idx in range(batch_size):
            loc_id = ids[row_idx]

            # Display Satellite Image (Column 0)
            sat_img = denormalize(sat_batch[row_idx])
            axes[row_idx, 0].imshow(sat_img)
            axes[row_idx, 0].set_title(f"ID: {loc_id} | Satellite")
            axes[row_idx, 0].axis("off")

            # Display Drone Image(s) (Columns 1 .. N)
            if drone_samples_per_location > 1:
                for k_idx in range(drone_samples_per_location):
                    drone_img = denormalize(drone_batch[row_idx, k_idx])
                    axes[row_idx, 1 + k_idx].imshow(drone_img)
                    axes[row_idx, 1 + k_idx].set_title(f"Drone {k_idx + 1}")
                    axes[row_idx, 1 + k_idx].axis("off")
            else:
                drone_img = denormalize(drone_batch[row_idx])
                axes[row_idx, 1].imshow(drone_img)
                axes[row_idx, 1].set_title("Drone 1")
                axes[row_idx, 1].axis("off")

        save_path = "cross_view_dataloader_vis_result.png"
        plt.tight_layout()
        plt.savefig(save_path, dpi=150)
        print(f"\nVisualization output saved to: {os.path.abspath(save_path)}")

        break


if __name__ == "__main__":
    run_cross_view_dataloader_test(
        dataset_dir="dataset/cross_view/train",
        batch_size=4,
        drone_samples_per_location=2,
        input_size=224,
    )
