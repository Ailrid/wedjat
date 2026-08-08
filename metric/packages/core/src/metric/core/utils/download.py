"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

import glob
import json
import math
import os
from io import BytesIO
from concurrent.futures import ThreadPoolExecutor, as_completed

import mercantile
import rasterio
from rasterio.merge import merge
from rasterio.transform import from_bounds
import requests

HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
}


def get_zoom_for_resolution(resolution_meters):
    initial_resolution = 2 * math.pi * 6378137 / 256
    zoom = math.log2(initial_resolution / resolution_meters)
    return round(zoom)


def process_single_tile(tile, url_template, output_dir):
    """
    Download, georeference, and save a single map tile.
    Returns None on success, or a dict containing error metadata on failure.
    """
    x, y, z = tile.x, tile.y, tile.z
    file_name = f"{z}_{x}_{y}.tif"
    output_path = os.path.join(output_dir, file_name)

    if os.path.exists(output_path):
        return None

    try:
        url = url_template.format(x=x, y=y, z=z)
        response = requests.get(url, headers=HEADERS, timeout=15)
        response.raise_for_status()
        img_bytes = response.content

        bounds = mercantile.xy_bounds(x, y, z)
        with rasterio.open(BytesIO(img_bytes)) as src:
            transform = from_bounds(
                bounds.left,
                bounds.bottom,
                bounds.right,
                bounds.top,
                src.width,
                src.height,
            )
            with rasterio.open(
                output_path,
                "w",
                driver="GTiff",
                height=src.height,
                width=src.width,
                count=src.count,
                dtype=src.dtypes[0],
                crs="EPSG:3857",
                transform=transform,
                compress="lzw",
            ) as dst:
                dst.write(src.read())
        return None
    except Exception as e:
        return {
            "x": x,
            "y": y,
            "z": z,
            "url_template": url_template,
            "output_path": output_path,
            "error": str(e),
        }


def batch_download_parallel(
    bbox,
    url_template,
    output_dir,
    resolution=0.5,
    max_workers=10,
    retry_json_path="failed_tiles.json",
) -> None:
    """
    Batch download map tiles in parallel using ThreadPoolExecutor.
    """
    west, south, east, north = bbox
    zoom = get_zoom_for_resolution(resolution)
    tiles = list(mercantile.tiles(west, south, east, north, zoom))
    total_tiles = len(tiles)

    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    failed_list = []
    completed = 0

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_tile = {
            executor.submit(process_single_tile, t, url_template, output_dir): t
            for t in tiles
        }

        for future in as_completed(future_to_tile):
            result = future.result()
            completed += 1
            if result:
                failed_list.append(result)

            if completed % 10 == 0 or completed == total_tiles:
                print(
                    f"\rProgress: {completed}/{total_tiles} (failed: {len(failed_list)})",
                    end="",
                    flush=True,
                )

    print()

    if failed_list:
        with open(retry_json_path, "w", encoding="utf-8") as f:
            json.dump(failed_list, f, indent=4)


def retry_failures_parallel(json_path, max_workers=5) -> None:
    """
    Retry failed download tasks asynchronously from a JSON file.
    """
    if not os.path.exists(json_path):
        return

    with open(json_path, "r", encoding="utf-8") as f:
        tasks = json.load(f)

    class Tile:
        def __init__(self, x, y, z):
            self.x, self.y, self.z = x, y, z

    still_failed = []
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_task = {
            executor.submit(
                process_single_tile,
                Tile(t["x"], t["y"], t["z"]),
                t["url_template"],
                os.path.dirname(t["output_path"]),
            ): t
            for t in tasks
        }

        for future in as_completed(future_to_task):
            res = future.result()
            if res:
                still_failed.append(res)

    if still_failed:
        with open(json_path, "w", encoding="utf-8") as f:
            json.dump(still_failed, f, indent=4)
    else:
        os.remove(json_path)


def merge_tiles(input_dir: str, output_path: str) -> None:
    """
    Merge all GeoTIFF tiles in input_dir into a single GeoTIFF file.
    """
    search_path = os.path.join(input_dir, "*.tif")
    tif_files: list[str] = glob.glob(search_path)

    if not tif_files:
        print(f"Error: No .tif files found in {input_dir}")
        return

    print(f"Merging {len(tif_files)} tiles...")

    # Pass file paths directly to prevent opening too many file handles at once
    mosaic, out_trans = merge(tif_files, nodata=0)

    # Read spatial properties from the first dataset safely
    with rasterio.open(tif_files[0]) as first_src:
        out_meta = first_src.meta.copy()
        crs = first_src.crs

    out_meta.update(
        {
            "driver": "GTiff",
            "height": mosaic.shape[1],
            "width": mosaic.shape[2],
            "transform": out_trans,
            "crs": crs,
            "nodata": 0,
            "compress": "lzw",
            "bigtiff": "YES",
        }
    )

    with rasterio.open(output_path, "w", **out_meta) as dest:
        dest.write(mosaic)

    print(f"Successfully saved merged image to: {output_path}")
