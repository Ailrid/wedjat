use crate::error::BackendError;
use crate::superpoint::SuperPointOutput;
use half::f16;
use rayon::prelude::*;
use rknn3::rknn::Rknn3;

pub struct LightGlue {
    model: Rknn3,
    match_threshold: f32,
}
pub struct LightGlueOutput {
    pub matches: Vec<(usize, usize)>,
}

impl LightGlue {
    /// Create a new LightGlue instance.
    pub fn new(
        device_id: impl Into<String>,
        model_path: impl Into<String>,
        weight_path: impl Into<String>,
        match_threshold: f32,
    ) -> Result<Self, BackendError> {
        let model = Rknn3::new(device_id, model_path, weight_path)?;
        Ok(Self {
            model,
            match_threshold,
        })
    }
    pub fn infer(
        &mut self,
        sp0: &SuperPointOutput,
        sp1: &SuperPointOutput,
    ) -> Result<LightGlueOutput, BackendError> {
        let num_kpts0 = sp0.keypoints.len();
        let num_kpts1 = sp1.keypoints.len();
        let desc_dim0 = sp0.descriptors.first().map_or(0, |d| d.len());
        let desc_dim1 = sp1.descriptors.first().map_or(0, |d| d.len());

        // Keypoints for image 0: Shape (1, N0, 2)
        let mut kpts0_flat: Vec<f16> = Vec::with_capacity(num_kpts0 * 2);
        for kp in &sp0.keypoints {
            kpts0_flat.push(f16::from_f32(kp[0]));
            kpts0_flat.push(f16::from_f32(kp[1]));
        }

        // Descriptors for image 0: Shape (1, N0, 256)
        let mut descs0_flat: Vec<f16> = Vec::with_capacity(num_kpts0 * desc_dim0);
        for desc in &sp0.descriptors {
            descs0_flat.extend(desc.iter().map(|&v| f16::from_f32(v)));
        }

        // Keypoints for image 1: Shape (1, N1, 2)
        let mut kpts1_flat: Vec<f16> = Vec::with_capacity(num_kpts1 * 2);
        for kp in &sp1.keypoints {
            kpts1_flat.push(f16::from_f32(kp[0]));
            kpts1_flat.push(f16::from_f32(kp[1]));
        }

        // Descriptors for image 1: Shape (1, N1, 256)
        let mut descs1_flat: Vec<f16> = Vec::with_capacity(num_kpts1 * desc_dim1);
        for desc in &sp1.descriptors {
            descs1_flat.extend(desc.iter().map(|&v| f16::from_f32(v)));
        }

        // Pass 4 separate inputs into model runner matching ONNX input order
        self.model
            .run(&[&kpts0_flat, &kpts1_flat, &descs0_flat, &descs1_flat])?;

        // Retrieve raw assignment score matrix
        let output_scores = self.model.get_output(0)?;

        // Execute parallel post-processing
        let matches = self.decode_mnn_matches(&output_scores, num_kpts0);

        Ok(LightGlueOutput { matches })
    }

    /// Perform parallel Mutual Nearest Neighbor (MNN) decoding on matching matrix using Rayon.
    fn decode_mnn_matches(&self, scores: &[f32], num_kpts: usize) -> Vec<(usize, usize)> {
        // Parallel row max search: img0 -> img1
        let best_0_to_1: Vec<(usize, f32)> = (0..num_kpts)
            .into_par_iter()
            .map(|i| {
                let row_offset = i * num_kpts;
                let row_slice = &scores[row_offset..row_offset + num_kpts];

                let mut max_idx = 0;
                let mut max_val = f32::NEG_INFINITY;

                for (j, &val) in row_slice.iter().enumerate() {
                    if val > max_val {
                        max_val = val;
                        max_idx = j;
                    }
                }
                (max_idx, max_val)
            })
            .collect();

        // Parallel column max search: img1 -> img0
        let best_1_to_0: Vec<usize> = (0..num_kpts)
            .into_par_iter()
            .map(|j| {
                let mut max_idx = 0;
                let mut max_val = f32::NEG_INFINITY;

                for i in 0..num_kpts {
                    let val = scores[i * num_kpts + j];
                    if val > max_val {
                        max_val = val;
                        max_idx = i;
                    }
                }
                max_idx
            })
            .collect();

        // MNN mutual agreement filter
        let threshold = self.match_threshold;
        (0..num_kpts)
            .into_par_iter()
            .filter_map(|i| {
                let (j, score) = best_0_to_1[i];
                if score > threshold && best_1_to_0[j] == i {
                    Some((i, j))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::lightglue::LightGlue;
    use crate::superpoint::SuperPoint;
    use opencv::{
        core::{self, CV_8UC3, Mat, Point, Scalar, Vector},
        imgcodecs, imgproc,
        prelude::*,
    };
    use std::time::Instant;

    #[test]
    fn test_superpoint_lightglue_pipeline() -> Result<(), Box<dyn std::error::Error>> {
        // Configuration parameters
        let img_path0 = "assets/test1.png";
        let img_path1 = "assets/test2.png";
        let output_path = "lightglue_matches.jpg";

        let device_id = "0000:01:00.0";
        let sp_model_path = "./assets/superpoint.rknn";
        let sp_weight_path = "./assets/superpoint.weight";
        let lg_model_path = "./assets/lightglue.rknn";
        let lg_weight_path = "./assets/lightglue.weight";

        let num_keypoints = 256;
        let nms_radius = 4;
        let remove_borders = 4;
        let match_threshold = 0.0;

        let warmup_iterations = 5;
        let bench_iterations = 100;

        // Load images
        let mat0 = imgcodecs::imread(img_path0, imgcodecs::IMREAD_COLOR)?;
        let mat1 = imgcodecs::imread(img_path1, imgcodecs::IMREAD_COLOR)?;

        if mat0.empty() || mat1.empty() {
            return Err("Failed to load test images".into());
        }

        // Ensure Mat memory buffer continuity for RKNN inference
        let mut cont_mat0 = Mat::default();
        let mut cont_mat1 = Mat::default();

        if mat0.is_continuous() {
            cont_mat0 = mat0.clone();
        } else {
            mat0.copy_to(&mut cont_mat0)?;
        }

        if mat1.is_continuous() {
            cont_mat1 = mat1.clone();
        } else {
            mat1.copy_to(&mut cont_mat1)?;
        }

        // Initialize SuperPoint and LightGlue backends
        let mut superpoint = SuperPoint::new(
            device_id,
            sp_model_path,
            sp_weight_path,
            num_keypoints,
            nms_radius,
            remove_borders,
        )?;

        let mut lightglue =
            LightGlue::new(device_id, lg_model_path, lg_weight_path, match_threshold)?;

        // Extract keypoints and descriptors using SuperPoint
        let sp_out0 = superpoint.infer(&cont_mat0)?;
        let sp_out1 = superpoint.infer(&cont_mat1)?;

        println!(
            "SuperPoint extracted {} and {} keypoints.",
            sp_out0.keypoints.len(),
            sp_out1.keypoints.len()
        );

        // Warmup LightGlue NPU context
        for _ in 0..warmup_iterations {
            let _ = lightglue.infer(&sp_out0, &sp_out1)?;
        }

        // Run single inference for visual output
        let lg_out = lightglue.infer(&sp_out0, &sp_out1)?;
        println!("Found {} valid matches.", lg_out.matches.len());

        // Benchmark LightGlue performance
        let start_time = Instant::now();
        for _ in 0..bench_iterations {
            let _ = lightglue.infer(&sp_out0, &sp_out1)?;
        }
        let total_duration = start_time.elapsed();

        let avg_time_ms = total_duration.as_secs_f64() * 1000.0 / (bench_iterations as f64);
        let fps = (bench_iterations as f64) / total_duration.as_secs_f64();

        println!("--- LightGlue Benchmark Results ---");
        println!("Iterations: {}", bench_iterations);
        println!("Average Latency: {:.2} ms", avg_time_ms);
        println!("Throughput: {:.2} FPS", fps);

        // Canvas preparation for drawing matches side by side
        let h0 = cont_mat0.rows();
        let w0 = cont_mat0.cols();
        let h1 = cont_mat1.rows();
        let w1 = cont_mat1.cols();

        let canvas_height = h0.max(h1);
        let canvas_width = w0 + w1;

        let mut canvas = Mat::new_rows_cols_with_default(
            canvas_height,
            canvas_width,
            CV_8UC3,
            Scalar::all(0.0),
        )?;

        let mut roi_left = Mat::roi_mut(&mut canvas, core::Rect::new(0, 0, w0, h0))?;
        cont_mat0.copy_to(&mut roi_left)?;

        let mut roi_right = Mat::roi_mut(&mut canvas, core::Rect::new(w0, 0, w1, h1))?;
        cont_mat1.copy_to(&mut roi_right)?;

        // Draw matching points and connecting lines
        for (idx0, idx1) in &lg_out.matches {
            let kp0 = sp_out0.keypoints[*idx0];
            let kp1 = sp_out1.keypoints[*idx1];

            let pt0 = Point::new(kp0[0] as i32, kp0[1] as i32);
            let pt1 = Point::new((kp1[0] as i32) + w0, kp1[1] as i32);

            imgproc::line(
                &mut canvas,
                pt0,
                pt1,
                Scalar::new(0.0, 255.0, 0.0, 0.0), // Green lines
                1,
                imgproc::LINE_AA,
                0,
            )?;

            imgproc::circle(
                &mut canvas,
                pt0,
                3,
                Scalar::new(0.0, 0.0, 255.0, 0.0), // Red points on image 0
                -1,
                imgproc::LINE_AA,
                0,
            )?;

            imgproc::circle(
                &mut canvas,
                pt1,
                3,
                Scalar::new(255.0, 0.0, 0.0, 0.0), // Blue points on image 1
                -1,
                imgproc::LINE_AA,
                0,
            )?;
        }

        // Save result
        imgcodecs::imwrite(output_path, &canvas, &Vector::new())?;
        println!("Saved match visualization to {}", output_path);

        Ok(())
    }
}
