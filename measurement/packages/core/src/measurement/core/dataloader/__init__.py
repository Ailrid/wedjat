"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

import math
import os
import random
import cv2
import numpy as np
import rasterio
from rasterio.windows import Window
import torch
from torch.utils.data import DataLoader, IterableDataset
from torchvision.transforms import v2


class TiffLoader(IterableDataset):

    def __init__(
        self,
        tiff_folder: str,
        input_size: int = 256,
        iter_times: int = 100,
        samples_per_yield: int = 2,
        true_sample_number: int = 2,
        max_shift: int = 64,
        max_scale: float = 1.1,
        max_rotation: float = 30.0,
        max_pitch: float = 10.0,
        transform=None,
        is_train: bool = True,
    ):
        super().__init__()
        self.tiff_folder = tiff_folder
        self.input_size = input_size
        self.iter_times = iter_times
        self.true_sample_number = true_sample_number
        self.samples_per_yield = samples_per_yield

        self.max_shift = max_shift
        self.max_scale = max_scale
        self.max_rotation = max_rotation
        self.max_pitch = max_pitch
        self.transform = transform
        self.is_train = is_train

        self.tiff_list = [
            os.path.join(self.tiff_folder, f)
            for f in os.listdir(self.tiff_folder)
            if f.endswith((".tif", ".tiff"))
        ]

        # Calculate enlarged crop size to prevent black borders during perspective warping
        large_size = int(math.ceil(self.input_size * 1.8 * self.max_scale))
        self.large_crop_size = large_size if large_size % 2 == 0 else large_size + 1
        self.margin = self.large_crop_size // 2

        # Ensure all TIFF files are tiled once during initialization
        for tiff_path in self.tiff_list:
            self._ensure_tiled(tiff_path, tile_size=256)

    def _get_3d_perspective_matrix(self, w, h, pitch, roll, yaw, fov=60):
        """Calculate 3D homography transformation matrix from camera pose."""
        f = (w / 2.0) / math.tan(math.radians(fov / 2.0))
        K = np.array([[f, 0, w / 2.0], [0, f, h / 2.0], [0, 0, 1.0]], dtype=np.float32)
        K_inv = np.linalg.inv(K)

        rx, ry, rz = math.radians(pitch), math.radians(roll), math.radians(yaw)

        Rx = np.array(
            [
                [1, 0, 0],
                [0, math.cos(rx), -math.sin(rx)],
                [0, math.sin(rx), math.cos(rx)],
            ]
        )
        Ry = np.array(
            [
                [math.cos(ry), 0, math.sin(ry)],
                [0, 1, 0],
                [-math.sin(ry), 0, math.cos(ry)],
            ]
        )
        Rz = np.array(
            [
                [math.cos(rz), -math.sin(rz), 0],
                [math.sin(rz), math.cos(rz), 0],
                [0, 0, 1],
            ]
        )

        R = Rz @ Ry @ Rx
        return (K @ R @ K_inv).astype(np.float32)

    def _apply_transform(self, img_large, is_query=True):
        """Apply perspective projection, rotation, and scaling."""
        h_l, w_l = img_large.shape[:2]

        if is_query:
            pitch = np.random.uniform(0, self.max_pitch)
            roll = np.random.uniform(-10, 10)
        else:
            pitch = 0.0
            roll = 0.0

        yaw = np.random.uniform(-self.max_rotation, self.max_rotation)
        scale = np.random.uniform(1.0 / self.max_scale, self.max_scale)

        H = self._get_3d_perspective_matrix(w_l, h_l, pitch, roll, yaw, fov=60)

        if scale != 1.0:
            S = np.array(
                [
                    [scale, 0, (1 - scale) * w_l / 2.0],
                    [0, scale, (1 - scale) * h_l / 2.0],
                    [0, 0, 1.0],
                ],
                dtype=np.float32,
            )
            H = H @ S

        warped = cv2.warpPerspective(
            img_large,
            H,
            (w_l, h_l),
            flags=cv2.INTER_LINEAR,
            borderMode=cv2.BORDER_REFLECT,
        )

        start_x = (w_l - self.input_size) // 2
        start_y = (h_l - self.input_size) // 2
        img_crop = warped[
            start_y : start_y + self.input_size, start_x : start_x + self.input_size
        ]

        return img_crop

    def _ensure_tiled(self, tiff_path: str, tile_size: int = 256):
        """Convert image to tiled structure if required."""
        with rasterio.open(tiff_path) as src:
            if src.is_tiled:
                return
            print(f"Converting {tiff_path} to tiled TIFF...")
            profile = src.profile.copy()
            data = src.read()

        profile.update(
            tiled=True,
            blockxsize=tile_size,
            blockysize=tile_size,
        )

        temp_path = tiff_path + ".tmp"
        with rasterio.open(temp_path, "w", **profile) as dst:
            dst.write(data)

        os.replace(temp_path, tiff_path)

    def _sample_single_pair(
        self, src: rasterio.DatasetReader, width: int, height: int
    ) -> tuple[torch.Tensor, torch.Tensor]:
        """Extract a single anchor tensor and its corresponding positive tensors."""
        cx_anchor = random.randint(self.margin, width - self.margin)
        cy_anchor = random.randint(self.margin, height - self.margin)

        # Read anchor window
        anchor_window = Window(
            cx_anchor - self.margin,  # type: ignore
            cy_anchor - self.margin,
            self.large_crop_size,
            self.large_crop_size,
        )
        anchor_large = src.read(window=anchor_window)
        if anchor_large.shape[0] > 3:
            anchor_large = anchor_large[:3, :, :]
        anchor_large = np.transpose(anchor_large, (1, 2, 0))

        if self.is_train:
            anchor_patch = self._apply_transform(anchor_large, is_query=False)
        else:
            s_x = (self.large_crop_size - self.input_size) // 2
            anchor_patch = anchor_large[
                s_x : s_x + self.input_size, s_x : s_x + self.input_size
            ]

        anchor_tensor = (
            torch.from_numpy(anchor_patch.copy()).permute(2, 0, 1).contiguous()
        )
        if self.transform:
            anchor_tensor = self.transform(anchor_tensor)

        # Generate positive samples with pitch, yaw, scale, and center shift
        pos_tensors = []
        for _ in range(self.true_sample_number):
            shift_x = random.randint(-self.max_shift, self.max_shift)
            shift_y = random.randint(-self.max_shift, self.max_shift)

            cx_pos = min(max(self.margin, cx_anchor + shift_x), width - self.margin)
            cy_pos = min(max(self.margin, cy_anchor + shift_y), height - self.margin)

            pos_window = Window(
                cx_pos - self.margin,  # type: ignore
                cy_pos - self.margin,
                self.large_crop_size,
                self.large_crop_size,
            )
            pos_large = src.read(window=pos_window)
            if pos_large.shape[0] > 3:
                pos_large = pos_large[:3, :, :]
            pos_large = np.transpose(pos_large, (1, 2, 0))

            if self.is_train:
                pos_patch = self._apply_transform(pos_large, is_query=True)
            else:
                s_x = (self.large_crop_size - self.input_size) // 2
                pos_patch = pos_large[
                    s_x : s_x + self.input_size, s_x : s_x + self.input_size
                ]

            p_tensor = torch.from_numpy(pos_patch.copy()).permute(2, 0, 1).contiguous()
            if self.transform:
                p_tensor = self.transform(p_tensor)

            pos_tensors.append(p_tensor)

        return anchor_tensor, torch.stack(pos_tensors)

    def __len__(self):
        return len(self.tiff_list) * self.iter_times

    def __iter__(self):
        tasks = [(tiff, i) for tiff in self.tiff_list for i in range(self.iter_times)]

        for _ in range(len(self.tiff_list)):
            idx = random.randint(0, len(self.tiff_list) - 1)
            current_tiff = self.tiff_list[idx]
            
            with rasterio.open(current_tiff) as src:

                for _ in range(self.iter_times):
                    width, height = src.width, src.height

                    if width < 2 * self.margin or height < 2 * self.margin:
                        continue

                    anchors, positives = [], []
                    for _ in range(self.samples_per_yield):
                        anchor_tensor, pos_tensor = self._sample_single_pair(
                            src, width, height
                        )
                        anchors.append(anchor_tensor)
                        positives.append(pos_tensor)

                    yield torch.stack(anchors), torch.stack(positives)


def get_tiff_dataloader(
    tiff_folder,
    input_size=256,
    batch_size=2,
    true_sample_number=2,
    samples_per_yield=2,
    is_train=True,
    iter_times=100,
    num_workers=4,
):
    if is_train:
        tensor_transforms = v2.Compose(
            [
                v2.ColorJitter(brightness=0.3, contrast=0.3, saturation=0.2, hue=0.05),
                v2.ToDtype(torch.float32, scale=True),
                v2.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
                v2.RandomErasing(p=0.3, scale=(0.02, 0.15)),
            ]
        )
    else:
        tensor_transforms = v2.Compose(
            [
                v2.ToDtype(torch.float32, scale=True),
                v2.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
            ]
        )

    dataset = TiffLoader(
        tiff_folder=tiff_folder,
        input_size=input_size,
        iter_times=iter_times,
        true_sample_number=true_sample_number,
        samples_per_yield=samples_per_yield,
        max_shift=input_size // 4,
        max_scale=1.1,
        max_rotation=30,
        max_pitch=10,
        transform=tensor_transforms,
        is_train=is_train,
    )

    # Note: If samples_per_yield > 1 and dataset already outputs a batch, set batch_size=None or adjust batch_size accordingly
    return DataLoader(
        dataset,
        batch_size=batch_size,
        num_workers=num_workers,
        pin_memory=True,
    )
