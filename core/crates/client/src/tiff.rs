//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core

use opencv::core::{CV_8UC1, CV_8UC3, Mat, MatTraitManual, Vec3b};
use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

#[derive(Debug, thiserror::Error)]
pub enum TiffClientError {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("TIFF Error: {0}")]
    TIFFLoadError(#[from] tiff::TiffError),
    #[error("OpenCV Error: {0}")]
    OpenCVError(#[from] opencv::Error),
    #[error("Unsupported color format or bit depth")]
    UnsupportedFormat,
    #[error("Invalid crop dimensions")]
    InvalidCropDimension,
}

/// Tile TIFF Reader.
///
/// # Fields
///
/// - `decoder` (`Decoder<BufReader<File>>`) - Decoder.
/// - `file_path` (`PathBuf`) - Full path of tiff file.
/// - `width` (`u32`) - TIFF image width.
/// - `height` (`u32`) - TIFF image height.
/// - `tile_width` (`u32`) - TIFF tile width.
/// - `tile_height` (`u32`) - TIFF tile height.
/// - `samples_per_pixel` (`u32`) - Channels per pixel.
/// - `pixel_scale` (`Vec<f64>`) - x and y and z scale.
pub struct TiledTiffReader {
    pub decoder: Decoder<BufReader<File>>,
    pub file_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub samples_per_pixel: u32,
    pub pixel_scale: Vec<f64>,
}

impl TiledTiffReader {
    pub fn new<P>(file_path: P) -> Result<Self, TiffClientError>
    where
        P: AsRef<Path>,
    {
        let path_ref = file_path.as_ref();
        let file = File::open(path_ref)?;
        let io = BufReader::with_capacity(1024 * 1024, file);
        let mut decoder = Decoder::new(io)?;

        let samples_per_pixel = decoder.get_tag_u32(Tag::SamplesPerPixel)?;

        if samples_per_pixel != 1 && samples_per_pixel != 3 {
            return Err(TiffClientError::UnsupportedFormat);
        }

        let planar_config = decoder.get_tag_u32(Tag::PlanarConfiguration).unwrap_or(1);
        if planar_config == 2 {
            return Err(TiffClientError::UnsupportedFormat);
        }

        let width = decoder.get_tag_u32(Tag::ImageWidth)?;
        let height = decoder.get_tag_u32(Tag::ImageLength)?;
        let tile_width = decoder.get_tag_u32(Tag::TileWidth)?;
        let tile_height = decoder.get_tag_u32(Tag::TileLength)?;
        let pixel_scale = decoder.get_tag_f64_vec(Tag::ModelPixelScaleTag)?;

        Ok(Self {
            decoder,
            file_path: path_ref.to_path_buf(),
            width,
            height,
            tile_width,
            tile_height,
            samples_per_pixel,
            pixel_scale,
        })
    }

    pub fn tiles_across(&self) -> u32 {
        (self.width + self.tile_width - 1) / self.tile_width
    }

    /// Crops a region based on pixel coordinates.
    ///
    /// - `x`: Top-left horizontal coordinate in image space.
    /// - `y`: Top-left vertical coordinate in image space.
    /// - `crop_w`: Requested width in pixels.
    /// - `crop_h`: Requested height in pixels.
    pub fn crop_region(
        &mut self,
        x: u32,
        y: u32,
        crop_w: u32,
        crop_h: u32,
    ) -> Result<Mat, TiffClientError> {
        if crop_w == 0 || crop_h == 0 || x >= self.width || y >= self.height {
            return Err(TiffClientError::InvalidCropDimension);
        }

        let actual_w = crop_w.min(self.width - x);
        let actual_h = crop_h.min(self.height - y);

        let cv_type = if self.samples_per_pixel == 1 {
            CV_8UC1
        } else {
            CV_8UC3
        };

        let channels = self.samples_per_pixel as usize;
        let mut out_mat = unsafe { Mat::new_rows_cols(actual_h as i32, actual_w as i32, cv_type)? };

        let start_tile_x = x / self.tile_width;
        let end_tile_x = (x + actual_w - 1) / self.tile_width;
        let start_tile_y = y / self.tile_height;
        let end_tile_y = (y + actual_h - 1) / self.tile_height;

        let tiles_across = self.tiles_across();

        for ty in start_tile_y..=end_tile_y {
            for tx in start_tile_x..=end_tile_x {
                let chunk_index = ty * tiles_across + tx;

                let tile_data = match self.decoder.read_chunk(chunk_index)? {
                    DecodingResult::U8(data) => data,
                    _ => return Err(TiffClientError::UnsupportedFormat),
                };

                let tile_x0 = tx * self.tile_width;
                let tile_y0 = ty * self.tile_height;

                let cur_tile_w = self.tile_width.min(self.width - tile_x0);
                let cur_tile_h = self.tile_height.min(self.height - tile_y0);

                let inter_x0 = x.max(tile_x0);
                let inter_x1 = (x + actual_w).min(tile_x0 + cur_tile_w);
                let inter_y0 = y.max(tile_y0);
                let inter_y1 = (y + actual_h).min(tile_y0 + cur_tile_h);

                for curr_y in inter_y0..inter_y1 {
                    let tile_local_y = curr_y - tile_y0;
                    let tile_local_x_start = inter_x0 - tile_x0;

                    let src_start =
                        ((tile_local_y * cur_tile_w + tile_local_x_start) as usize) * channels;
                    let copy_len = ((inter_x1 - inter_x0) as usize) * channels;
                    let src_end = src_start + copy_len;

                    if src_end > tile_data.len() {
                        return Err(TiffClientError::UnsupportedFormat);
                    }

                    let mat_local_y = curr_y - y;
                    let mat_local_x_start = inter_x0 - x;
                    let dest_start = (mat_local_x_start as usize) * channels;
                    let dest_end = dest_start + copy_len;

                    if channels == 1 {
                        let row_slice = out_mat.at_row_mut::<u8>(mat_local_y as i32)?;
                        row_slice[dest_start..dest_end]
                            .copy_from_slice(&tile_data[src_start..src_end]);
                    } else {
                        let row_slice = out_mat.at_row_mut::<Vec3b>(mat_local_y as i32)?;
                        let byte_slice = unsafe {
                            std::slice::from_raw_parts_mut(
                                row_slice.as_mut_ptr() as *mut u8,
                                row_slice.len() * 3,
                            )
                        };
                        byte_slice[dest_start..dest_end]
                            .copy_from_slice(&tile_data[src_start..src_end]);
                    }
                }
            }
        }

        Ok(out_mat)
    }
}

/// The client responsible for capturing TIFF images
///
/// # Fields
///
/// - `tiff_folder` (`PathBuf`) - TIFF folder path.
/// - `tiff_reader` (`RefCell<Option<TiledTiffReader>>`) - TIFF file reader.
pub struct TiffClient {
    tiff_folder: PathBuf,
    pub tiff_reader: RefCell<Option<TiledTiffReader>>,
}

impl TiffClient {
    pub fn new(tiff_folder: impl AsRef<Path>) -> Self {
        Self {
            tiff_folder: tiff_folder.as_ref().to_path_buf(),
            tiff_reader: RefCell::new(None),
        }
    }

    /// Crops a region from the specified TIFF image using standard pixel coordinates.
    pub fn crop(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        tiff_name: impl Into<String>,
    ) -> Result<Mat, TiffClientError> {
        let tiff_name_str = tiff_name.into();
        let tiff_path = self.tiff_folder.join(&tiff_name_str);
        let mut reader_borrow = self.tiff_reader.borrow_mut();

        let need_reload = match reader_borrow.as_ref() {
            Some(reader) => reader.file_path != tiff_path,
            None => true,
        };

        if need_reload {
            let reader = TiledTiffReader::new(tiff_path)?;
            *reader_borrow = Some(reader);
        }

        let reader = reader_borrow.as_mut().unwrap();
        reader.crop_region(x, y, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use std::time::Instant;

    #[test]
    fn test_tiff_reader_and_crop_performance() {
        let mut tiff = TiledTiffReader::new("output_tiled.tif").expect("Failed to open TIFF file");

        println!("=== Tiled TIFF Metadata ===");
        println!("File: {:?}", tiff.file_path);
        println!("Dimensions: {}x{}", tiff.width, tiff.height);
        println!("Tile Dimensions: {}x{}", tiff.tile_width, tiff.tile_height);
        println!("Pixel Scale: {:?}", tiff.pixel_scale);

        let crop_size: u32 = 512;
        let num_iterations: u32 = 100;
        let mut rng = rand::rng();

        let max_x = tiff.width.saturating_sub(crop_size);
        let max_y = tiff.height.saturating_sub(crop_size);

        println!("\n=== Random Crop Benchmark ===");
        println!("Requested Crop Size: {}x{}", crop_size, crop_size);
        println!("Total Iterations: {}", num_iterations);

        let start_total = Instant::now();
        let mut successful_crops = 0;

        for i in 0..num_iterations {
            let x = if max_x > 0 {
                rng.random_range(0..=max_x)
            } else {
                0
            };
            let y = if max_y > 0 {
                rng.random_range(0..=max_y)
            } else {
                0
            };

            let crop_start = Instant::now();
            let result = tiff.crop_region(x, y, crop_size, crop_size);
            let _crop_duration = crop_start.elapsed();

            match result {
                Ok(_mat) => {
                    successful_crops += 1;
                }
                Err(e) => {
                    eprintln!("Crop failed at iteration {} ({}, {}): {:?}", i, x, y, e);
                }
            }
        }

        let total_duration = start_total.elapsed();
        let avg_duration = total_duration / num_iterations;
        let crops_per_sec = (num_iterations as f64) / total_duration.as_secs_f64();

        let bytes_per_pixel = tiff.samples_per_pixel as f64;
        let bytes_per_crop = (crop_size * crop_size) as f64 * bytes_per_pixel;
        let total_mb = (bytes_per_crop * num_iterations as f64) / (1024.0 * 1024.0);
        let mb_per_sec = total_mb / total_duration.as_secs_f64();

        println!("----------------------------------------");
        println!("Successful Crops: {}/{}", successful_crops, num_iterations);
        println!("Total Benchmark Time: {:?}", total_duration);
        println!("Average Crop Time: {:?}", avg_duration);
        println!("Processing Speed: {:.2} crops/sec", crops_per_sec);
        println!("Data Throughput: {:.2} MB/s", mb_per_sec);
        println!("========================================");

        assert_eq!(successful_crops, num_iterations);
    }
}
