use crate::error::BackendError;
use opencv::core::{Mat, MatTraitConstManual};
use rknn3::rknn::Rknn3;

pub struct FeatureExtractor {
    model: Rknn3,
}

impl FeatureExtractor {
    /// Create a new FeatureExtractor instance.
    pub fn new(
        device_id: impl Into<String>,
        model_path: impl Into<String>,
        weight_path: impl Into<String>,
    ) -> Result<Self, BackendError> {
        let model = Rknn3::new(device_id, model_path, weight_path)?;
        Ok(Self { model })
    }

    /// Perform inference on preprocessed Mat and return L2-normalized embedding vector.
    pub fn infer(&mut self, img: &Mat) -> Result<Vec<f32>, BackendError> {
        let img_bytes = img.data_bytes()?;

        self.model.run(&[img_bytes])?;

        let mut output_slice = self.model.get_output(0)?;

        // Perform L2 normalization on embedding vector for metric learning
        let norm_sq: f32 = output_slice.iter().map(|&x| x * x).sum();
        let norm = norm_sq.sqrt().max(1e-6);
        let inv_norm = 1.0 / norm;

        for val in output_slice.iter_mut() {
            *val *= inv_norm;
        }

        Ok(output_slice)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use opencv::imgcodecs;
//     use std::time::Instant;

//     #[test]
//     fn test_feature_extractor_fps() -> Result<(), Box<dyn std::error::Error>> {
//         let device_id = "0000:01:00.0";
//         let model_path = "./assets/model.rknn";
//         let weight_path = "./assets/model.weight";
//         let image_path = "assets/test1.png";
        
//         let src_mat = imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR)?;
//         if src_mat.empty() {
//             return Err("Failed to load image from path".into());
//         }

//         let mut extractor = FeatureExtractor::new(device_id, model_path, weight_path)?;

//         // Preprocess image without rigid dimension assumptions
//         let target_dim = Some((224, 224));
//         let input_mat = extractor.preprocess(&src_mat, target_dim)?;

//         // Warmup NPU context
//         for _ in 0..10 {
//             let _ = extractor.infer_preprocessed(&input_mat)?;
//         }

//         // Benchmark Pure NPU Execution Performance
//         let iterations = 1000;
//         let start_time = Instant::now();

//         for _ in 0..iterations {
//             let embedding = extractor.infer_preprocessed(&input_mat)?;
//             assert!(!embedding.is_empty());
//         }

//         let total_duration = start_time.elapsed();
//         let avg_time_ms = total_duration.as_secs_f64() * 1000.0 / (iterations as f64);
//         let fps = (iterations as f64) / total_duration.as_secs_f64();

//         println!("--- Feature Extractor Benchmark Results ---");
//         println!("Iterations: {}", iterations);
//         println!("Average Latency: {:.2} ms", avg_time_ms);
//         println!("Throughput: {:.2} FPS", fps);

//         Ok(())
//     }
// }
