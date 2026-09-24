use camera::Frame;
use client::qdrant::QdrantResult;
use nalgebra::{Matrix3, Vector3};
use opencv::core::{Mat, MatTraitConst, Point2f};


#[derive(Debug, Clone)]
pub struct CenterLocation {
    pub pixel_x: u32,
    pub pixel_y: u32,
    pub ecef_x: f64,
    pub ecef_y: f64,
    pub ecef_z: f64,
    pub lat: f64,
    pub lon: f64,
    pub yaw: f32,
}

#[derive(Debug, Clone)]
pub struct FrameLocation {
    pub frame: Frame,
    pub item: QdrantResult,
    pub center: CenterLocation,
}

#[derive(Debug, Clone)]
pub enum LocationResult {
    Location(FrameLocation),
    Uncertain(String),
}

#[derive(Debug, Clone)]
pub struct FramePoint {
    pub inliers: usize,
    pub center_location: CenterLocation,
    pub qdrant_result: QdrantResult,
}

fn geodetic_to_ecef(lat: f64, lon: f64, alt: f64) -> (f64, f64, f64) {
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;
    const B: f64 = A * (1.0 - F);
    const E2: f64 = (A * A - B * B) / (A * A);

    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();

    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    let n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();

    let x = (n + alt) * cos_lat * cos_lon;
    let y = (n + alt) * cos_lat * sin_lon;
    let z = (n * (1.0 - E2) + alt) * sin_lat;

    (x, y, z)
}

impl FramePoint {
    pub fn new(
        inliers: usize,
        center: Point2f,
        h_matrix: &Mat,
        qdrant_result: QdrantResult,
    ) -> Result<Self, opencv::error::Error> {
        let payload = &qdrant_result.payload;

        let res_x = payload.res.get(0).copied().unwrap_or(0.5);
        let res_y = payload.res.get(1).copied().unwrap_or(0.5);

        let dx_meters = center.x as f64 * res_x;
        let dy_meters = center.y as f64 * res_y;

        const METERS_PER_DEGREE_LAT: f64 = 111_000.0;
        let meters_per_degree_lon = METERS_PER_DEGREE_LAT * payload.location.lat.to_radians().cos();

        let delta_lat = -dy_meters / METERS_PER_DEGREE_LAT;
        let delta_lon = dx_meters / meters_per_degree_lon;

        let calculated_lat = payload.location.lat + delta_lat;
        let calculated_lon = payload.location.lon + delta_lon;

        let (ecef_x, ecef_y, ecef_z) = geodetic_to_ecef(calculated_lat, calculated_lon, 0.0);

        let h00 = *h_matrix.at_2d::<f64>(0, 0)?;
        let h10 = *h_matrix.at_2d::<f64>(1, 0)?;
        let yaw_rad = h10.atan2(h00);
        let yaw_deg = yaw_rad.to_degrees() as f32;

        let pixel_x = payload.x + center.x as u32;
        let pixel_y = payload.y + center.y as u32;

        let center_location = CenterLocation {
            pixel_x,
            pixel_y,
            ecef_x,
            ecef_y,
            ecef_z,
            lat: calculated_lat,
            lon: calculated_lon,
            yaw: yaw_deg,
        };

        Ok(Self {
            inliers,
            center_location,
            qdrant_result,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnuPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl EnuPoint {
    pub fn distance_to(&self, other: &EnuPoint) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    Initializing,
    Tracking,
    Lost,
    Dead,
}

pub struct Trajectory {
    pub points: Vec<(FramePoint, EnuPoint)>,
    pub health: f32,
    pub accumulated_inliers: usize,
    pub consecutive_failures: usize,
    pub consecutive_hits: usize,
    pub start_time: std::time::Instant,
    pub rotate: Matrix3<f64>,
    pub translation: Vector3<f64>,
    pub state: TrackState,
    pub total_span: f64,
    hover_threshold: f64,
    init_hits: usize,
    dead_frames: usize,
}

impl Trajectory {
    pub fn new(
        first_point: FramePoint,
        hover_threshold: f64,
        init_hits: usize,
        dead_frames: usize,
    ) -> Self {
        let translation = Vector3::new(
            first_point.center_location.ecef_x,
            first_point.center_location.ecef_y,
            first_point.center_location.ecef_z,
        );

        let up = translation.normalize();
        let z_axis = Vector3::new(0.0, 0.0, 1.0);
        let east = z_axis.cross(&up).normalize();
        let north = up.cross(&east).normalize();

        let rotate = Matrix3::new(
            east.x, east.y, east.z, north.x, north.y, north.z, up.x, up.y, up.z,
        );

        let first_enu_point = EnuPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        let initial_inliers = first_point.inliers;

        Self {
            points: vec![(first_point, first_enu_point)],
            health: 15.0,
            accumulated_inliers: initial_inliers,
            consecutive_failures: 0,
            consecutive_hits: 1,
            start_time: std::time::Instant::now(),
            rotate,
            translation,
            state: TrackState::Initializing,
            total_span: 0.0,
            hover_threshold,
            init_hits,
            dead_frames,
        }
    }

    pub fn ecef_to_enu(&self, ecef_x: f64, ecef_y: f64, ecef_z: f64) -> EnuPoint {
        let ecef_vec = Vector3::new(ecef_x, ecef_y, ecef_z);
        let delta = ecef_vec - self.translation;
        let enu_vec = self.rotate * delta;
        EnuPoint {
            x: enu_vec.x,
            y: enu_vec.y,
            z: enu_vec.z,
        }
    }

    pub fn distance(&self, point: &FramePoint) -> f32 {
        if let Some((_, last_enu)) = self.points.last() {
            let candidate_enu = self.ecef_to_enu(
                point.center_location.ecef_x,
                point.center_location.ecef_y,
                point.center_location.ecef_z,
            );
            let spatial_dist = last_enu.distance_to(&candidate_enu) as f32;
            let quality_factor = 10.0 / (point.inliers as f32 + 1.0);

            spatial_dist + quality_factor
        } else {
            f32::MAX
        }
    }

    pub fn update(&mut self, point_opt: Option<&FramePoint>) {
        match point_opt {
            Some(point) => {
                self.consecutive_failures = 0;
                self.consecutive_hits += 1;
                self.health = (self.health + 1.0).min(20.0);

                let new_enu = self.ecef_to_enu(
                    point.center_location.ecef_x,
                    point.center_location.ecef_y,
                    point.center_location.ecef_z,
                );

                if let Some((_, last_enu)) = self.points.last() {
                    let step_dist = last_enu.distance_to(&new_enu);

                    if step_dist >= self.hover_threshold {
                        self.accumulated_inliers += point.inliers;
                        self.total_span += step_dist;
                        self.points.push((point.clone(), new_enu));
                    } else if let Some(last_pair) = self.points.last_mut() {
                        last_pair.0 = point.clone();
                    }
                }

                match self.state {
                    TrackState::Initializing => {
                        if self.consecutive_hits >= self.init_hits {
                            self.state = TrackState::Tracking;
                        }
                    }
                    TrackState::Lost => {
                        self.state = TrackState::Tracking;
                    }
                    _ => {}
                }
            }
            None => {
                self.consecutive_failures += 1;
                self.consecutive_hits = 0;
                self.health -= 3.0;

                match self.state {
                    TrackState::Tracking => {
                        self.state = TrackState::Lost;
                    }
                    TrackState::Lost => {
                        if self.consecutive_failures > self.dead_frames || self.health <= 0.0 {
                            self.state = TrackState::Dead;
                        }
                    }
                    TrackState::Dead => {}
                    TrackState::Initializing => {
                        self.state = TrackState::Dead;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EstimatorConfig {
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

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self {
            hover_threshold: 0.5,
            init_hits: 3,
            dead_frames: 5,
            match_distance_cutoff: 15.0,
            nms_radius_meters: 0.2,
            min_span_required: 2.0,
        }
    }
}

pub struct Estimator {
    pub trajectories: Vec<Trajectory>,
    pub config: EstimatorConfig,
}

impl Estimator {
    pub fn new(config: EstimatorConfig) -> Self {
        Self {
            trajectories: Vec::new(),
            config,
        }
    }

    pub fn update_frame(&mut self, raw_points: Vec<FramePoint>, frame: &Frame) -> LocationResult {
        // 原始观测点聚类去重
        let merged_points = self.merge_points(raw_points);
        // 构建代价矩阵并做全局关联
        let assignments = self.associate_points(&merged_points);
        // 驱动所有轨迹演进 (Update) 与新轨迹开户
        self.update_trajectories(&merged_points, &assignments);
        // 垃圾回收 (Retain) 与全局定位裁决 (Estimate)
        self.cleanup_and_estimate(frame)
    }

    /// 基于 ECEF 几何欧氏距离做单帧内点的近距离去重 (NMS)
    fn merge_points(&self, points: Vec<FramePoint>) -> Vec<FramePoint> {
        let mut merged: Vec<FramePoint> = Vec::new();
        let radius_sq = self.config.nms_radius_meters.powi(2);

        for pt in points {
            let mut is_duplicate = false;
            for existing in &merged {
                let dx = pt.center_location.ecef_x - existing.center_location.ecef_x;
                let dy = pt.center_location.ecef_y - existing.center_location.ecef_y;
                let dz = pt.center_location.ecef_z - existing.center_location.ecef_z;
                if (dx * dx + dy * dy + dz * dz) < radius_sq {
                    is_duplicate = true;
                    break;
                }
            }
            if !is_duplicate {
                merged.push(pt);
            }
        }
        merged
    }

    /// 第二道关：计算轨迹与观测点的 Cost 并做最优匹配分配
    fn associate_points(&self, merged_points: &[FramePoint]) -> Vec<Option<usize>> {
        let mut assignments: Vec<Option<usize>> = vec![None; self.trajectories.len()];
        let mut assigned_points = vec![false; merged_points.len()];

        for (t_idx, traj) in self.trajectories.iter().enumerate() {
            if traj.state == TrackState::Dead {
                continue;
            }

            let mut best_cost = self.config.match_distance_cutoff;
            let mut best_p_idx = None;

            for (p_idx, point) in merged_points.iter().enumerate() {
                if assigned_points[p_idx] {
                    continue;
                }

                let cost = traj.distance(point);
                if cost < best_cost {
                    best_cost = cost;
                    best_p_idx = Some(p_idx);
                }
            }

            if let Some(p_idx) = best_p_idx {
                assignments[t_idx] = Some(p_idx);
                assigned_points[p_idx] = true;
            }
        }

        assignments
    }

    /// 第三道关：更新已有轨迹，并为孤立未匹配点开辟新 Initializing 轨迹
    fn update_trajectories(&mut self, merged_points: &[FramePoint], assignments: &[Option<usize>]) {
        let mut point_assigned_flags = vec![false; merged_points.len()];

        // 1. 更新现有轨迹
        for (t_idx, traj) in self.trajectories.iter_mut().enumerate() {
            if traj.state == TrackState::Dead {
                continue;
            }

            if let Some(p_idx) = assignments[t_idx] {
                traj.update(Some(&merged_points[p_idx]));
                point_assigned_flags[p_idx] = true;
            } else {
                traj.update(None);
            }
        }

        // 2. 为未分配点创建新 Trajectory
        for (p_idx, &is_assigned) in point_assigned_flags.iter().enumerate() {
            if !is_assigned {
                let new_traj = Trajectory::new(
                    merged_points[p_idx].clone(),
                    self.config.hover_threshold,
                    self.config.init_hits,
                    self.config.dead_frames,
                );
                self.trajectories.push(new_traj);
            }
        }
    }

    /// 清理销毁死轨迹，并在合法轨迹中裁决最佳输出，附带明确失败原因
    fn cleanup_and_estimate(&mut self, frame: &Frame) -> LocationResult {
        // 彻底清理死掉的轨迹
        self.trajectories.retain(|t| t.state != TrackState::Dead);

        if self.trajectories.is_empty() {
            return LocationResult::Uncertain(
                "No active trajectories available in estimator".to_string(),
            );
        }

        // 查找并统计处于不同状态下的轨迹
        let tracking_trajectories: Vec<&Trajectory> = self
            .trajectories
            .iter()
            .filter(|t| t.state == TrackState::Tracking)
            .collect();

        if tracking_trajectories.is_empty() {
            let init_count = self
                .trajectories
                .iter()
                .filter(|t| t.state == TrackState::Initializing)
                .count();
            let lost_count = self
                .trajectories
                .iter()
                .filter(|t| t.state == TrackState::Lost)
                .count();
            return LocationResult::Uncertain(format!(
                "No trajectories in Tracking state (Initializing: {}, Lost: {})",
                init_count, lost_count
            ));
        }

        // 筛选空间跨度（底气）达标的轨迹
        let valid_trajectories: Vec<&&Trajectory> = tracking_trajectories
            .iter()
            .filter(|t| t.total_span >= self.config.min_span_required)
            .collect();

        if valid_trajectories.is_empty() {
            let max_span = tracking_trajectories
                .iter()
                .map(|t| t.total_span)
                .fold(0.0f64, f64::max);
            return LocationResult::Uncertain(format!(
                "Insufficient spatial span: max current span is {:.2}m, required {:.2}m",
                max_span, self.config.min_span_required
            ));
        }

        // 4. 从达标的轨迹中，按内点总数和跨度裁决出最优的一条
        let best_traj = valid_trajectories.into_iter().max_by(|a, b| {
            a.accumulated_inliers
                .cmp(&b.accumulated_inliers)
                .then_with(|| a.total_span.partial_cmp(&b.total_span).unwrap())
        });

        if let Some(traj) = best_traj {
            if let Some((last_point, _)) = traj.points.last() {
                let frame_location = FrameLocation {
                    frame: frame.clone(),
                    item: last_point.qdrant_result.clone(),
                    center: last_point.center_location.clone(),
                };
                return LocationResult::Location(frame_location);
            }
        }

        LocationResult::Uncertain(
            "Failed to extract valid location point from best trajectory".to_string(),
        )
    }
}
