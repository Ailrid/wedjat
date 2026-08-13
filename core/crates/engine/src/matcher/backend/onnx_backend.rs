use crate::errors::BackendError;
use crate::fill_images_to_buffer;
use crate::matcher::traits::MatcherBackend;
use opencv::prelude::*;
use ort::{
    ep,
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};

pub struct OnnxBackend {
    session: Session,
}

impl MatcherBackend for OnnxBackend {
    #[tracing::instrument(level = "info", skip(self, drone_img), fields(backend = "onnx"), err)]
    fn forword(&mut self, drone_img: &Mat) -> Result<Vec<f32>, BackendError> {
        //要转换一下格式
        // 准备输入数据
        let mut input_buffer: Vec<f32> = Vec::new();

        fill_images_to_buffer!(drone_img, input_buffer);

        // 运行模型
        let output = self.session.run(ort::inputs![Tensor::from_array((
            [1, drone_img.channels(), drone_img.rows(), drone_img.cols()],
            input_buffer
        ))?])?;

        let img_vector = output[0].try_extract_tensor::<f32>()?;

        //直接返回结果
        Ok(img_vector.1.to_vec())
    }
}

impl OnnxBackend {
    #[tracing::instrument(level = "info")]
    pub fn new(
        level: GraphOptimizationLevel,
        intra_threads: usize,
        onnx_path: &str,
    ) -> Result<Self, BackendError> {
        let session = Session::builder()?
            .with_execution_providers([
                // Prefer TensorRT over CUDA.
                ep::TensorRT::default().build(),
                ep::CUDA::default().build(),
                // Use DirectML on Windows if NVIDIA EPs are not available
                ep::DirectML::default().build(),
                // Or use ANE on Apple platforms
                ep::CoreML::default().build(),
            ])?
            .with_optimization_level(level)?
            .with_intra_threads(intra_threads)?
            .commit_from_file(onnx_path)?;
        Ok(Self { session })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::{
        core::{self, Mat},
        imgcodecs, imgproc,
    };
    use ort::session::builder::GraphOptimizationLevel;
    use std::path::Path;

    #[test]
    fn test_onnx_backend_simple_inference() -> Result<(), Box<dyn std::error::Error>> {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
        // let _ = tracing_subscriber::fmt()
        //     .with_env_filter("debug") // 故意设为 debug 级别来看 instrument 的输出
        //     .with_test_writer() // 关键：确保日志能被 cargo test 捕获
        //     .try_init();
        // // 1. 路径准备
        let model_path = "assets/twinnet_inference.onnx";
        let img_path = "assets/0.jpg";
        const IMAGE_SIZE: usize = 256;

        if !Path::new(model_path).exists() || !Path::new(img_path).exists() {
            println!("测试跳过：找不到模型或图像文件。");
            return Ok(());
        }

        // 2. 初始化后端
        let mut backend = OnnxBackend::new(GraphOptimizationLevel::Level3, 4, model_path)?;

        // 3. 加载原始图像 (OpenCV 默认 BGR)
        let img_raw = imgcodecs::imread(img_path, imgcodecs::IMREAD_COLOR)?;
        if img_raw.empty() {
            panic!("无法读取图像: {}", img_path);
        }
        let mut resized = Mat::default();
        imgproc::resize(
            &img_raw,
            &mut resized,
            core::Size::new(IMAGE_SIZE as i32, IMAGE_SIZE as i32),
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        )?;

        // 4. 执行推理 (内部会进行 256x256 Resize 和 ImageNet 归一化宏调用)
        let start_time = std::time::Instant::now();
        tracing::info!("Starting inference...");
        let vector = backend.forword(&resized)?;
        let duration = start_time.elapsed();

        // 5. 打印统计信息 (用于和 Python 对齐)
        let v_len = vector.len();
        let mean: f32 = vector.iter().sum::<f32>() / v_len as f32;
        let max = vector.iter().fold(f32::MIN, |a, &b| a.max(b));
        let min = vector.iter().fold(f32::MAX, |a, &b| a.min(b));

        println!("\n================ ONNX 推理结果统计 ================");
        println!("处理耗时: {:?}", duration);
        println!("向量长度: {}", v_len);
        println!("数值分布:");
        println!("  - 最小值: {:.6}", min);
        println!("  - 最大值: {:.6}", max);
        println!("  - 平均值: {:.6}", mean);

        if v_len >= 8 {
            println!("前 8 位特征值: {:.6?}", &vector[..8]);
        }
        println!("==================================================\n");

        // 6. 基础校验
        assert!(v_len > 0, "特征向量不应为空");
        // 如果均值和极值都在正常范围（通常 -10 到 10 之间），说明预处理没出大问题
        assert!(max.abs() < 100.0, "特征值异常大，请检查归一化逻辑");

        Ok(())
    }
}
