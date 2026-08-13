use crate::errors::BackendError;
use crate::fill_images_to_buffer;
use crate::matcher::traits::MatcherBackend;
use opencv::prelude::*;
use openvino::{Core, DeviceType, ElementType, Shape, Tensor};

pub struct OpenVinoBackend {
    _core: Core,
    model: openvino::CompiledModel,
}

impl MatcherBackend for OpenVinoBackend {
    #[tracing::instrument(
        level = "info",
        skip(self, drone_img),
        fields(backend = "vino")
        err
    )]
    fn forword(&mut self, drone_img: &Mat) -> Result<Vec<f32>, BackendError> {
        // 准备输入数据
        let mut input_buffer: Vec<f32> = Vec::new();
        fill_images_to_buffer!(drone_img, input_buffer);
        // 创建推理请求
        let mut infer_request = self.model.create_infer_request()?;
        // 创建输入 Tensor [2, 1, 256, 256] 并填充数据
        let shape = Shape::new(&[
            1,
            drone_img.channels() as i64,
            drone_img.rows() as i64,
            drone_img.cols() as i64,
        ])?;

        let mut input_tensor = Tensor::new(ElementType::F32, &shape)?;
        // 使用 get_data_mut 填充数据
        input_tensor
            .get_data_mut::<f32>()?
            .copy_from_slice(&input_buffer);

        // 设置到请求中
        infer_request.set_input_tensor(&input_tensor)?;
        // 执行推理
        infer_request.infer()?;
        // 获取输出
        let img_vector = infer_request.get_output_tensor_by_index(0)?;

        Ok(img_vector.get_data::<f32>()?.to_vec())
    }
}

impl OpenVinoBackend {
    #[tracing::instrument(level = "info")]
    pub fn new(
        xml_path: &str,
        bin_path: &str,
        device: DeviceType<'_>,
    ) -> Result<Self, BackendError> {
        // 初始化 OpenVINO 核心
        let mut core = Core::new()?;
        // 读取模型
        let model = core.read_model_from_file(xml_path, bin_path)?;
        // 编译模型
        let model = core.compile_model(&model, device)?;
        Ok(Self { _core: core, model })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::{
        core::{self, Mat},
        imgcodecs, imgproc,
    };

    use std::path::Path;

    #[test]
    fn test_onnx_backend_simple_inference() -> Result<(), Box<dyn std::error::Error>> {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
        println!(
            "Current LD_LIBRARY_PATH: {:?}",
            std::env::var("LD_LIBRARY_PATH")
        );
        // 1. 路径准备
        let model_path = "assets/twinnet_inference.onnx";
        let img_path = "assets/0.jpg";
        const IMAGE_SIZE: usize = 256;

        if !Path::new(model_path).exists() || !Path::new(img_path).exists() {
            println!("测试跳过：找不到模型或图像文件。");
            return Ok(());
        }

        // 2. 初始化后端
        let mut backend = OpenVinoBackend::new(model_path, "", openvino::DeviceType::CPU)?;

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
        let vector = backend.forword(&resized)?;
        let duration = start_time.elapsed();

        // 5. 打印统计信息 (用于和 Python 对齐)
        let v_len = vector.len();
        let mean: f32 = vector.iter().sum::<f32>() / v_len as f32;
        let max = vector.iter().fold(f32::MIN, |a, &b| a.max(b));
        let min = vector.iter().fold(f32::MAX, |a, &b| a.min(b));

        println!("\n================ VINO 推理结果统计 ================");
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
