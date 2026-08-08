"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

import os
import matplotlib.pyplot as plt
import numpy as np
from metric.core import get_tiff_dataloader


def denormalize(tensor):
    # Convert normalized tensor back to unnormalized RGB image
    mean = np.array([0.485, 0.456, 0.406]).reshape(1, 1, 3)
    std = np.array([0.229, 0.224, 0.225]).reshape(1, 1, 3)

    img = tensor.permute(1, 2, 0).cpu().numpy()
    img = img * std + mean
    img = np.clip(img * 255.0, 0, 255).astype(np.uint8)
    return img


def run_tiff_dataloader_test(
    tiff_folder,
    batch_size=4,
    samples_per_yield=3,
    true_sample_number=3,
    input_size=256,
):
    if not os.path.exists(tiff_folder):
        print(f"Directory not found: {tiff_folder}")
        return

    print(f"Loading TIFF dataset from directory: {tiff_folder}")

    dataloader = get_tiff_dataloader(
        tiff_folder=tiff_folder,
        input_size=input_size,
        batch_size=batch_size,
        samples_per_yield=samples_per_yield,
        true_sample_number=true_sample_number,
        is_train=True,
        iter_times=10,
        num_workers=0,
    )

    for batch_idx, (anchors, positives) in enumerate(dataloader):
        print("\n" + "=" * 50)
        print(f"Batch {batch_idx + 1} successfully loaded. Tensor validation:")
        print(f"Anchors   shape [B, S, C, H, W]    : {anchors.shape}")
        print(f"Positives shape [B, S, N, C, H, W] : {positives.shape}")

        print(
            f"Anchors   value range              : [{anchors.min():.2f}, {anchors.max():.2f}]"
        )
        print(
            f"Positives value range              : [{positives.min():.2f}, {positives.max():.2f}]"
        )
        print("=" * 50)

        # Strict shape verification for 5D anchors and 6D positives
        expected_anchor_shape = (
            batch_size,
            samples_per_yield,
            3,
            input_size,
            input_size,
        )
        expected_pos_shape = (
            batch_size,
            samples_per_yield,
            true_sample_number,
            3,
            input_size,
            input_size,
        )

        assert (
            anchors.shape == expected_anchor_shape
        ), f"Expected anchor shape {expected_anchor_shape}, got {anchors.shape}"
        assert (
            positives.shape == expected_pos_shape
        ), f"Expected positive shape {expected_pos_shape}, got {positives.shape}"

        # Flatten total samples B * S into total rows for grid display
        total_samples = batch_size * samples_per_yield
        anchors_flat = anchors.reshape(total_samples, 3, input_size, input_size)
        positives_flat = positives.reshape(
            total_samples, true_sample_number, 3, input_size, input_size
        )

        total_cols = 1 + true_sample_number
        fig, axes = plt.subplots(
            total_samples, total_cols, figsize=(3.5 * total_cols, 3.5 * total_samples)
        )

        # Standardize 2D axes array structure
        if total_samples == 1:
            axes = np.expand_dims(axes, axis=0)

        for row_idx in range(total_samples):
            # Display anchor image
            anchor_img = denormalize(anchors_flat[row_idx])
            axes[row_idx, 0].imshow(anchor_img)
            axes[row_idx, 0].set_title(f"Sample {row_idx + 1}: Anchor")
            axes[row_idx, 0].axis("off")

            # Display positive samples
            for p_idx in range(true_sample_number):
                pos_img = denormalize(positives_flat[row_idx, p_idx])
                axes[row_idx, 1 + p_idx].imshow(pos_img)
                axes[row_idx, 1 + p_idx].set_title(f"Pos {p_idx + 1}")
                axes[row_idx, 1 + p_idx].axis("off")

        save_path = "iterable_dataloader_vis_result.png"
        plt.tight_layout()
        plt.savefig(save_path, dpi=150)
        print(f"\nVisualization output saved to: {os.path.abspath(save_path)}")

        break


if __name__ == "__main__":
    run_tiff_dataloader_test(
        tiff_folder="experiment/train",
        batch_size=4,
        samples_per_yield=2,
        true_sample_number=2,
        input_size=256,
    )
