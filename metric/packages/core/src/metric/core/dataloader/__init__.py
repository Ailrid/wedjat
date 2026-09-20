"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import math
import os
import random
from typing import Optional, Tuple
import cv2
import numpy as np
import rasterio
from rasterio.windows import Window
import torch
from torch.utils.data import DataLoader, Dataset, IterableDataset
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





class CrossViewDataset(Dataset):
    """
    Dataset loader for cross-view (Drone & Satellite) matching dataset.

    Directory structure expected:
    dataset_dir/
        ├── satellite/
        │   ├── 000001/
        │   │   └── xxx.jpg (1 satellite image)
        │   └── 000002/ ...
        └── drone/
            ├── 000001/
            │   ├── yyy1.jpg
            │   └── yyy2.jpg (multiple drone images)
            └── 000002/ ...
    """

    def __init__(
        self,
        root_dir: str,
        input_size: Tuple[int, int] = (224, 224),
        drone_samples_per_location: int = 1,
        max_rotation: float = 30.0,
        max_pitch: float = 10.0,
        transform=None,
        is_train: bool = True,
    ):
        super().__init__()
        self.root_dir = root_dir
        self.input_size = input_size  # (height, width)
        self.drone_samples_per_location = drone_samples_per_location
        self.max_rotation = max_rotation
        self.max_pitch = max_pitch
        self.transform = transform
        self.is_train = is_train

        self.satellite_dir = os.path.join(self.root_dir, "satellite")
        self.drone_dir = os.path.join(self.root_dir, "drone")

        # Collect matching location folder names (e.g. '000001', '000002')
        sat_folders = {
            f
            for f in os.listdir(self.satellite_dir)
            if os.path.isdir(os.path.join(self.satellite_dir, f)) and f.isdigit()
        }
        drone_folders = {
            f
            for f in os.listdir(self.drone_dir)
            if os.path.isdir(os.path.join(self.drone_dir, f)) and f.isdigit()
        }

        self.location_ids = sorted(list(sat_folders.intersection(drone_folders)))

        if len(self.location_ids) == 0:
            raise ValueError(f"No matching location folders found in {self.root_dir}")

        # Pre-index image file paths for quick loading
        self.dataset_index = []
        for loc_id in self.location_ids:
            sat_loc_path = os.path.join(self.satellite_dir, loc_id)
            drone_loc_path = os.path.join(self.drone_dir, loc_id)

            sat_images = [
                os.path.join(sat_loc_path, img)
                for img in os.listdir(sat_loc_path)
                if img.lower().endswith((".jpg", ".jpeg", ".png"))
            ]
            drone_images = [
                os.path.join(drone_loc_path, img)
                for img in os.listdir(drone_loc_path)
                if img.lower().endswith((".jpg", ".jpeg", ".png"))
            ]

            if sat_images and drone_images:
                self.dataset_index.append(
                    {
                        "id": loc_id,
                        "satellite": sat_images[0],  # Take the single satellite image
                        "drone": drone_images,  # Keep all drone images list
                    }
                )

    def _get_3d_perspective_matrix(
        self, w: int, h: int, pitch: float, roll: float, yaw: float, fov: float = 60.0
    ):
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

    def _apply_perspective(self, img: np.ndarray) -> np.ndarray:
        """Apply 3D perspective distortion for augmentation during training."""
        h, w = img.shape[:2]
        pitch = np.random.uniform(0, self.max_pitch)
        roll = np.random.uniform(-10, 10)
        yaw = np.random.uniform(-self.max_rotation, self.max_rotation)

        H = self._get_3d_perspective_matrix(w, h, pitch, roll, yaw, fov=60)

        warped = cv2.warpPerspective(
            img,
            H,
            (w, h),
            flags=cv2.INTER_LINEAR,
            borderMode=cv2.BORDER_REFLECT,
        )
        return warped

    def _load_and_preprocess_image(
        self, img_path: str, is_drone: bool = False
    ) -> torch.Tensor:
        """Load image from disk, resize, apply optional geometric warping, and convert to Tensor."""
        img = cv2.imread(img_path)
        if img is None:
            raise FileNotFoundError(f"Failed to read image at: {img_path}")
        img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)

        # Apply perspective distortion during training (e.g. for drone images)
        if self.is_train and is_drone and (self.max_pitch > 0 or self.max_rotation > 0):
            img = self._apply_perspective(img)

        # Resize to specified target size (width, height)
        target_w, target_h = self.input_size
        if (img.shape[1], img.shape[0]) != (target_w, target_h):
            img = cv2.resize(img, (target_w, target_h), interpolation=cv2.INTER_LINEAR)

        tensor_img = torch.from_numpy(img).permute(2, 0, 1).contiguous()

        if self.transform:
            tensor_img = self.transform(tensor_img)

        return tensor_img

    def __len__(self) -> int:
        return len(self.dataset_index)

    def __getitem__(self, idx: int):
        item = self.dataset_index[idx]

        # Load satellite image
        sat_tensor = self._load_and_preprocess_image(item["satellite"], is_drone=False)

        # Sample drone image(s)
        drone_paths = item["drone"]
        if self.is_train:
            # Randomly select N drone images if multiple available
            selected_drone_paths = random.choices(
                drone_paths, k=self.drone_samples_per_location
            )
        else:
            # During eval/test, select deterministically or all available up to specified count
            selected_drone_paths = drone_paths[: self.drone_samples_per_location]
            if len(selected_drone_paths) < self.drone_samples_per_location:
                # Fill up if not enough samples
                selected_drone_paths += [drone_paths[0]] * (
                    self.drone_samples_per_location - len(selected_drone_paths)
                )

        drone_tensors = [
            self._load_and_preprocess_image(path, is_drone=True)
            for path in selected_drone_paths
        ]

        if self.drone_samples_per_location == 1:
            drone_out = drone_tensors[0]
        else:
            drone_out = torch.stack(drone_tensors)

        return sat_tensor, drone_out, item["id"]


def get_cross_view_dataloader(
    root_dir: str,
    input_size: Tuple[int, int] = (256, 256),
    batch_size: int = 8,
    drone_samples_per_location: int = 1,
    is_train: bool = True,
    num_workers: int = 4,
    shuffle: Optional[bool] = None,
):
    """DataLoader builder helper function."""
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

    dataset = CrossViewDataset(
        root_dir=root_dir,
        input_size=input_size,
        drone_samples_per_location=drone_samples_per_location,
        max_rotation=30.0 if is_train else 0.0,
        max_pitch=10.0 if is_train else 0.0,
        transform=tensor_transforms,
        is_train=is_train,
    )

    should_shuffle = is_train if shuffle is None else shuffle

    return DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=should_shuffle,
        num_workers=num_workers,
        pin_memory=True,
    )
