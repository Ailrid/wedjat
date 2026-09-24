use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub qdrant_url: String,
    pub qdrant_port: usize,
    pub qdrant_collection_name: String,
    pub qdrant_query_limit: usize,
    pub qdrant_query_radius: f32,
    pub tiff_folder: String,
    pub device_id: String,
    // superpoint
    pub superpoint_rknn_path: String,
    pub superpoint_weight_path: String,
    pub num_keypoints: usize,
    pub nms_radius: usize,
    pub remove_borders: usize,
    // lightglue
    pub lightglue_rknn_path: String,
    pub lightglue_weight_path: String,
    pub match_threshold: f32,
    // metric
    pub metric_rknn_path: String,
    pub metric_weight_path: String,
    // estimator
    /// 悬停判定门限 (米)
    pub hover_threshold: f64,
    /// 轨迹由 Initializing 升级为 Tracking 所需的连续命中帧数
    pub init_hits: usize,
    /// 允许的最大连续跟丢/失配帧数，超过后轨迹转为 Dead
    pub dead_frames: usize,
    /// 匹配分配时的距离惩罚截断阈值 (距离超过此值的匹配将被忽略)
    pub match_distance_cutoff: f32,
    /// 输入原始观测点近距离 NMS 去重的平移聚类半径 (米)
    pub nms_radius_meters: f64,
    /// 轨迹必须具备的最小空间跨度 (米)，底气达标才对外输出定位
    pub min_span_required: f64,
}
