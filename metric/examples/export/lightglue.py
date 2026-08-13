"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import cv2
import numpy as np
import torch
import onnxruntime as ort
from metric.export.lightglue import LightGlue
from superpoint import post_process_superpoint


def export_lightglue(
    output_path: str = "lightglue_static.onnx",
) -> None:
    model = LightGlue().eval()
    dummy_keypoints = torch.randn(2, 1024, 2, dtype=torch.float32)
    dummy_descriptors = torch.randn(2, 1024, 256, dtype=torch.float32)

    torch.onnx.export(
        model,
        (dummy_keypoints, dummy_descriptors),
        output_path,
        export_params=True,
        opset_version=14,
        do_constant_folding=True,
        input_names=["keypoints", "descriptors"],
        output_names=["scores"],
        dynamic_axes=None,
    )
    print(f"Static LightGlue ONNX model exported to {output_path}")


def preprocess_image(
    image_path: str, target_size: tuple[int, int] = (256, 256)
) -> np.ndarray:
    img = cv2.imread(image_path, cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise FileNotFoundError(f"Could not load image at {image_path}")
    img_resized = cv2.resize(img, target_size)
    tensor_img = img_resized.astype(np.float32) / 255.0
    return np.expand_dims(tensor_img, axis=(0, 1))  # (1, 1, H, W)


def extract_features_single_image(
    sp_session: ort.InferenceSession,
    image_path: str,
    num_keypoints: int = 1024,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    img_data = preprocess_image(image_path, target_size=(256, 256))

    onnx_inputs = {"input": img_data}
    outputs = sp_session.run(["scores", "descriptors"], onnx_inputs)
    scores_np, descriptors_np = outputs[0], outputs[1]

    scores_tensor = torch.from_numpy(scores_np)
    descriptors_tensor = torch.from_numpy(descriptors_np)

    keypoints, keypoint_descriptors = post_process_superpoint(
        scores_tensor, descriptors_tensor, num_keypoints=num_keypoints
    )

    return (
        keypoints.numpy(),  # Shape: (1, 1024, 2)
        keypoint_descriptors.numpy(),  # Shape: (1, 1024, 256)
        img_data[0, 0],  # Shape: (256, 256)
    )


def draw_matches(
    img0: np.ndarray,
    img1: np.ndarray,
    kpts0: np.ndarray,
    kpts1: np.ndarray,
    matches: np.ndarray,
    output_path: str = "match_result.png",
) -> None:
    img0_bgr = cv2.cvtColor((img0 * 255).astype(np.uint8), cv2.COLOR_GRAY2BGR)
    img1_bgr = cv2.cvtColor((img1 * 255).astype(np.uint8), cv2.COLOR_GRAY2BGR)

    h0, w0 = img0_bgr.shape[:2]
    h1, w1 = img1_bgr.shape[:2]

    canvas = np.zeros((max(h0, h1), w0 + w1, 3), dtype=np.uint8)
    canvas[:h0, :w0] = img0_bgr
    canvas[:h1, w0 : w0 + w1] = img1_bgr

    for idx0, idx1 in matches:
        pt0 = tuple(map(int, kpts0[idx0]))
        pt1 = (int(kpts1[idx1][0]) + w0, int(kpts1[idx1][1]))

        color = tuple(map(int, np.random.randint(0, 255, size=3)))

        cv2.circle(canvas, pt0, radius=3, color=(0, 0, 255), thickness=-1)
        cv2.circle(canvas, pt1, radius=3, color=(0, 0, 255), thickness=-1)
        cv2.line(canvas, pt0, pt1, color=color, thickness=1, lineType=cv2.LINE_AA)

    cv2.imwrite(output_path, canvas)
    print(f"Match visualization saved to: {output_path}")


def draw_homography_warp(
    img0: np.ndarray,
    img1: np.ndarray,
    kpts0: np.ndarray,
    kpts1: np.ndarray,
    matches: np.ndarray,
    output_path: str = "warp_result.png",
) -> None:
    if len(matches) < 4:
        print("At least 4 matching points are required to calculate homography.")
        return

    # Extract matched keypoint coordinates
    src_pts = kpts0[matches[:, 0]].reshape(-1, 1, 2)
    dst_pts = kpts1[matches[:, 1]].reshape(-1, 1, 2)

    # Compute homography matrix using RANSAC
    H, mask = cv2.findHomography(src_pts, dst_pts, cv2.RANSAC, 5.0)

    if H is None:
        print("Homography computation failed.")
        return

    img0_bgr = cv2.cvtColor((img0 * 255).astype(np.uint8), cv2.COLOR_GRAY2BGR)
    img1_bgr = cv2.cvtColor((img1 * 255).astype(np.uint8), cv2.COLOR_GRAY2BGR)

    h1, w1 = img1_bgr.shape[:2]

    # Warp img0 to img1's coordinate space
    warped_img0 = cv2.warpPerspective(img0_bgr, H, (w1, h1))

    # Blend warped img0 and img1 for visual comparison
    mask_warped = (warped_img0 > 0).astype(np.uint8)
    blended = img1_bgr.copy()
    overlap_area = mask_warped > 0
    blended[overlap_area] = cv2.addWeighted(
        img1_bgr[overlap_area], 0.5, warped_img0[overlap_area], 0.5, 0
    )

    cv2.imwrite(output_path, blended)
    print(f"Warp visualization saved to: {output_path}")


def run_matching_pipeline(
    sp_onnx_path: str,
    lg_onnx_path: str,
    img0_path: str,
    img1_path: str,
    output_image_path: str = "match_result.png",
    warp_image_path: str = "warp_result.png",
    match_threshold: float = 0.0,
    num_keypoints: int = 1024,
) -> None:
    sp_session = ort.InferenceSession(sp_onnx_path, providers=["CPUExecutionProvider"])
    lg_session = ort.InferenceSession(lg_onnx_path, providers=["CPUExecutionProvider"])

    kpts0, desc0, img0_data = extract_features_single_image(
        sp_session, img0_path, num_keypoints
    )
    kpts1, desc1, img1_data = extract_features_single_image(
        sp_session, img1_path, num_keypoints
    )

    lg_kpts_input = np.concatenate([kpts0, kpts1], axis=0)
    lg_desc_input = np.concatenate([desc0, desc1], axis=0)

    lg_inputs = {
        "keypoints": lg_kpts_input,
        "descriptors": lg_desc_input,
    }
    lg_outputs = lg_session.run(["scores"], lg_inputs)
    log_assignment = lg_outputs[0]

    scores = np.exp(log_assignment[0]) # type: ignore

    max_indices0 = np.argmax(scores, axis=1)
    max_scores0 = np.max(scores, axis=1)

    max_indices1 = np.argmax(scores, axis=0)

    matches = []
    for idx0, idx1 in enumerate(max_indices0):
        if max_indices1[idx1] == idx0 and max_scores0[idx0] > match_threshold:
            matches.append((idx0, idx1))

    matches = np.array(matches)
    print(f"Total valid matches found: {len(matches)}")

    draw_matches(
        img0_data,
        img1_data,
        kpts0[0],
        kpts1[0],
        matches,
        output_path=output_image_path,
    )

    draw_homography_warp(
        img0_data,
        img1_data,
        kpts0[0],
        kpts1[0],
        matches,
        output_path=warp_image_path,
    )


if __name__ == "__main__":
    sp_onnx_file = "assets/superpoint.onnx"
    lg_onnx_file = "assets/lightglue.onnx"
    img0_file = "assets/test1.png"
    img1_file = "assets/test2.png"

    export_lightglue(lg_onnx_file)

    run_matching_pipeline(
        sp_onnx_path=sp_onnx_file,
        lg_onnx_path=lg_onnx_file,
        img0_path=img0_file,
        img1_path=img1_file,
        output_image_path="lightglue_match_result.png",
        warp_image_path="lightglue_warp_result.png",
        match_threshold=0.1,
    )
