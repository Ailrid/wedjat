use nalgebra::{Matrix3, SMatrix, SVector, UnitQuaternion, Vector3};
use std::ops::AddAssign;

pub const O_R: usize = 0;
pub const O_V: usize = 3;
pub const O_P: usize = 6;
pub const O_BG: usize = 9;
pub const O_BA: usize = 12;

///IMU测量数据
pub struct ImuMeasurement {
    pub timestamp: f32,
    pub acc: Vector3<f32>,
    pub gyro: Vector3<f32>,
}

///雅克比矩阵的各项
pub struct PreintegratedJacobians {
    pub rot_bg: Matrix3<f32>, // JRg
    pub vel_bg: Matrix3<f32>, // JVg
    pub vel_ba: Matrix3<f32>, // JVa
    pub pos_bg: Matrix3<f32>, // JPg
    pub pos_ba: Matrix3<f32>, // JPa
}

impl Default for PreintegratedJacobians {
    fn default() -> Self {
        Self {
            pos_ba: Matrix3::zeros(),
            pos_bg: Matrix3::zeros(),
            vel_ba: Matrix3::zeros(),
            vel_bg: Matrix3::zeros(),
            rot_bg: Matrix3::zeros(),
        }
    }
}

///预积分的结果
pub struct Preintegration {
    pub dt: f32,
    pub acc_bias: Vector3<f32>,
    pub gyro_bias: Vector3<f32>,

    // 预积分增量量，以第一个点的位置为基准坐标系
    pub delta_p: Vector3<f32>,
    pub delta_v: Vector3<f32>,
    pub delta_r: UnitQuaternion<f32>,

    // 平均速度和角速度，以第一个点的位置为基准坐标系
    pub mean_acc: Vector3<f32>,
    pub mean_gyro: Vector3<f32>,

    // 雅可比矩阵
    pub jacobians: PreintegratedJacobians,

    // 协方差矩阵15x15
    pub covariance: SMatrix<f32, 15, 15>,

    // 计算协方差用到的缓存矩阵，避免重复申请内存
    mat_a: SMatrix<f32, 9, 9>,
    mat_b: SMatrix<f32, 9, 6>,

    // 随机游走噪声的方差
    nga: SMatrix<f32, 6, 6>,
    nga_walk: SMatrix<f32, 6, 6>,
}

impl Preintegration {
    /// Describe this function.
    ///
    /// # Arguments
    ///
    /// - `acc_bias` (`Vector3<f32>`) -加速度计偏置.
    /// - `gyro_bias` (`Vector3<f32>`) - 角速度计偏置.
    /// - `noise_acc` (`f32`) - 加速度噪声方差.
    /// - `noise_gyro` (`f32`) - 角速度噪声方差.
    /// - `noise_acc_walk` (`f32`) - 加速度随机游走噪声方差.
    /// - `noise_gyro_walk` (`f32`) - 角速度随机游走噪声方差.

    pub fn new(
        acc_bias: Vector3<f32>,
        gyro_bias: Vector3<f32>,
        noise_acc: f32,
        noise_gyro: f32,
        noise_acc_walk: f32,
        noise_gyro_walk: f32,
    ) -> Self {
        let nga = SMatrix::<f32, 6, 6>::from_diagonal(&SVector::<f32, 6>::from_column_slice(&[
            noise_gyro.powi(2),
            noise_gyro.powi(2),
            noise_gyro.powi(2),
            noise_acc.powi(2),
            noise_acc.powi(2),
            noise_acc.powi(2),
        ]));

        let nga_walk =
            SMatrix::<f32, 6, 6>::from_diagonal(&SVector::<f32, 6>::from_column_slice(&[
                noise_gyro_walk.powi(2),
                noise_gyro_walk.powi(2),
                noise_gyro_walk.powi(2),
                noise_acc_walk.powi(2),
                noise_acc_walk.powi(2),
                noise_acc_walk.powi(2),
            ]));

        let mut obj = Self {
            dt: 0.0,
            acc_bias,
            gyro_bias,
            delta_p: Vector3::zeros(),
            delta_v: Vector3::zeros(),
            delta_r: UnitQuaternion::identity(),
            mean_acc: Vector3::zeros(),
            mean_gyro: Vector3::zeros(),
            jacobians: PreintegratedJacobians::default(),
            covariance: SMatrix::<f32, 15, 15>::zeros(),
            mat_a: SMatrix::<f32, 9, 9>::identity(),
            mat_b: SMatrix::<f32, 9, 6>::zeros(),
            nga,
            nga_walk,
        };
        obj.reset();
        obj
    }
    pub fn integrate(&mut self, acc: &Vector3<f32>, gyro: &Vector3<f32>, dt: f32) {
        // 减去偏差
        let un_acc = acc - self.acc_bias;
        let un_gyro = gyro - self.gyro_bias;

        // 保存当前的平均加速度和角速度
        self.mean_acc = (self.dt * self.mean_acc + self.delta_r * un_acc * dt) / (self.dt + dt);
        self.mean_gyro = (self.dt * self.mean_gyro + un_gyro * dt) / (self.dt + dt);

        // 更新p和v
        let acc_world = self.delta_r * un_acc;
        self.delta_p += self.delta_v * dt + 0.5 * acc_world * dt * dt;
        self.delta_v += acc_world * dt;

        //更新雅克比和协方差矩阵
        self.update_state(&un_acc, &un_gyro, dt);

        //更新r

        self.dt += dt;
    }

    pub fn update_state(&mut self, un_acc: &Vector3<f32>, un_gyro: &Vector3<f32>, dt: f32) {
        // 计算局部旋转增量及其右雅可比
        let rotation_vec = un_gyro * dt;
        let d_ri = UnitQuaternion::from_scaled_axis(rotation_vec);
        let right_j = right_jacobian(&rotation_vec);
        // 转换成反对称矩阵
        let acc_hat = hat(&un_acc);
        let delta_r_mat = self.delta_r.to_rotation_matrix().matrix().clone();
        // 在更新r之前，更新雅可比和协方差
        self.update_jacobian(&delta_r_mat, &d_ri, &right_j, &acc_hat, dt);
        self.update_covariance(&delta_r_mat, &d_ri, &right_j, &acc_hat, dt);
        // 更新r
        self.delta_r = self.delta_r * d_ri;
        self.delta_r.renormalize();
    }

    ///更新雅克比矩阵
    pub fn update_jacobian(
        &mut self,
        delta_r_mat: &Matrix3<f32>,
        d_ri: &UnitQuaternion<f32>,
        right_j: &Matrix3<f32>,
        acc_hat: &Matrix3<f32>,
        dt: f32,
    ) {
        let dt2 = dt * dt;

        // 更新位置雅可比 (JPa, JPg)
        self.jacobians.pos_ba += self.jacobians.vel_ba * dt - 0.5 * delta_r_mat * dt2;
        self.jacobians.pos_bg +=
            self.jacobians.vel_bg * dt - 0.5 * delta_r_mat * acc_hat * dt2 * self.jacobians.rot_bg;

        // 更新速度雅可比 (JVa, JVg)
        self.jacobians.vel_ba -= delta_r_mat * dt;
        self.jacobians.vel_bg -= delta_r_mat * acc_hat * dt * self.jacobians.rot_bg;

        // 更新旋转雅可比 (JRg)
        // 使用 dRi.deltaR^T * JRg - JR * dt
        self.jacobians.rot_bg =
            d_ri.to_rotation_matrix().transpose().matrix() * self.jacobians.rot_bg - right_j * dt;
    }

    /// 更新协方差矩阵
    pub fn update_covariance(
        &mut self,
        delta_r_mat: &Matrix3<f32>,
        d_ri: &UnitQuaternion<f32>,
        right_j: &Matrix3<f32>,
        acc_hat: &Matrix3<f32>,
        dt: f32,
    ) {
        let dt2 = dt * dt;

        // 构造状态转移矩阵 A (9x9)
        // 对应 ORB-SLAM3 顺序: 0:R, 3:V, 6:P
        self.mat_a.fill_with_identity();

        // A.block<3, 3>(3, 0) = -dR * dt * Wacc;
        self.mat_a
            .fixed_view_mut::<3, 3>(3, 0)
            .copy_from(&(-delta_r_mat * dt * acc_hat));

        // A.block<3, 3>(6, 0) = -0.5f * dR * dt * dt * Wacc;
        self.mat_a
            .fixed_view_mut::<3, 3>(6, 0)
            .copy_from(&(-0.5 * delta_r_mat * dt2 * acc_hat));

        // A.block<3, 3>(6, 3) = dt * I;
        self.mat_a.fixed_view_mut::<3, 3>(6, 3).fill_with_identity();
        self.mat_a.fixed_view_mut::<3, 3>(6, 3).scale_mut(dt);

        // A.block<3, 3>(0, 0) = dRi.deltaR.transpose();
        self.mat_a
            .fixed_view_mut::<3, 3>(0, 0)
            .copy_from(&d_ri.to_rotation_matrix().transpose().matrix());

        // 构造噪声映射矩阵 B (9x6)
        self.mat_b.fill(0.0);

        // B.block<3, 3>(0, 0) = dRi.rightJ * dt; (针对陀螺仪噪声)
        self.mat_b
            .fixed_view_mut::<3, 3>(0, 0)
            .copy_from(&(right_j * dt));

        // B.block<3, 3>(3, 3) = dR * dt; (针对加速度计噪声)
        self.mat_b
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&(delta_r_mat * dt));

        // B.block<3, 3>(6, 3) = 0.5f * dR * dt * dt;
        self.mat_b
            .fixed_view_mut::<3, 3>(6, 3)
            .copy_from(&(0.5 * delta_r_mat * dt2));

        // 更新协方差矩阵的前 9 维: C = A * C * A^T + B * Nga * B^T
        let current_c = self.covariance.fixed_view::<9, 9>(0, 0);
        let next_c = self.mat_a * current_c * self.mat_a.transpose()
            + self.mat_b * self.nga * self.mat_b.transpose();

        self.covariance
            .fixed_view_mut::<9, 9>(0, 0)
            .copy_from(&next_c);

        // 更新随机游走部分 (偏置协方差)
        self.covariance
            .fixed_view_mut::<6, 6>(9, 9)
            .add_assign(&self.nga_walk* dt);
    }
    /// 重置预积分增量、雅可比和协方差
    pub fn reset(&mut self) {
        self.dt = 0.0;
        self.delta_p.fill(0.0);
        self.delta_v.fill(0.0);
        self.delta_r = UnitQuaternion::identity();
        self.mean_acc.fill(0.0);
        self.mean_gyro.fill(0.0);

        // 初始化雅可比为单位阵或零阵
        self.jacobians.pos_ba.fill(0.0);
        self.jacobians.pos_bg.fill(0.0);
        self.jacobians.vel_ba.fill(0.0);
        self.jacobians.vel_bg.fill(0.0);
        self.jacobians.rot_bg.fill(0.0);

        self.covariance.fill(0.0);
        // 初始时偏置的随机游走部分通常保持不变或设为极小值
    }
}

// 计算归一化旋转向量的3x3反对称矩阵
fn hat(v: &Vector3<f32>) -> Matrix3<f32> {
    let mut m = Matrix3::zeros();
    m[(0, 1)] = -v.z;
    m[(0, 2)] = v.y;
    m[(1, 0)] = v.z;
    m[(1, 2)] = -v.x;
    m[(2, 0)] = -v.y;
    m[(2, 1)] = v.x;

    m
}

/// 计算so3旋转向量对应的右雅克比矩阵
fn right_jacobian(v: &Vector3<f32>) -> Matrix3<f32> {
    let phi = v.norm();
    //如果太小，直接返回一个单位矩阵
    let i = Matrix3::identity();
    if phi < 1e-4 {
        return i;
    }

    let w = hat(&v);
    //右雅克比矩阵
    i - w * (1.0 - phi.cos()) / phi.powi(2) + w * w * ((phi - phi.sin()) / phi.powi(3))
}
