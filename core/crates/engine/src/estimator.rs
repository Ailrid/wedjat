use crate::types::{ENUPoint, PredictPoint, PredictResult};
use nalgebra::{Matrix3, Vector3};

//这个结构体用于保存一个可能的路径
pub struct Trajectory {
    /// 记录上一次更新后的路径长度，用于判断本帧是否成功匹配
    pub pre_length: usize,
    /// 路径中所有的绝对位置观测点
    pub points: Vec<ENUPoint>,
    /// 评估路径的生命值（例如 0.0 ~ 20.0），决定是否保留该路径
    pub health: f32,
    /// 该轨迹累计获得的内点总数，代表了轨迹的“历史底气”
    pub accumulated_inliers: usize,
    /// 记录该轨迹最近连续多少帧没有获得新的观测点
    pub consecutive_failures: usize,
    /// 轨迹创建的时间戳，用于计算轨迹的“年龄”
    pub start_time: std::time::Instant,
    // 上一次的位置
    pub last_pos: Vector3<f32>,
    // 预测的位置
    pub pred_pos: Option<Vector3<f32>>,
    // 预测的不确定性
    pub uncertainty: f32,
    //ENU to ECEF
    pub rotate: Matrix3<f32>,
    pub translation: Vector3<f64>,
}

impl Trajectory {
    /// 创建新轨迹，必须由一个高质量的初始点开启
    pub fn new(first_point: PredictPoint) -> Self {
        // 设置东北天坐标系原点
        let translation = Vector3::new(first_point.x, first_point.y, first_point.z);
        // 计算旋转矩阵 (ECEF -> ENU)
        // Up轴：指向天空
        let up = translation.normalize();
        // 临时北极轴 [0, 0, 1]
        let z_axis = Vector3::new(0.0, 0.0, 1.0);
        // East轴
        let east = z_axis.cross(&up).normalize();
        // North轴
        let north = up.cross(&east).normalize();
        // 构造旋转矩阵
        let r = east.cast::<f32>();
        let n = north.cast::<f32>();
        let u = up.cast::<f32>();

        let rotate = Matrix3::new(
            r.x, r.y, r.z, // 第一行 (East)
            n.x, n.y, n.z, // 第二行 (North)
            u.x, u.y, u.z, // 第三行 (Up)
        );

        // 将第一个点转为内部的 ENUPoint
        // 第一个点相对于自己，pos 永远是 [0, 0, 0]
        let first_enu_point = ENUPoint {
            pos: Vector3::zeros(),
            utm_x: first_point.utm_x,
            utm_y: first_point.utm_y,
            crs: first_point.crs.clone(),
            pixel_x: first_point.pixel_x,
            pixel_y: first_point.pixel_y,
            src: first_point.src.clone(),
            row_pos: first_point,
        };

        Self {
            pre_length: 0,
            accumulated_inliers: first_enu_point.row_pos.inlier_count,
            health: 15.0,
            consecutive_failures: 0,
            start_time: std::time::Instant::now(),
            // 局部系起点永远是 0
            last_pos: Vector3::zeros(),
            pred_pos: None,
            uncertainty: 1.0,
            points: vec![first_enu_point],
            rotate,
            translation,
        }
    }
    /// 外部 ECEF -> 内部 ENU
    pub fn world_to_local(&self, ecef_x: f64, ecef_y: f64, ecef_z: f64) -> Vector3<f32> {
        let ecef_vec = Vector3::new(ecef_x, ecef_y, ecef_z);
        let delta = ecef_vec - self.translation; // f64 减法保精度
        self.rotate * delta.cast::<f32>() // 转 f32 后旋转
    }

    /// 内部 ENU -> 外部 ECEF
    pub fn local_to_world(&self, local_pos: &Vector3<f32>) -> (f64, f64, f64) {
        // 旋转矩阵转置就是逆转换
        let delta_ecef = self.rotate.transpose() * local_pos;
        let res = delta_ecef.cast::<f64>() + self.translation;
        (res.x, res.y, res.z)
    }

    /// 向轨迹添加新点，并立即更新累积信息
    pub fn push(&mut self, new_point: &PredictPoint) {
        // 累加内点数
        self.accumulated_inliers += new_point.inlier_count;

        // 连续失败计数立刻清零
        self.consecutive_failures = 0;

        // ECEF (f64) -> ENU (f32)
        let local_pos = self.world_to_local(new_point.x, new_point.y, new_point.z);

        // 存入
        let enu_point = ENUPoint {
            pos: local_pos,             // 局部切平面坐标 (用于内部计算)
            row_pos: new_point.clone(), // 原始 ECEF 观测 (用于对账或输出)
            utm_x: new_point.utm_x,
            utm_y: new_point.utm_y,
            crs: new_point.crs.clone(),
            pixel_x: new_point.pixel_x,
            pixel_y: new_point.pixel_y,
            src: new_point.src.clone(),
        };

        self.points.push(enu_point);
    }

    /// 每一帧调用的状态维护函数
    pub fn update(&mut self) {
        match &self.points[..] {
            // 抓到了新点
            [.., p_curr] if self.points.len() > self.pre_length => {
                self.pre_length = self.points.len();

                // 用地图匹配给的 ENU 坐标覆盖预测值
                self.last_pos = p_curr.pos;

                // 状态维护
                self.consecutive_failures = 0;
                self.uncertainty = 1.0; // 既然匹配到了，不确定性重置

                // 奖励健康值
                let bonus = (p_curr.row_pos.inlier_count as f32 / 20.0).min(2.0);
                self.health = (self.health + 1.0 + bonus).min(30.0);
            }

            // 没抓到新点
            _ => {
                self.consecutive_failures += 1;
                // 惯性续航：如果这一帧有 IMU 预测，就用预测值更新 last_pos
                // 这样下一秒的 predict 就能从这个“推测的位置”继续起跳
                if let Some(prediction) = self.pred_pos {
                    self.last_pos = prediction;
                }
                // 惩罚健康值
                let penalty = 1.0 * (self.consecutive_failures as f32);
                self.health -= penalty;
                // 增加不确定性
                self.uncertainty += 0.5;
            }
        }
        // 清空预测
        self.pred_pos = None;
    }

    /// 如果给了位移，直接加
    pub fn predict(&mut self, displacement: &Vector3<f32>) {
        self.pred_pos = Some(self.last_pos + displacement);
    }

    pub fn distance(&self, new_point: &PredictPoint) -> f32 {
        // 将 ECEF 地图点转换到局部 ENU 切平面
        let obs_pos_enu = self.world_to_local(new_point.x, new_point.y, new_point.z);
        let spatial_dist;
        match self.pred_pos {
            Some(prediction) => {
                // 有外部给的预测，用外部预测来算
                spatial_dist = (obs_pos_enu - prediction).norm();
            }
            None => {
                // 没有外部的预测，用内部平均速度来算
                let last_point_time = self.points[self.points.len() - 1].row_pos.time;
                let dt = (new_point.time - last_point_time).as_secs_f32();

                let v = match &self.points[..] {
                    [.., p_prve, p_last] => (p_last.pos - p_prve.pos) / dt,
                    _ => Vector3::zeros(),
                };
                // 尝试匀速外推
                let fallback_prediction = self.last_pos + v * dt;
                spatial_dist = (obs_pos_enu - fallback_prediction).norm();
            }
        }
        let quality_factor = 20.0 / (new_point.inlier_count as f32).max(1.0);
        spatial_dist * quality_factor / self.uncertainty
    }

    /// 该轨迹现在给出的坐标，能不能信？
    pub fn is_reliable(&self) -> bool {
        // 宁缺毋滥原则：
        // 1. 必须是这帧刚更新过的 (consecutive_failures == 0)
        // 2. 轨迹不能太短 (至少要有 3 个点的时序共识)
        // 3. 累计内点数要达标 (代表不仅有长度，还有质量)
        self.consecutive_failures == 0 && self.points.len() >= 3 && self.accumulated_inliers > 60
    }

    /// 获取当前最新的坐标观测（用于 Estimator 输出）
    pub fn get_latest_pos(&self) -> Option<PredictResult> {
        if self.is_reliable() {
            match &self.points[..] {
                // 抓到了新点
                [.., p_curr] => {
                    let xyz = self.local_to_world(&p_curr.pos);
                    Some(PredictResult {
                        x: xyz.0,
                        y: xyz.1,
                        z: xyz.2,
                        utm_x: p_curr.utm_x,
                        utm_y: p_curr.utm_y,
                        crs: p_curr.crs.clone(),
                        src: p_curr.src.clone(),
                        pixel_x: p_curr.pixel_x,
                        pixel_y: p_curr.pixel_y,
                    })
                }
                _ => {
                    return None;
                }
            }
        } else {
            None
        }
    }
}

pub struct Estimator {
    //路径缓冲区
    pub buffer_trajectory: Vec<Trajectory>,
    // 合并阈值
    pub merge_threshold: f32,
    // 匹配阈值
    pub match_threshold: f32,
}

impl Estimator {
    pub fn new(merge_threshold: f32, match_threshold: f32) -> Self {
        Self {
            buffer_trajectory: Vec::new(),
            merge_threshold: merge_threshold,
            match_threshold: match_threshold,
        }
    }

    pub fn merge_points(&mut self, new_points: Vec<PredictPoint>) -> Vec<PredictPoint> {
        // 只有一个点，不合并
        if new_points.len() <= 1 {
            return new_points;
        }

        let mut merged_results = Vec::new();
        let mut processed = vec![false; new_points.len()];

        // 贪婪合并
        for i in 0..new_points.len() {
            if processed[i] {
                continue;
            }

            // 找到所有与当前点 i 距离近的点（形成一个簇）
            let mut cluster_indices = Vec::new();
            cluster_indices.push(i); // 包含点 i 自己
            processed[i] = true;

            for j in (i + 1)..new_points.len() {
                // 使用欧式距离判断是否属于同一个物理位置的冗余观测
                if !processed[j] && new_points[i].distance(&new_points[j]) < self.merge_threshold {
                    cluster_indices.push(j);
                    processed[j] = true;
                }
            }

            // 如果簇内只有一个点，直接保留
            if cluster_indices.len() == 1 {
                merged_results.push(new_points[i].clone());
                continue;
            }

            // 执行多源信息加权平差
            let mut total_weight = 0.0;
            let (mut sum_x, mut sum_y, mut sum_z) = (0.0, 0.0, 0.0);
            let (mut sum_utm_x, mut sum_utm_y) = (0.0, 0.0);
            let (mut sum_pixel_x, mut sum_pixel_y) = (0.0, 0.0);

            let mut max_score = 0.0f32;
            let mut total_inliers = 0;

            for &idx in &cluster_indices {
                let p = &new_points[idx];
                let uncertainty = 1.05 - p.score as f64;

                // 权重 = 证据强度 / 不确定性的平方
                let weight = (p.inlier_count as f64 + 1.0) / uncertainty.powi(2);

                sum_x += p.x * weight;
                sum_y += p.y * weight;
                sum_z += p.z * weight;
                sum_utm_x += p.utm_x * weight;
                sum_utm_y += p.utm_y * weight;
                sum_pixel_x += p.pixel_x * weight;
                sum_pixel_y += p.pixel_y * weight;

                total_weight += weight;

                max_score = max_score.max(p.score);
                total_inliers += p.inlier_count;
            }

            // 生成融合后的高精度观测点
            if total_weight > 0.0 {
                merged_results.push(PredictPoint {
                    x: sum_x / total_weight,
                    y: sum_y / total_weight,
                    z: sum_z / total_weight,
                    utm_x: sum_utm_x / total_weight,
                    utm_y: sum_utm_y / total_weight,
                    pixel_x: sum_pixel_x / total_weight,
                    pixel_y: sum_pixel_y / total_weight,
                    src: new_points[cluster_indices[0]].src.clone(),
                    crs: new_points[cluster_indices[0]].crs.clone(), // 以簇中第一个点的坐标系为准
                    time: new_points[cluster_indices[0]].time,
                    frame_id: new_points[cluster_indices[0]].frame_id,
                    inlier_count: total_inliers, // 累加内点，增强该点在后续匹配中的胜算
                    score: max_score,            // 保留最高置信度作为代表
                });
            }
        }

        merged_results
    }

    ///更新轨迹，传入新的点对比历史轨迹给出预测
    pub fn update(&mut self, new_points: Vec<PredictPoint>, displacement: Option<Vector3<f32>>) {
        if let Some(params) = &displacement {
            for traj in &mut self.buffer_trajectory {
                // 利用IMU预测轨迹的新位置
                traj.predict(&params);
            }
        }

        // 合并近距离的冗余观测（解决“一处多点”的问题）
        let merged_points = self.merge_points(new_points);

        //计算所有轨迹与所有新点之间的综合距离
        let mut all_candidates = merged_points
            .iter()
            .enumerate()
            .flat_map(|(p_idx, pt)| {
                self.buffer_trajectory
                    .iter()
                    .enumerate()
                    .map(move |(t_idx, traj)| (t_idx, p_idx, traj.distance(pt)))
            })
            .filter(|(_, _, dist)| *dist < self.match_threshold)
            .collect::<Vec<_>>();

        // 按综合距离从小到大排
        all_candidates.sort_by(|a, b| a.2.total_cmp(&b.2));

        // 记录本帧哪些点被用了，哪些轨迹更新了
        let mut point_used = vec![false; merged_points.len()];
        let mut traj_updated = vec![false; self.buffer_trajectory.len()];

        // 贪婪匹配,为现有的轨迹分配最合适的点
        for (t_idx, p_idx, _) in all_candidates {
            if !point_used[p_idx] && !traj_updated[t_idx] {
                // 把点塞进去
                self.buffer_trajectory[t_idx].push(&merged_points[p_idx]);
                point_used[p_idx] = true;
                traj_updated[t_idx] = true;
            }
        }

        // 自立门户：那些清晰且没人要的点开启新轨迹
        let new_trajs = point_used
            .iter()
            .enumerate()
            .filter(|&(p_idx, &used)| !used && merged_points[p_idx].inlier_count > 15)
            .map(|(p_idx, _)| Trajectory::new(merged_points[p_idx].clone()))
            .collect::<Vec<_>>();

        // 将新轨迹加入缓冲池
        self.buffer_trajectory.extend(new_trajs);

        // 状态结算：统一调用每个轨迹的 update()
        self.buffer_trajectory.iter_mut().for_each(|t| t.update());

        // 踢掉死掉的轨迹
        self.buffer_trajectory.retain(|t| t.health > 0.0);
    }

    /// 获得当前最可能的位置
    pub fn estimate(&self) -> Option<PredictResult> {
        // 提取所有可靠轨迹并按内点数排序
        let mut reliable_trajs: Vec<_> = self
            .buffer_trajectory
            .iter()
            .filter(|t| t.is_reliable())
            .collect();

        // 按累积内点降序排列
        reliable_trajs.sort_by(|a, b| b.accumulated_inliers.cmp(&a.accumulated_inliers));

        if reliable_trajs.is_empty() {
            return None;
        }

        // 如果只有一条可靠轨迹，直接输出
        if reliable_trajs.len() == 1 {
            return reliable_trajs[0].get_latest_pos();
        }

        // 存在多条可靠轨迹，进行“双强对比”
        let best = reliable_trajs[0];
        let second = reliable_trajs[1];

        // 计算空间距离
        let pos_best = best.get_latest_pos()?;
        let pos_second = second.get_latest_pos()?;
        let dist = ((pos_best.x - pos_second.x).powi(2)
            + (pos_best.y - pos_second.y).powi(2)
            + (pos_best.z - pos_second.z).powi(2))
        .sqrt();

        // 如果两者的位置离得很近（< 5.0m），认为没有歧义，输出最强的那个
        if dist < 5.0 {
            return Some(pos_best);
        }

        // 如果第一名的内点数是第二名的 1.5 倍以上，认为“真理已明”，输出最强的
        if best.accumulated_inliers as f32 > second.accumulated_inliers as f32 * 1.5 {
            return Some(pos_best);
        }

        // 位置远且内点数旗鼓相当 -> 存在严重歧义，闭嘴
        None
    }
}
