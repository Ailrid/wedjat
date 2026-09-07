use crate::error::BackendError;
use opencv::{
    core::{self, CV_8UC1, Mat, MatTraitConst, MatTraitConstManual, Point, Size},
    imgproc,
};
use rayon::prelude::*;
use rknn3::rknn::Rknn3;

pub struct SuperPointOutput {
    pub keypoints: Vec<[f32; 2]>,
    pub descriptors: Vec<Vec<f32>>,
}

pub struct SuperPoint {
    model: Rknn3,
    num_keypoints: usize,
    nms_radius: usize,
    remove_borders: usize,
}

impl SuperPoint {
    pub fn new(
        device_id: impl Into<String>,
        model_path: impl Into<String>,
        weight_path: impl Into<String>,
        num_keypoints: usize,
        nms_radius: usize,
        remove_borders: usize,
    ) -> Result<Self, BackendError> {
        let model = Rknn3::new(device_id, model_path, weight_path)?;

        Ok(Self {
            model,
            num_keypoints,
            nms_radius,
            remove_borders,
        })
    }

    pub fn infer(&mut self, img: &Mat) -> Result<SuperPointOutput, BackendError> {
        let height = img.rows() as usize;
        let width = img.cols() as usize;

        let gray_img = if img.typ() == CV_8UC1 {
            img.clone()
        } else {
            let mut gray = Mat::default();
            imgproc::cvt_color(img, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
            gray
        };

        let img_bytes = gray_img.data_bytes()?;
        self.model.run(&[img_bytes])?;

        let scores = self.model.get_output(0)?;
        let descriptors = self.model.get_output(1)?;

        self.post_process_superpoint(&scores, &descriptors, height, width)
    }
    pub fn post_process_superpoint(
        &self,
        scores: &[f32],
        descriptors: &[f32],
        height: usize,
        width: usize,
    ) -> Result<SuperPointOutput, BackendError> {
        let score_mat = Mat::new_rows_cols_with_data(height as i32, width as i32, scores)?;

        let ksize = (self.nms_radius * 2 + 1) as i32;
        let kernel = imgproc::get_structuring_element(
            imgproc::MORPH_RECT,
            Size::new(ksize, ksize),
            Point::new(-1, -1),
        )?;

        let mut max_map = Mat::default();
        imgproc::dilate(
            &score_mat,
            &mut max_map,
            &kernel,
            Point::new(-1, -1),
            1,
            core::BORDER_CONSTANT,
            core::Scalar::default(),
        )?;

        let score_slice = score_mat.data_typed::<f32>()?;
        let max_slice = max_map.data_typed::<f32>()?;

        let border = self.remove_borders;
        let y_end = height.saturating_sub(border);
        let x_end = width.saturating_sub(border);

        let valid_rows = y_end.saturating_sub(border);
        let valid_cols = x_end.saturating_sub(border);
        let max_possible_points = valid_rows * valid_cols;
        let target_k = self.num_keypoints.min(max_possible_points);

        if target_k == 0 {
            return Ok(SuperPointOutput {
                keypoints: Vec::new(),
                descriptors: Vec::new(),
            });
        }

        // Separate candidates into NMS local maxima and non-NMS points
        let mut nms_candidates: Vec<(f32, usize, usize)> = Vec::with_capacity(4096);
        let mut non_nms_candidates: Vec<(f32, usize, usize)> = Vec::new();

        for y in border..y_end {
            let row_offset = y * width;
            for x in border..x_end {
                let idx = row_offset + x;
                let val = score_slice[idx];

                if val == max_slice[idx] {
                    nms_candidates.push((val, x, y));
                } else {
                    non_nms_candidates.push((val, x, y));
                }
            }
        }

        // Select exact Top-K keypoints, backfilling from non-NMS if NMS points are insufficient
        let valid_candidates = if nms_candidates.len() >= target_k {
            if nms_candidates.len() > target_k {
                nms_candidates.select_nth_unstable_by(target_k - 1, |a, b| {
                    b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
                });
                nms_candidates.truncate(target_k);
            }
            nms_candidates
        } else {
            let needed = target_k - nms_candidates.len();
            if non_nms_candidates.len() > needed {
                non_nms_candidates.select_nth_unstable_by(needed - 1, |a, b| {
                    b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
                });
                non_nms_candidates.truncate(needed);
            }
            nms_candidates.extend(non_nms_candidates);
            nms_candidates
        };

        let desc_h = height / 8;
        let desc_w = width / 8;
        let desc_channels = 256;
        let map_area = desc_h * desc_w;

        let keypoints: Vec<[f32; 2]> = valid_candidates
            .iter()
            .map(|&(_, x, y)| [x as f32, y as f32])
            .collect();

        // Parallel descriptor sampling using Rayon
        let descriptors_vec: Vec<Vec<f32>> = valid_candidates
            .par_iter()
            .map(|&(_, x, y)| {
                let gx = (x as f32 / (width - 1) as f32) * (desc_w - 1) as f32;
                let gy = (y as f32 / (height - 1) as f32) * (desc_h - 1) as f32;

                let x0 = gx.floor() as usize;
                let y0 = gy.floor() as usize;
                let x1 = (x0 + 1).min(desc_w - 1);
                let y1 = (y0 + 1).min(desc_h - 1);

                let dx = gx - x0 as f32;
                let dy = gy - y0 as f32;

                let w00 = (1.0 - dx) * (1.0 - dy);
                let w01 = dx * (1.0 - dy);
                let w10 = (1.0 - dx) * dy;
                let w11 = dx * dy;

                let idx00 = y0 * desc_w + x0;
                let idx01 = y0 * desc_w + x1;
                let idx10 = y1 * desc_w + x0;
                let idx11 = y1 * desc_w + x1;

                let mut desc_vec = vec![0.0f32; desc_channels];
                let mut norm_sq = 0.0f32;

                // Cache-friendly sampling across 256 channels
                for c in 0..desc_channels {
                    let channel_offset = c * map_area;

                    let v00 = descriptors[channel_offset + idx00];
                    let v01 = descriptors[channel_offset + idx01];
                    let v10 = descriptors[channel_offset + idx10];
                    let v11 = descriptors[channel_offset + idx11];

                    let val = w00 * v00 + w01 * v01 + w10 * v10 + w11 * v11;
                    desc_vec[c] = val;
                    norm_sq += val * val;
                }

                let norm = norm_sq.sqrt().max(1e-6);
                let inv_norm = 1.0 / norm;
                for val in desc_vec.iter_mut() {
                    *val *= inv_norm;
                }

                desc_vec
            })
            .collect();

        Ok(SuperPointOutput {
            keypoints,
            descriptors: descriptors_vec,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::{
        core::{self, Mat, Point, Scalar, Vector},
        imgcodecs, imgproc,
        prelude::*,
    };
    use std::time::Instant;

    #[test]
    fn test_superpoint_pipeline_and_fps() -> Result<(), Box<dyn std::error::Error>> {
        // Load image using OpenCV
        let image_path = "assets/test1.png";
        let src_mat = imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR)?;
        if src_mat.empty() {
            return Err("Failed to load image from path".into());
        }

        // Initialize SuperPoint engine
        let device_id = "0000:01:00.0";
        let model_path = "./assets/superpoint.rknn";
        let weight_path = "./assets/superpoint.weight";
        let num_keypoints = 256;
        let nms_radius = 4;
        let remove_borders = 4;

        let mut superpoint = SuperPoint::new(
            device_id,
            model_path,
            weight_path,
            num_keypoints,
            nms_radius,
            remove_borders,
        )?;

        // Ensure Mat memory continuity when extracting bytes
        let mut continuous_mat = Mat::default();
        if src_mat.is_continuous() {
            continuous_mat = src_mat.clone();
        } else {
            src_mat.copy_to(&mut continuous_mat)?;
        }

        // Warmup NPU context
        for _ in 0..5 {
            let _ = superpoint.infer(&continuous_mat)?;
        }

        // Run single inference for visualization and validation
        let output = superpoint.infer(&continuous_mat)?;
        println!("Extracted {} keypoints.", output.keypoints.len());

        // Draw keypoints onto output color matrix
        let mut display_mat = continuous_mat.clone();
        for kp in &output.keypoints {
            let center = Point::new(kp[0] as i32, kp[1] as i32);
            imgproc::circle(
                &mut display_mat,
                center,
                2,
                Scalar::new(0.0, 255.0, 0.0, 0.0), // Green point
                -1,
                imgproc::LINE_AA,
                0,
            )?;
        }

        // Save visualization image
        let output_path = "superpoint_result.jpg";
        imgcodecs::imwrite(output_path, &display_mat, &Vector::new())?;
        println!("Saved visual result to {}", output_path);

        // Benchmark FPS performance
        let iterations = 100;
        let start_time = Instant::now();

        for _ in 0..iterations {
            let _ = superpoint.infer(&continuous_mat)?;
        }

        let total_duration = start_time.elapsed();
        let avg_time_ms = total_duration.as_secs_f64() * 1000.0 / (iterations as f64);
        let fps = (iterations as f64) / total_duration.as_secs_f64();

        println!("--- Benchmark Results ---");
        println!("Iterations: {}", iterations);
        println!("Average Latency: {:.2} ms", avg_time_ms);
        println!("Throughput: {:.2} FPS", fps);

        Ok(())
    }
}
