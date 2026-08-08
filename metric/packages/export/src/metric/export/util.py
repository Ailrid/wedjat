import csv
import glob
import os
from typing import Dict, List, Optional, Tuple
from rasterio.windows import Window
import cv2
import matplotlib.pyplot as plt
import numpy as np
import pyproj
import rasterio
from rasterio.crs import CRS
from qdrant_client import QdrantClient
from .refer import ONNXInferencer


def load_csv_metadata(csv_path: str) -> Dict[str, Dict[str, float]]:
    metadata = {}
    if not os.path.exists(csv_path):
        print(f"Warning: CSV file not found at {csv_path}")
        return metadata

    with open(csv_path, mode="r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            filename = row["file"].strip()
            try:
                metadata[filename] = {
                    "lat": float(row["lat"]),
                    "lon": float(row["lon"]),
                    "alt": float(row["alt"]),
                    "yaw": float(row["yaw"]),
                    "pitch": float(row["pitch"]),
                    "roll": float(row["roll"]),
                }
            except ValueError:
                continue
    return metadata


def preprocess_uav_image(
    image_rgb: np.ndarray,
    yaw: float,
    scale: float = 1.0,
    crop_size: int = 256,
) -> np.ndarray:

    h, w = image_rgb.shape[:2]

    # 1. 顺时针/逆时针旋转图像至正北 (假设 yaw 为偏离正北的角度)
    # 绕图像中心旋转
    center = (w / 2.0, h / 2.0)
    # 旋转矩阵（yaw 角度旋转）
    rotation_matrix = cv2.getRotationMatrix2D(center, -yaw, 1.0)

    # 计算旋转后的外包盒，防止图像边缘裁剪丢失
    cos = np.abs(rotation_matrix[0, 0])
    sin = np.abs(rotation_matrix[0, 1])
    new_w = int((h * sin) + (w * cos))
    new_h = int((h * cos) + (w * sin))

    # 调整变换矩阵平移量
    rotation_matrix[0, 2] += (new_w / 2) - center[0]
    rotation_matrix[1, 2] += (new_h / 2) - center[1]

    rotated_img = cv2.warpAffine(
        image_rgb,
        rotation_matrix,
        (new_w, new_h),
        flags=cv2.INTER_LINEAR,
        borderMode=cv2.BORDER_CONSTANT,
        borderValue=(0, 0, 0),
    )

    # 按比例缩放
    scaled_w = int(new_w * scale)
    scaled_h = int(new_h * scale)
    interpolation = cv2.INTER_AREA if scale < 1.0 else cv2.INTER_CUBIC
    # scaled_img = cv2.resize(
    #     rotated_img, (scaled_w, scaled_h), interpolation=interpolation
    # )
    scaled_img = cv2.resize(
        image_rgb, (scaled_w, scaled_h), interpolation=interpolation
    )

    # 3. 中心裁剪 / Pad 填充
    cropped_img = _center_crop_or_pad(scaled_img, crop_size)
    return cropped_img


def _center_crop_or_pad(image: np.ndarray, crop_size: int) -> np.ndarray:
    """对图像中心裁剪或填充补齐到 crop_size x crop_size."""
    h, w, _ = image.shape

    if h < crop_size:
        pad_h = crop_size - h
        top_pad = pad_h // 2
        bottom_pad = pad_h - top_pad
        image = np.pad(
            image,
            ((top_pad, bottom_pad), (0, 0), (0, 0)),
            mode="constant",
            constant_values=0,
        )
        h = crop_size

    if w < crop_size:
        pad_w = crop_size - w
        left_pad = pad_w // 2
        right_pad = pad_w - left_pad
        image = np.pad(
            image,
            ((0, 0), (left_pad, right_pad), (0, 0)),
            mode="constant",
            constant_values=0,
        )
        w = crop_size

    start_x = (w - crop_size) // 2
    start_y = (h - crop_size) // 2

    return image[start_y : start_y + crop_size, start_x : start_x + crop_size]


def lonlat_to_pixel(
    lon: float,
    lat: float,
    src_raster: rasterio.DatasetReader,
    transformer: Optional[pyproj.Transformer] = None,
) -> Tuple[int, int]:
    """将 (Lon, Lat) 经纬度换算为 TIF 图像的 (Pixel_X, Pixel_Y) 像素坐标."""
    if transformer:
        # 如果 TIF 的 CRS 不是 WGS84 经纬度（如 UTM），先投影转换
        x_proj, y_proj = transformer.transform(lon, lat)
    else:
        x_proj, y_proj = lon, lat

    # rasterio.index 使用地图投影坐标计算 Row(y), Col(x)
    row, col = src_raster.index(x_proj, y_proj)
    return int(col), int(row)


def test_img_folder_rank(
    folder_path: str,
    csv_path: str,
    tif_path: str,
    model_path: str,
    collection_name: str,
    input_size: int = 256,
    scale: float = 1.0,
    host: str = "127.0.0.1",
    port: int = 6333,
    visualize_top_k: int = 3,
    output_dir: str = "tif_star_search_visualizations",
) -> None:
    """Read image folder and metadata, run retrieval, save patch comparison plots,

    and plot all query results on a single combined map.
    """
    if not os.path.exists(folder_path) or not os.path.exists(tif_path):
        print("Error: Invalid folder_path or tif_path.")
        return

    csv_metadata = load_csv_metadata(csv_path)

    img_paths = sorted(
        glob.glob(os.path.join(folder_path, "*.png"))
        + glob.glob(os.path.join(folder_path, "*.jpg"))
        + glob.glob(os.path.join(folder_path, "*.jpeg"))
    )

    if not img_paths:
        print(f"Error: No image files found in {folder_path}")
        return

    inferencer = ONNXInferencer(model_path)
    client = QdrantClient(host=host, port=port)

    if visualize_top_k > 0:
        os.makedirs(output_dir, exist_ok=True)

    tif_filename = os.path.basename(tif_path)

    with rasterio.open(tif_path) as src_raster:
        tif_crs = src_raster.crs
        transformer = None
        if tif_crs and tif_crs != CRS.from_epsg(4326):
            transformer = pyproj.Transformer.from_crs(
                "EPSG:4326", tif_crs, always_xy=True
            )

        tif_w, tif_h = src_raster.width, src_raster.height
        print(f"TIF Dimension: {tif_w} x {tif_h}, CRS: {tif_crs}")

        # List to collect query points and search results for combined star map
        query_results = []

        for img_path in img_paths:
            raw_filename = os.path.basename(img_path)
            image_bgr = cv2.imread(img_path)
            if image_bgr is None:
                continue

            image_rgb = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2RGB)

            meta = csv_metadata.get(raw_filename)
            yaw = meta.get("yaw", 0.0) if meta else 0.0
            query_lon = meta.get("lon") if meta else None
            query_lat = meta.get("lat") if meta else None

            # 1. Preprocess UAV image
            processed_img = preprocess_uav_image(
                image_rgb=image_rgb,
                yaw=yaw,
                scale=scale,
                crop_size=input_size,
            )

            # 2. Extract features
            data = np.transpose(processed_img, (2, 0, 1))
            feat: np.ndarray = inferencer.infer(data)
            query_vector = feat.flatten().tolist()

            # 3. Query Qdrant
            search_result = client.query_points(
                collection_name=collection_name,
                query=query_vector,
                limit=5,
            )
            points = search_result.points

            # 4. Generate visualizations if within visualize_top_k limit
            if len(query_results) < visualize_top_k and query_lon and query_lat:
                query_px, query_py = lonlat_to_pixel(
                    query_lon, query_lat, src_raster, transformer
                )

                # Collect data for final combined star map plot
                query_results.append(
                    {
                        "filename": raw_filename,
                        "query_pixel": (query_px, query_py),
                        "search_points": points,
                    }
                )

                # Determine hit rank based on ground truth spatial distance
                hit_rank = None
                for idx, point in enumerate(points, start=1):
                    payload = point.payload or {}
                    px, py = payload.get("pixel_x"), payload.get("pixel_y")
                    match_src = payload.get("src")
                    if px is not None and py is not None:
                        if (
                            match_src == tif_filename
                            and abs(px - query_px) < input_size
                            and abs(py - query_py) < input_size
                        ):
                            hit_rank = idx
                            break

                # Generate 1x6 patch comparison plot (Query vs Top-5 Matches)
                plot_patch_search_result(
                    src_raster=src_raster,
                    query_data=data,
                    query_pos=(query_px, query_py),
                    search_points=points,
                    crop_size=input_size,
                    hit_rank=hit_rank,
                    sample_idx=len(query_results),
                    output_dir=output_dir,
                    tif_filename=tif_filename,
                )

        # Draw all collected queries on a single combined star map
        if query_results:
            _draw_combined_star_map_visualization(
                src_raster=src_raster,
                transformer=transformer,
                query_results=query_results,
                output_dir=output_dir,
            )


def _draw_combined_star_map_visualization(
    src_raster: rasterio.DatasetReader,
    transformer: Optional[pyproj.Transformer],
    query_results: List[dict],
    output_dir: str,
) -> None:
    """Render all query points and their retrieved matches on a single TIF canvas."""
    tif_w, tif_h = src_raster.width, src_raster.height

    dpi = 100
    fig, ax = plt.subplots(figsize=(tif_w / dpi, tif_h / dpi), dpi=dpi)

    # Render background map
    try:
        overview_factor = max(1, int(max(tif_w, tif_h) / 4096))
        bg_data = src_raster.read(
            [1, 2, 3],
            out_shape=(
                3,
                int(tif_h / overview_factor),
                int(tif_w / overview_factor),
            ),
        )
        bg_img = np.transpose(bg_data, (1, 2, 0))
        if bg_img.max() > 1.0:
            bg_img = bg_img / 255.0
        ax.imshow(bg_img, extent=[0, tif_w, tif_h, 0])  # type: ignore
    except Exception as e:
        print(f"Warning: Failed to render background image, plotting schema only: {e}")
        ax.set_facecolor("black")
        ax.set_xlim(0, tif_w)
        ax.set_ylim(tif_h, 0)

    # Assign distinct base colors for each query
    query_colors = ["#FFD700", "#00FFFF", "#FF00FF", "#00FF00", "#FF4500"]

    for q_idx, q_data in enumerate(query_results, start=1):
        q_filename = q_data["filename"]
        q_x, q_y = q_data["query_pixel"]
        search_points = q_data["search_points"]

        q_color = query_colors[(q_idx - 1) % len(query_colors)]

        # 1. Plot Query center location
        ax.scatter(
            q_x,
            q_y,
            c=q_color,
            edgecolors="red",
            s=350,
            marker="*",
            zorder=5,
        )
        ax.annotate(
            f"Q{q_idx}: {q_filename}",
            (q_x, q_y),
            textcoords="offset points",
            xytext=(0, 12),
            ha="center",
            color=q_color,
            fontsize=11,
            fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.2", fc="black", alpha=0.6),
        )

        # 2. Plot Top-5 matches for current query
        for rank_idx, point in enumerate(search_points, start=1):
            payload = point.payload or {}
            score = getattr(point, "score", 0.0)

            px = payload.get("pixel_x")
            py = payload.get("pixel_y")

            if px is None or py is None:
                loc = payload.get("location", {})
                lon, lat = loc.get("lon"), loc.get("lat")
                if lon is not None and lat is not None:
                    px, py = lonlat_to_pixel(lon, lat, src_raster, transformer)

            if px is not None and py is not None:
                # Plot connecting line
                ax.plot(
                    [q_x, px],
                    [q_y, py],
                    linestyle="--",
                    linewidth=1.5,
                    color=q_color,
                    alpha=0.7,
                    zorder=3,
                )

                # Plot match marker
                ax.scatter(
                    px,
                    py,
                    c=q_color,
                    s=120,
                    marker="o",
                    zorder=4,
                    edgecolors="white",
                )

                # Annotate Rank and Score
                ax.annotate(
                    f"Q{q_idx}-R{rank_idx}\n{score:.3f}",
                    (px, py),
                    textcoords="offset points",
                    xytext=(0, -16),
                    ha="center",
                    color="white",
                    fontsize=9,
                    fontweight="bold",
                    bbox=dict(boxstyle="round,pad=0.2", fc="black", alpha=0.7),
                )

    ax.set_title(
        f"Combined Rank-5 Retrieval Star Map ({len(query_results)} Queries)",
        fontsize=16,
        color="white",
        pad=10,
    )
    ax.axis("off")

    save_path = os.path.join(output_dir, "combined_star_map_visualization.png")
    plt.tight_layout()
    plt.savefig(save_path, dpi=dpi, bbox_inches="tight", facecolor="black")
    plt.close(fig)
    print(f"Saved combined star-map visualization to: {save_path}")


def plot_patch_search_result(
    src_raster: rasterio.DatasetReader,
    query_data: np.ndarray,
    query_pos: Tuple[int, int],
    search_points: List,
    crop_size: int,
    hit_rank: Optional[int],
    sample_idx: int,
    output_dir: str,
    tif_filename: Optional[str] = None,
) -> None:
    """Render and save a 1x6 panel comparing Query patch with Top-5 retrieved patches."""
    fig, axes = plt.subplots(1, 6, figsize=(18, 3.5))

    # Convert CHW array to HWC for Matplotlib display
    query_img = np.transpose(query_data, (1, 2, 0))
    if query_img.max() > 1.0:
        query_img = query_img / 255.0
    query_img = np.clip(query_img, 0, 1)

    qx, qy = query_pos

    # Subplot 0: Query Patch
    axes[0].imshow(query_img)
    axes[0].set_title(f"Query Patch\nPos: ({qx}, {qy})", fontsize=10, color="blue")
    axes[0].axis("off")

    if tif_filename is None:
        tif_filename = os.path.basename(src_raster.name)

    # Subplots 1-5: Top-5 Matches
    for idx in range(5):
        ax = axes[idx + 1]

        if idx < len(search_points):
            point = search_points[idx]
            payload = point.payload or {}
            px = payload.get("pixel_x")
            py = payload.get("pixel_y")
            match_src = payload.get("src")
            score = getattr(point, "score", 0.0)

            if px is not None and py is not None:
                # Read patch window directly from GeoTIFF
                window = Window(int(px), int(py), int(crop_size), int(crop_size))  # type: ignore
                try:
                    patch_data = src_raster.read([1, 2, 3], window=window)
                    patch_img = np.transpose(patch_data, (1, 2, 0))
                    if patch_img.max() > 1.0:
                        patch_img = patch_img / 255.0
                    patch_img = np.clip(patch_img, 0, 1)

                    ax.imshow(patch_img)

                    # Determine ground truth match status based on spatial proximity
                    is_hit = (
                        match_src == tif_filename
                        and abs(px - qx) < crop_size
                        and abs(py - qy) < crop_size
                    )

                    title_color = "green" if is_hit else "red"
                    status_text = "MATCH" if is_hit else "MISS"

                    ax.set_title(
                        f"Rank {idx+1} [{status_text}]\nScore: {score:.4f}\nPos: ({px}, {py})",
                        fontsize=9,
                        color=title_color,
                    )
                except Exception as e:
                    ax.text(
                        0.5,
                        0.5,
                        f"Read Error\n{e}",
                        ha="center",
                        va="center",
                        fontsize=8,
                    )
                    ax.set_title(f"Rank {idx+1}", fontsize=9)
            else:
                ax.text(0.5, 0.5, "No Location", ha="center", va="center")
                ax.set_title(f"Rank {idx+1}", fontsize=9)
        else:
            ax.text(0.5, 0.5, "Empty", ha="center", va="center")
            ax.set_title(f"Rank {idx+1}", fontsize=9)

        ax.axis("off")

    hit_text = f"Rank {hit_rank}" if hit_rank else "None"
    plt.suptitle(
        f"Sample #{sample_idx} - Realtime Query Search (Hit Rank: {hit_text})",
        fontsize=12,
        fontweight="bold",
    )
    plt.tight_layout()

    os.makedirs(output_dir, exist_ok=True)
    save_path = os.path.join(output_dir, f"query_sample_{sample_idx}_rank5.png")
    plt.savefig(save_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Saved visualization plot: {save_path}")
