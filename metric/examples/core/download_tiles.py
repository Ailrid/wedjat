"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from metric.core import (
    batch_download_parallel,
    merge_tiles,
    reproject_to_utm,
    resample_tif,
    retry_failures_parallel,
)

if __name__ == "__main__":
    input_folder = "./google_tiles"
    output_tif = "./merged_lv18.tif"
    utm_tif = "./merged_lv18_utm.tif"
    final_tif = "./merged_lv18_utm_0.5.tif"

    my_bbox = [
        114.594,  # West
        37.841,  # South
        114.707,  # East
        37.931,  # North
    ]

    target_url = "https://mt1.google.com/vt/lyrs=s&x={x}&y={y}&z={z}"

    #Download tiles in parallel
    batch_download_parallel(
        bbox=my_bbox,
        url_template=target_url,
        output_dir=input_folder,
        resolution=0.5,
        max_workers=8,
    )

    # Retry failed tile downloads if any
    retry_failures_parallel("failed_tiles.json")

    # Merge downloaded tiles into a single GeoTIFF
    merge_tiles(input_folder, output_tif)

    # Reproject GeoTIFF to the optimal UTM zone (UTM Zone 50N for Shijiazhuang)
    reproject_to_utm(output_tif, utm_tif)

    # Resample resolution to 0.5m x 0.5m with tiling enabled
    resample_tif(utm_tif, final_tif, target_res=(0.5, 0.5))
