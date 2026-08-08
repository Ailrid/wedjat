"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from pyproj import CRS
from pyproj.aoi import AreaOfInterest
from pyproj.database import query_utm_crs_info
import numpy as np
import rasterio
from rasterio.transform import from_origin
from rasterio.warp import (
    Resampling,
    calculate_default_transform,
    reproject,
    transform_bounds,
)


def resample_tif(
    input_path: str,
    output_path: str,
    target_res: tuple[float, float],
    tile_size: int = 256,
) -> None:
    """
    Resample an existing GeoTIFF file to a target resolution and save it as a tiled GeoTIFF.
    """
    with rasterio.open(input_path) as src:
        left, bottom, right, top = src.bounds
        new_res_x, new_res_y = target_res

        # Compute new dimensions based on target resolution
        new_width = int(round((right - left) / new_res_x))
        new_height = int(round((top - bottom) / new_res_y))

        new_trans = from_origin(left, top, new_res_x, new_res_y)
        nodata_val = src.nodata if src.nodata is not None else 0

        new_data = np.empty((src.count, new_height, new_width), dtype=src.dtypes[0])

        # Reproject band by band to fix range issue with rasterio.band
        for i in range(1, src.count + 1):
            reproject(
                source=rasterio.band(src, i),
                destination=new_data[i - 1],
                src_transform=src.transform,
                src_crs=src.crs,
                dst_transform=new_trans,
                dst_crs=src.crs,
                resampling=Resampling.bilinear,
                src_nodata=nodata_val,
                dst_nodata=nodata_val,
            )

        new_meta = src.meta.copy()
        new_meta.update(
            {
                "height": new_height,
                "width": new_width,
                "transform": new_trans,
                "compress": "lzw",
                "bigtiff": "YES",
                "nodata": nodata_val,
                "tiled": True,
                "blockxsize": tile_size,
                "blockysize": tile_size,
            }
        )

        with rasterio.open(output_path, "w", **new_meta) as dest:
            dest.write(new_data)


def reproject_to_utm(input_path: str, output_path: str, tile_size: int = 256) -> None:
    """
    Reproject a GeoTIFF file to the optimal UTM zone and save as a tiled GeoTIFF.
    """
    with rasterio.open(input_path) as src:
        # Calculate bounding box center in WGS84
        bounds_wgs84 = transform_bounds(src.crs, "EPSG:4326", *src.bounds)
        lon = (bounds_wgs84[0] + bounds_wgs84[2]) / 2
        lat = (bounds_wgs84[1] + bounds_wgs84[3]) / 2

        # Query appropriate UTM EPSG code
        utm_crs_list = query_utm_crs_info(
            datum_name="WGS 84", area_of_interest=AreaOfInterest(lon, lat, lon, lat)
        )
        dst_crs = CRS.from_user_input(f"EPSG:{utm_crs_list[0].code}")

        # Compute transform parameters for the destination CRS
        transform, width, height = calculate_default_transform(
            src.crs, dst_crs, src.width, src.height, *src.bounds
        )

        nodata_val = src.nodata if src.nodata is not None else 0

        # Set up tile metadata
        new_meta = src.meta.copy()
        new_meta.update(
            {
                "crs": dst_crs,
                "transform": transform,
                "width": width,
                "height": height,
                "compress": "lzw",
                "nodata": nodata_val,
                "bigtiff": "YES",
                "tiled": True,
                "blockxsize": tile_size,
                "blockysize": tile_size,
            }
        )

        # Write data and perform reproject directly on dst bands
        with rasterio.open(output_path, "w", **new_meta) as dst:
            for i in range(1, src.count + 1):
                reproject(
                    source=rasterio.band(src, i),
                    destination=rasterio.band(dst, i),
                    src_transform=src.transform,
                    src_crs=src.crs,
                    dst_transform=transform,
                    dst_crs=dst_crs,
                    resampling=Resampling.bilinear,
                    src_nodata=nodata_val,
                    dst_nodata=nodata_val,
                )
