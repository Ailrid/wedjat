use crate::errors::BackendError;
use crate::extractor::traits::{ExtractorBackend, FromBackend};
use crate::extractor::types::*;
use crate::fill_images_to_buffer;
use opencv::prelude::*;
use openvino::{Core, DeviceType, ElementType, Shape, Tensor};

pub struct OpenVinoBackend {
    _core: Core,
    model: openvino::CompiledModel,
    input_buffer: Vec<f32>,
}

impl ExtractorBackend for OpenVinoBackend {
    type Output = FeatureData;
    #[tracing::instrument(
        level = "info",
        skip(self, drone_img, sat_img),
        fields(backend = "vino")
    )]
    fn forward(&mut self, drone_img: &Mat, sat_img: &Mat) -> Result<Self::Output, BackendError> {
        // 准备输入数据
        fill_images_to_buffer!(drone_img, sat_img, self.input_buffer, IMAGE_SIZE);

        // 创建推理请求
        let mut infer_request = self.model.create_infer_request()?;

        // 创建输入 Tensor [2, 1, 256, 256] 并填充数据
        let shape = Shape::new(&[2, 1, IMAGE_SIZE as i64, IMAGE_SIZE as i64])?;

        let mut input_tensor = Tensor::new(ElementType::F32, &shape)?;

        // 使用 get_data_mut 填充数据
        input_tensor
            .get_data_mut::<f32>()?
            .copy_from_slice(&self.input_buffer);

        // 设置到请求中
        infer_request.set_input_tensor(&input_tensor)?;

        // 执行推理
        infer_request.infer()?;

        // 获取输出
        let kpts_tensor = infer_request.get_output_tensor_by_index(0)?;
        let matches_tensor = infer_request.get_output_tensor_by_index(1)?;
        let scores_tensor = infer_request.get_output_tensor_by_index(2)?;

        let kpts_view = kpts_tensor.get_data::<i64>()?;
        let matches_view = matches_tensor.get_data::<i64>()?;
        let sources_view = scores_tensor.get_data::<f32>()?;

        let stride = MODEL_POINTS * 2;
        // 第一组点：从 0 到 stride
        let first_kpts = KeyPoints::from_data(&kpts_view[0..stride], (MODEL_POINTS, 2));
        // 第二组点：从 stride 到 stride * 2
        let second_kpts = KeyPoints::from_data(&kpts_view[stride..stride * 2], (MODEL_POINTS, 2));

        let binding = scores_tensor.get_shape()?;
        let num_matches = binding.get_dimensions();

        let matches = Matches::from_data(matches_view, (num_matches[0] as usize, 3));
        let scores = Scores::from_data(sources_view, num_matches[0] as usize);

        Ok(FeatureData {
            first_kpts,
            second_kpts,
            matches,
            scores,
        })
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

        Ok(Self {
            _core: core,
            model,
            input_buffer: vec![0.0f32; 2 * IMAGE_SIZE * IMAGE_SIZE],
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use opencv::{core, core::Point, core::Vector, imgcodecs, imgproc};

//     #[test]
//     fn test_openvino_backend_with_visualization() -> Result<(), Box<dyn std::error::Error>> {
//         // 使用 "AUTO" 设备，它会自动选择 GPU (如果可用) 或 CPU
//         // let mut backend = OpenVinoBackend::new(
//         //     "assets/superpoint.xml",
//         //     "assets/superpoint.bin",
//         //     openvino::DeviceType::CPU,
//         // )?;
//         let mut backend = OpenVinoBackend::new(
//             "assets/superpoint_lightglue_pipeline.onnx",
//             "",
//             openvino::DeviceType::CPU,
//         )?;

//         // 2. 加载原始图像
//         let img1_raw = imgcodecs::imread("assets/0.jpg", imgcodecs::IMREAD_COLOR)?;
//         let img2_raw = imgcodecs::imread("assets/5.jpg", imgcodecs::IMREAD_COLOR)?;

//         if img1_raw.empty() || img2_raw.empty() {
//             panic!("测试图片未找到，请检查路径");
//         }

//         let (h1, w1) = (img1_raw.rows(), img1_raw.cols());
//         let (h2, w2) = (img2_raw.rows(), img2_raw.cols());

//         let scale1_x = w1 as f32 / IMAGE_SIZE as f32;
//         let scale1_y = h1 as f32 / IMAGE_SIZE as f32;
//         let scale2_x = w2 as f32 / IMAGE_SIZE as f32;
//         let scale2_y = h2 as f32 / IMAGE_SIZE as f32;

//         // 4. 执行推理
//         let features = backend.forward(&img1_raw, &img2_raw)?;

//         // 5. 可视化连线
//         let mut canvas = Mat::default();
//         let mut imgs_to_combine = Vector::<Mat>::new();
//         imgs_to_combine.push(img1_raw.clone());
//         imgs_to_combine.push(img2_raw.clone());
//         core::hconcat(&imgs_to_combine, &mut canvas)?;

//         // 假设你的模型输出 Matches 形状是 (N, 3)，KeyPoints 形状是 (MODEL_POINTS, 2)
//         let match_cols = 3; // matches 的列数
//         let kpt_cols = 2; // keypoints 的列数 (x, y)

//         // 1. 获取匹配点的总数
//         // 由于 data 是一维的，总行数 = 总元素数 / 列数
//         let num_matches = features.matches.data.len() / match_cols;

//         // 2. 遍历匹配点
//         for i in 0..num_matches {
//             // 索引映射：row i, col 1 是第一个点的索引；col 2 是第二个点的索引
//             let idx1 = features.matches.data[i * match_cols + 1] as usize;
//             let idx2 = features.matches.data[i * match_cols + 2] as usize;

//             // --- 处理第一张图的坐标 ---
//             // 坐标在 KeyPoints.data 中的布局通常是 [x0, y0, x1, y1, ...]
//             let x1_raw = features.first_kpts.data[idx1 * kpt_cols + 0] as f32;
//             let y1_raw = features.first_kpts.data[idx1 * kpt_cols + 1] as f32;

//             let x1 = (x1_raw * scale1_x) as i32;
//             let y1 = (y1_raw * scale1_y) as i32;

//             // --- 处理第二张图的坐标 ---
//             let x2_raw = features.second_kpts.data[idx2 * kpt_cols + 0] as f32;
//             let y2_raw = features.second_kpts.data[idx2 * kpt_cols + 1] as f32;

//             // 第二张图坐标需要加上第一张图的宽度偏移 (w1)
//             let x2 = (x2_raw * scale2_x) as i32 + w1;
//             let y2 = (y2_raw * scale2_y) as i32;

//             let pt1 = Point::new(x1, y1);
//             let pt2 = Point::new(x2, y2);

//             // --- 绘制连线 ---
//             imgproc::line(
//                 &mut canvas,
//                 pt1,
//                 pt2,
//                 core::Scalar::new(0.0, 255.0, 0.0, 0.0), // 绿色
//                 1,
//                 8,
//                 0,
//             )?;

//             // --- 绘制特征点圆圈 ---
//             imgproc::circle(
//                 &mut canvas,
//                 pt1,
//                 4,
//                 core::Scalar::new(0.0, 0.0, 255.0, 0.0),
//                 -1,
//                 8,
//                 0,
//             )?;
//             imgproc::circle(
//                 &mut canvas,
//                 pt2,
//                 4,
//                 core::Scalar::new(0.0, 0.0, 255.0, 0.0),
//                 -1,
//                 8,
//                 0,
//             )?;
//         }

//         // 6. 保存结果
//         let output_path = "ov_match_result.jpg";
//         imgcodecs::imwrite(output_path, &canvas, &core::Vector::new())?;
//         println!("OpenVINO 匹配结果已保存至: {}", output_path);

//         Ok(())
//     }
// }

// // /home/shiraha_yuki/miniforge3/lib/python3.12/site-packages/openvino/libs/libopenvino_c.so.2541
