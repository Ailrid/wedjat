use crate::define_feature_data;
use serde::{Deserialize, Serialize};
// 定义并自动实现从字节流提取的逻辑
define_feature_data!(KeyPoints, i64, (usize, usize));
define_feature_data!(Matches, i64, (usize, usize));
define_feature_data!(Scores, f32, usize);

/// 算法提取的结果.
///
/// # Fields
///
/// - `first_kpts` (`KeyPoints`) - 第一张图的特征点[256,2].
/// - `second_kpts` (`KeyPoints`) - 第二张图的特征点[256,2].
/// - `matches` (`Matches`) - 匹配点对[n,3].第一个维度没用
/// - `scores` (`Scores`) - 匹配点得分[n].
///
pub struct FeatureData {
    pub first_kpts: KeyPoints,
    pub second_kpts: KeyPoints,
    pub matches: Matches,
    pub scores: Scores,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtractorCfg {
    // 后端类型,onnx或者vino
    pub backend_type: String,
    pub model_path: String,
    // 线程数
    pub threads: usize,
    // 设备类型,CPU / GPU / NPU
    pub device: String,
    // 匹配成功需要的最小内点数
    pub min_inliers: usize,
    // 匹配成功需要满足的得分阈值
    pub score_threshold: f32,
    // 是否检查行列式
    pub check_det: bool,
}

impl Default for ExtractorCfg {
    fn default() -> Self {
        Self {
            backend_type: "onnx".to_string(),
            model_path: "".to_string(),
            threads: 1,
            device: "CPU".to_string(),
            min_inliers: 10,
            score_threshold: 0.4,
            check_det: true,
        }
    }
}

pub const MODEL_POINTS: usize = 256;
pub const IMAGE_SIZE: usize = 256;
