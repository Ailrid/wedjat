"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import cv2
import numpy as np
import torch
import torch.nn.functional as F
import onnxruntime as ort

from metric.export.superpoint import SuperPoint


def export_superpoint(output_path: str = "superpoint_static.onnx") -> None:

    model = SuperPoint().eval()

    dummy_input = torch.randn(1, 1, 256, 256, dtype=torch.float32)

    torch.onnx.export(
        model,
        dummy_input,  # type: ignore
        output_path,
        export_params=True,
        opset_version=12,
        do_constant_folding=True,
        input_names=["input"],
        output_names=["scores", "descriptors"],
        dynamic_axes=None,
    )

    print(f"Static ONNX model successfully exported to {output_path}")


def simple_nms(scores: torch.Tensor, nms_radius: int = 4) -> torch.Tensor:
    def max_pool(x: torch.Tensor) -> torch.Tensor:
        return F.max_pool2d(
            x, kernel_size=nms_radius * 2 + 1, stride=1, padding=nms_radius
        )

    zeros = torch.zeros_like(scores)
    max_mask = scores == max_pool(scores)
    for _ in range(2):
        supp_mask = max_pool(max_mask.float()).bool()
        supp_scores = torch.where(supp_mask, zeros, scores)
        new_max_mask = supp_scores == max_pool(supp_scores)
        max_mask = max_mask | (new_max_mask & (~supp_mask))
    return torch.where(max_mask, scores, zeros)


def post_process_superpoint(
    scores: torch.Tensor,
    descriptors: torch.Tensor,
    num_keypoints: int = 1024,
    nms_radius: int = 4,
    remove_borders: int = 4,
) -> tuple[torch.Tensor, torch.Tensor]:

    b, _, h, w = scores.shape

    # Apply Non-Maximum Suppression
    scores = simple_nms(scores, nms_radius=nms_radius)

    # Discard keypoints near borders
    if remove_borders > 0:
        scores[:, :, :remove_borders, :] = -1
        scores[:, :, -remove_borders:, :] = -1
        scores[:, :, :, :remove_borders] = -1
        scores[:, :, :, -remove_borders:] = -1

    # Flatten scores to select top-k keypoints
    scores_flat = scores.reshape(b, -1)
    _, top_indices = scores_flat.topk(num_keypoints, dim=1)

    # Convert 1D flat indices back to 2D coordinates (Y, X)
    top_y = torch.div(top_indices, w, rounding_mode="floor")
    top_x = top_indices % w

    # Keypoints in <X, Y> order for OpenCV / LightGlue format
    keypoints = torch.stack([top_x, top_y], dim=-1).float()  # (B, K, 2)

    # Sample descriptors using grid_sample with normalized keypoint coordinates
    grid_x = (top_x.float() / (w - 1)) * 2.0 - 1.0
    grid_y = (top_y.float() / (h - 1)) * 2.0 - 1.0
    grid = torch.stack([grid_x, grid_y], dim=-1).unsqueeze(1)  # (B, 1, K, 2)

    # Sample from (B, 256, H/8, W/8) feature map
    keypoint_descriptors = F.grid_sample(
        descriptors, grid, mode="bilinear", align_corners=True
    ).squeeze(
        2
    )  # (B, 256, K)

    # Re-normalize extracted descriptors and permute to (B, K, 256)
    keypoint_descriptors = F.normalize(keypoint_descriptors, p=2, dim=1)
    keypoint_descriptors = keypoint_descriptors.permute(0, 2, 1)

    return keypoints, keypoint_descriptors


def preprocess_image(
    image_path: str, target_size: tuple[int, int] = (256, 256)
) -> np.ndarray:
    img = cv2.imread(image_path, cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise FileNotFoundError(f"Could not load image at {image_path}")
    img_resized = cv2.resize(img, target_size)
    tensor_img = img_resized.astype(np.float32) / 255.0

    # Expand dims to shape (1, 1, H, W) for Batch Size = 1
    tensor_img = np.expand_dims(tensor_img, axis=(0, 1))
    return tensor_img


def run_onnx_test(
    onnx_path: str,
    image_path: str,
    output_image_path: str = "result_keypoints.png",
    num_keypoints: int = 1024,
) -> None:
    img_data = preprocess_image(image_path, target_size=(256, 256))

    # Initialize ONNX Runtime Session
    session = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])

    # Run Inference
    onnx_inputs = {"input": img_data}
    outputs = session.run(["scores", "descriptors"], onnx_inputs)
    scores_np, descriptors_np = outputs[0], outputs[1]

    # Convert to PyTorch Tensors for post-processing
    scores_tensor = torch.from_numpy(scores_np)
    descriptors_tensor = torch.from_numpy(descriptors_np)

    # Post-process to extract top keypoints and descriptors
    keypoints, keypoint_descriptors = post_process_superpoint(
        scores_tensor, descriptors_tensor, num_keypoints=num_keypoints
    )

    print(f"Extracted keypoints shape: {keypoints.shape}")  # Should be (1, 1024, 2)
    print(
        f"Extracted descriptors shape: {keypoint_descriptors.shape}"
    )  # Should be (1, 1024, 256)

    # Extract 2D image array for visualization (1, 1, H, W) -> (H, W)
    img_2d = img_data[0, 0]
    img_color = cv2.cvtColor((img_2d * 255).astype(np.uint8), cv2.COLOR_GRAY2BGR)
    kpts_first_img = keypoints[0].numpy()

    for pt in kpts_first_img:
        x, y = int(pt[0]), int(pt[1])
        cv2.circle(img_color, (x, y), radius=2, color=(0, 255, 0), thickness=-1)

    cv2.imwrite(output_image_path, img_color)
    print(f"Visualization result saved to: {output_image_path}")


if __name__ == "__main__":

    onnx_model_file = "assets/superpoint.onnx"
    test_image_file = "assets/test1.png"
    export_superpoint(onnx_model_file)

    run_onnx_test(
        onnx_path=onnx_model_file,
        image_path=test_image_file,
        output_image_path="superpoint_output.png",
        num_keypoints=1024,
    )
