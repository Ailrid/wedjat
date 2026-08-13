"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: rknn
"""

import os
import random
import numpy as np
import rasterio
from rasterio.windows import Window
from PIL import Image


def random_crop_tiff(
    tif_path, output_dir, dataset_txt_path, crop_size=(256, 256), num_crops=10
):
    """Randomly crop patches from a TIFF file and automatically generate RKNN dataset.txt."""
    if not os.path.exists(tif_path):
        print(f"Error: TIFF file not found at {tif_path}")
        return

    os.makedirs(output_dir, exist_ok=True)
    crop_h, crop_w = crop_size

    with rasterio.open(tif_path) as src:
        img_w = src.width
        img_h = src.height

        if img_w < crop_w or img_h < crop_h:
            print(
                f"Error: Crop size {crop_size} is larger than image dimensions ({img_h}, {img_w})"
            )
            return

        print(f"--> Source image dimensions: {img_w}x{img_h}, Channels: {src.count}")
        print(f"--> Generating {num_crops} random crops...")

        success_count = 0
        saved_paths = []  # Store paths for dataset.txt

        while success_count < num_crops:
            xmin = random.randint(0, img_w - crop_w)
            ymin = random.randint(0, img_h - crop_h)

            window = Window(xmin, ymin, crop_w, crop_h)  # type: ignore
            data = src.read(window=window, out_shape=(src.count, crop_h, crop_w))

            if src.count >= 3:
                data = data[:3, :, :]
                data = np.transpose(data, (1, 2, 0))
            elif src.count == 1:
                data = data[0, :, :]
            else:
                data = np.transpose(data, (1, 2, 0))

            if data.dtype != np.uint8:
                if data.max() > data.min():
                    data = (
                        (data - data.min()) / (data.max() - data.min()) * 255
                    ).astype(np.uint8)
                else:
                    data = data.astype(np.uint8)

            try:
                img = Image.fromarray(data)
                output_filename = f"crop_{success_count:04d}_{ymin}_{xmin}.jpg"
                output_path = os.path.join(output_dir, output_filename)
                img.save(output_path, "JPEG", quality=95)

                # Record the saved image path
                saved_paths.append(output_filename)
                success_count += 1
            except Exception as e:
                print(f"Warning: Failed to save crop at ({xmin}, ymin). Error: {e}")
                continue

        print(f"Success: Saved {success_count} crops to '{output_dir}'")

        # Automatically generate dataset.txt for RKNN calibration
        print(f"--> Generating RKNN dataset file at: {dataset_txt_path}")
        try:
            with open(dataset_txt_path, "w") as f:
                for path in saved_paths:
                    f.write(path + "\n")
            print(
                f"Success: {dataset_txt_path} generated successfully with {len(saved_paths)} entries."
            )
        except Exception as e:
            print(f"Error: Failed to write {dataset_txt_path}. Error: {e}")



if __name__ == "__main__":

    tif_path = "assets/test.tif"
    output_dir = "./test_images"
    dataset_txt_path = "./test_images/dataset.txt"
    crop_size = (256, 256)  # (Height, Width)
    num_crops = 250

    random_crop_tiff(
        tif_path=tif_path,
        output_dir=output_dir,
        dataset_txt_path=dataset_txt_path,
        crop_size=crop_size,
        num_crops=num_crops,
    )
