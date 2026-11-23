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

from metric.export import MobileV2_MLSD_Large

MLSD_WEIGHT = (
    "/home/shiraha_yuki/文档/project/wedjat/metric/assets/mlsd_large_512_fp32.pth"
)


# =============================================================
# 1. 导出
# =============================================================
def export_mlsd(
    output_path: str = "mlsd_tiny_512.onnx",
    weight_path: str = MLSD_WEIGHT,
    input_size: int = 512,
) -> None:
    """导出 M-LSD Tiny 到 ONNX（static shape）。"""
    model = MobileV2_MLSD_Large()  # type: ignore
    model.eval()

    state = torch.load(weight_path, map_location="cpu")
    if isinstance(state, dict) and "state_dict" in state:
        state = state["state_dict"]
    state = {k.replace("module.", ""): v for k, v in state.items()}

    first_key = "backbone.features.0.0.weight"
    if first_key in state:
        print(f"[Info] 权重第一层 shape: {tuple(state[first_key].shape)}")
    print(f"[Info] 模型第一层 shape: {tuple(model.state_dict()[first_key].shape)}")

    model.load_state_dict(state, strict=True)
    model.eval()

    dummy_input = torch.randn(1, 4, input_size, input_size, dtype=torch.float32)

    torch.onnx.export(
        model,
        dummy_input,  # type: ignore
        output_path,
        export_params=True,
        opset_version=12,
        do_constant_folding=True,
        input_names=["input"],
        output_names=["raw_map"],
        dynamic_axes=None,
    )

    print(f"Static ONNX model successfully exported to {output_path}")


# =============================================================
# 2. 后处理（分两步：解码 + 去重过滤）
# =============================================================
def _decode_mlsd_raw(
    raw_map: torch.Tensor,
    input_size: int = 512,
    topk: int = 300,
) -> tuple[np.ndarray, np.ndarray]:
    """
    从 raw_map 解码出**所有**候选线段（不做长度/分数过滤，只做 NMS 和 topk）。
    返回:
        lines:  [N, 4]  (x1, y1, x2, y2)，input_size 空间
        scores: [N]
    """
    b, c, hm, wm = raw_map.shape
    assert c == 9, f"unexpected channels: {c}"

    with torch.no_grad():
        # 1. sigmoid
        center = torch.sigmoid(raw_map[:, 0])  # [B, Hm, Wm]

        # 2. 3×3 NMS
        hmax = F.max_pool2d(
            center.unsqueeze(1), kernel_size=3, stride=1, padding=1
        ).squeeze(1)
        heat = center * (hmax == center).float()
        heat = heat.reshape(-1)

        # 3. topk
        scores, indices = torch.topk(heat, topk, largest=True)
        ys = torch.div(indices, wm, rounding_mode="floor")
        xs = torch.fmod(indices, wm)

        # 4. 位移（通道顺序: x_s, y_s, x_e, y_e）
        disp = raw_map[0, 1:5].permute(1, 2, 0)  # [Hm, Wm, 4]

        # 5. 解码端点（map 空间）
        disp_sel = disp[ys, xs]                  # [N, 4]
        x1 = xs.float() + disp_sel[:, 0]
        y1 = ys.float() + disp_sel[:, 1]
        x2 = xs.float() + disp_sel[:, 2]
        y2 = ys.float() + disp_sel[:, 3]

        lines = torch.stack([x1, y1, x2, y2], dim=-1)

        # 6. 缩放：map → input_size
        scale = float(input_size) / float(wm)
        lines = lines * scale

        return lines.numpy().astype(np.float32), scores.numpy().astype(np.float32)


def _segment_nms(
    lines: np.ndarray,
    scores: np.ndarray,
    mid_dist_thr: float = 15.0,
    angle_thr_deg: float = 8.0,
) -> np.ndarray:
    """
    线段级 NMS：如果两条线的中点距离 < mid_dist_thr 且方向相近，
    认为是同一根物理边缘，保留分数高的。
    返回保留的索引。
    """
    if len(lines) == 0:
        return np.zeros((0,), dtype=np.int64)

    order = np.argsort(-scores)
    mids = np.stack([(lines[:, 0] + lines[:, 2]) * 0.5,
                     (lines[:, 1] + lines[:, 3]) * 0.5], axis=-1)  # [N, 2]
    angles = np.degrees(np.arctan2(lines[:, 3] - lines[:, 1],
                                   lines[:, 2] - lines[:, 0])) % 180.0  # [N]

    keep = []
    for i in order:
        dup = False
        for j in keep:
            d_mid = np.hypot(mids[i, 0] - mids[j, 0], mids[i, 1] - mids[j, 1])
            if d_mid > mid_dist_thr:
                continue
            da = abs(angles[i] - angles[j])
            da = min(da, 180.0 - da)
            if da < angle_thr_deg:
                dup = True
                break
        if not dup:
            keep.append(int(i))
    return np.asarray(keep, dtype=np.int64)


def post_process_mlsd(
    raw_map: torch.Tensor,
    input_size: int = 512,
    score_thr: float = 0.05,     # ★ 降低，比官方 0.10 宽松，短线也能过
    topk: int = 300,             # ★ 提高，给 NMS 更多候选
    min_len: float = 15.0,       # ★ 输入空间最小线长（px），按需调
    nms_mid_dist: float = 15.0,  # ★ NMS：中点距离阈值
    nms_angle_deg: float = 8.0,  # ★ NMS：方向差阈值
    do_nms: bool = True,         # 关掉可看原始解码效果
) -> tuple[np.ndarray, np.ndarray]:
    """
    M-LSD 后处理完整流程：
      1) 从 raw_map 解码出所有候选线
      2) 按 score_thr 过滤
      3) 按 min_len（输入空间）过滤
      4) 线段级 NMS 去重
    """
    lines, scores = _decode_mlsd_raw(raw_map, input_size, topk)

    # 分数过滤
    keep = scores > score_thr
    lines, scores = lines[keep], scores[keep]

    # 长度过滤（输入空间）
    if len(lines) > 0:
        L = np.hypot(lines[:, 2] - lines[:, 0], lines[:, 3] - lines[:, 1])
        keep = L >= min_len
        lines, scores = lines[keep], scores[keep]

    # 线段级 NMS
    if do_nms and len(lines) > 1:
        keep_idx = _segment_nms(lines, scores, nms_mid_dist, nms_angle_deg)
        lines, scores = lines[keep_idx], scores[keep_idx]

    return lines, scores


# =============================================================
# 3. 预处理
# =============================================================
def preprocess_image(
    image_path: str, target_size: tuple[int, int] = (512, 512)
) -> np.ndarray:
    """4 通道 RGBA、/127.5-1.0、CHW → [1, 4, H, W]"""
    img = cv2.imread(image_path, cv2.IMREAD_UNCHANGED)
    if img is None:
        raise FileNotFoundError(f"Could not load image at {image_path}")

    if img.ndim == 2:
        img = cv2.cvtColor(img, cv2.COLOR_GRAY2BGR)
    if img.shape[2] == 3:
        alpha = np.full((img.shape[0], img.shape[1], 1), 255, dtype=img.dtype)
        img = np.concatenate([img, alpha], axis=-1)

    img_resized = cv2.resize(img, target_size)
    tensor_img = img_resized.astype(np.float32) / 127.5 - 1.0
    tensor_img = tensor_img.transpose(2, 0, 1)
    tensor_img = np.expand_dims(tensor_img, axis=0)
    return tensor_img


# =============================================================
# 4. 测试
# =============================================================
def run_onnx_test(
    onnx_path: str,
    image_path: str,
    output_image_path: str = "mlsd_output.png",
    input_size: int = 512,
    score_thr: float = 0.05,     # ★ 和 post_process_mlsd 对齐
    topk: int = 300,
    min_len: float = 15.0,
    nms_mid_dist: float = 15.0,
    nms_angle_deg: float = 8.0,
    color_by_score: bool = True, # ★ 按分数上色，方便判断哪些高分
) -> None:
    img_data = preprocess_image(image_path, target_size=(input_size, input_size))

    session = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
    outputs = session.run(["raw_map"], {"input": img_data})
    raw_map = torch.from_numpy(outputs[0])

    lines, scores = post_process_mlsd(
        raw_map,
        input_size=input_size,
        score_thr=score_thr,
        topk=topk,
        min_len=min_len,
        nms_mid_dist=nms_mid_dist,
        nms_angle_deg=nms_angle_deg,
    )

    print(f"Detected lines: {len(lines)}")
    if len(scores) > 0:
        print(f"Score range: [{scores.min():.3f}, {scores.max():.3f}]")
        L = np.hypot(lines[:, 2] - lines[:, 0], lines[:, 3] - lines[:, 1])
        print(f"Length range: [{L.min():.1f}, {L.max():.1f}] px")

    # 可视化
    img = cv2.imread(image_path, cv2.IMREAD_COLOR)
    if img is None:
        raise FileNotFoundError(image_path)
    img = cv2.resize(img, (input_size, input_size))

    for idx, (x1, y1, x2, y2) in enumerate(lines):
        if color_by_score:
            s = float(scores[idx])
            # 高分 → 绿，中分 → 黄，低分 → 橙
            if s > 0.3:
                color = (0, 255, 0)
            elif s > 0.15:
                color = (0, 255, 255)
            else:
                color = (0, 165, 255)
        else:
            color = (0, 255, 0)
        cv2.line(img, (int(x1), int(y1)), (int(x2), int(y2)),
                 color, 2, cv2.LINE_AA)

    # 左上角统计文字
    cv2.putText(img, f"N={len(lines)}", (10, 30),
                cv2.FONT_HERSHEY_SIMPLEX, 0.9, (0, 0, 255), 2, cv2.LINE_AA)

    cv2.imwrite(output_image_path, img)
    print(f"Visualization result saved to: {output_image_path}")


if __name__ == "__main__":
    onnx_model_file = "assets/mlsd_tiny_512.onnx"
    test_image_file = "assets/image.png"

    export_mlsd(
        output_path=onnx_model_file,
        weight_path=MLSD_WEIGHT,
        input_size=512,
    )

    run_onnx_test(
        onnx_path=onnx_model_file,
        image_path=test_image_file,
        output_image_path="mlsd_output.png",
        input_size=512,
        score_thr=0.05,      # ★ 从 0.10 降下来，短线能过
        topk=200,            # ★ 从 200 提高，多给候选
        min_len=20.0,        # ★ 输入空间最小长度
        nms_mid_dist=15.0,   # ★ 中点近于 15px 且方向近的，判定为重复
        nms_angle_deg=8.0,
    )
