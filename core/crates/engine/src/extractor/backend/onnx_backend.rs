use crate::errors::BackendError;
use crate::extractor::traits::{ExtractorBackend, FromBackend};
use crate::extractor::types::*;
use crate::fill_images_to_buffer;
use opencv::prelude::*;
use ort::{
    ep,
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};

pub struct OnnxBackend {
    session: Session,
    input_buffer: Vec<f32>,
}

impl ExtractorBackend for OnnxBackend {
    type Output = FeatureData;
    #[tracing::instrument(
        level = "info",
        skip(self, drone_img, sat_img),
        fields(backend = "onnx")
    )]
    fn forward(&mut self, drone_img: &Mat, sat_img: &Mat) -> Result<Self::Output, BackendError> {
        //要转换一下格式
        // 准备输入数据
        fill_images_to_buffer!(drone_img, sat_img, self.input_buffer, IMAGE_SIZE);

        // 运行模型
        let output = self.session.run(ort::inputs![Tensor::from_array((
            [2, 1, IMAGE_SIZE, IMAGE_SIZE],
            self.input_buffer.clone()
        ))?])?;

        let kpts_view = output[0].try_extract_tensor::<i64>()?;
        let matches_view = output[1].try_extract_tensor::<i64>()?;
        let sources_view = output[2].try_extract_tensor::<f32>()?;

        let stride = MODEL_POINTS * 2;
        // 第一组点：从 0 到 stride
        let first_kpts = KeyPoints::from_data(&kpts_view.1[0..stride], (MODEL_POINTS, 2));

        // 第二组点：从 stride 到 stride * 2
        let second_kpts = KeyPoints::from_data(&kpts_view.1[stride..stride * 2], (MODEL_POINTS, 2));

        let matches = Matches::from_data(matches_view.1, (matches_view.0[0] as usize, 3));
        let scores = Scores::from_data(sources_view.1, sources_view.0.len());

        Ok(FeatureData {
            first_kpts,
            second_kpts,
            matches,
            scores,
        })
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
                ep::TensorRT::default().build(),
                ep::CUDA::default().build(),
                ep::DirectML::default().build(),
                ep::CoreML::default().build(),
            ])?
            .with_optimization_level(level)?
            .with_intra_threads(intra_threads)?
            .commit_from_file(onnx_path)?;
        Ok(Self {
            session,
            input_buffer: vec![0.0f32; 2 * IMAGE_SIZE * IMAGE_SIZE],
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use opencv::{
//         core::{self, Point, Vector},
//         imgcodecs, imgproc,
//     };
//     use std::path::Path;

//     #[test]
//     fn test_onnx_backend_with_visualization() -> Result<(), Box<dyn std::error::Error>> {
//         // 1. 初始化后端 (请确保文件路径正确)
//         let model_path = "assets/superpoint_lightglue_pipeline.onnx";
//         if !Path::new(model_path).exists() {
//             println!("跳过测试：找不到模型文件 {}", model_path);
//             return Ok(());
//         }

//         let mut backend = OnnxBackend::new(GraphOptimizationLevel::Level3, 4, model_path)?;

//         // 2. 加载原始图像
//         let img1_raw = imgcodecs::imread("assets/0.jpg", imgcodecs::IMREAD_COLOR)?;
//         let img2_raw = imgcodecs::imread("assets/5.jpg", imgcodecs::IMREAD_COLOR)?;

//         if img1_raw.empty() || img2_raw.empty() {
//             panic!("测试图片未找到，请检查路径");
//         }

//         // 1. 获取原图尺寸并计算缩放比例
//         let (h1, w1) = (img1_raw.rows(), img1_raw.cols());
//         let (h2, w2) = (img2_raw.rows(), img2_raw.cols());

//         // 计算坐标缩放倍数 (原图宽 / 模型输入宽)
//         let scale1_x = w1 as f32 / IMAGE_SIZE as f32;
//         let scale1_y = h1 as f32 / IMAGE_SIZE as f32;
//         let scale2_x = w2 as f32 / IMAGE_SIZE as f32;
//         let scale2_y = h2 as f32 / IMAGE_SIZE as f32;

//         // 3. 执行推理
//         let features = backend.forward(&img1_raw, &img2_raw)?;

//         // 4. 构建可视化画布：直接使用原图拼接，而不是使用 0-1 的灰度推理图
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
//                 2,
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
//         let output_path = "match_result_full_res.jpg";
//         imgcodecs::imwrite(output_path, &canvas, &core::Vector::new())?;

//         Ok(())
//     }
// }
