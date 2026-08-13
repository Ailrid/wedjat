use csv;
use imu::integrator;
use nalgebra::{UnitQuaternion, Vector3};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GtData {
    timestamp: u64,
    p_x: f32,
    p_y: f32,
    p_z: f32,
    q_w: f32,
    q_x: f32,
    q_y: f32,
    q_z: f32,
    v_x: f32,
    v_y: f32,
    v_z: f32,
    bg_x: f32,
    bg_y: f32,
    bg_z: f32,
    ba_x: f32,
    ba_y: f32,
    ba_z: f32,
}

#[derive(Debug, Deserialize)]
struct ImuData {
    timestamp: u64,
    w_x: f32,
    w_y: f32,
    w_z: f32,
    a_x: f32,
    a_y: f32,
    a_z: f32,
}
#[test]
///imu预积分测试
fn imu_integrator() {
    let imu_csv = "imu.csv";
    let gt_csv = "gt.csv";

    let mut imu_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(imu_csv)
        .unwrap();

    let mut gt_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(gt_csv)
        .unwrap();

    let imu_records: Vec<ImuData> = imu_reader.deserialize().collect::<Result<_, _>>().unwrap();
    let gt_records: Vec<GtData> = gt_reader.deserialize().collect::<Result<_, _>>().unwrap();

    let start_idx = 1000;
    let t_start = gt_records[start_idx].timestamp;
    let t_end = t_start + 500_000_000;

    let gt_start = &gt_records[start_idx];
    let gt_end = gt_records.iter().find(|r| r.timestamp >= t_end).unwrap();

    let ba = Vector3::new(gt_start.ba_x, gt_start.ba_y, gt_start.ba_z);
    let bg = Vector3::new(gt_start.bg_x, gt_start.bg_y, gt_start.bg_z);

    let mut preint = integrator::Preintegration::new(ba, bg, 0.08, 0.004, 0.0001, 0.00001);

    let mut last_ts = 0u64;
    for imu in imu_records
        .iter()
        .filter(|r| r.timestamp >= t_start && r.timestamp <= gt_end.timestamp)
    {
        if last_ts == 0 {
            last_ts = imu.timestamp;
            continue;
        }
        let dt = (imu.timestamp - last_ts) as f32 * 1e-9;
        let acc = Vector3::new(imu.a_x, imu.a_y, imu.a_z);
        let gyro = Vector3::new(imu.w_x, imu.w_y, imu.w_z);
        preint.integrate(&acc, &gyro, dt);
        last_ts = imu.timestamp
    }
    let g_w = Vector3::new(0.0, 0.0, -9.81);
    let dt = preint.dt;

    let p_i = Vector3::new(gt_start.p_x, gt_start.p_y, gt_start.p_z);
    let v_i = Vector3::new(gt_start.v_x, gt_start.v_y, gt_start.v_z);

    let q_i = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
        gt_start.q_w,
        gt_start.q_x,
        gt_start.q_y,
        gt_start.q_z,
    ));

    // 预测公式: Pj = Pi + Vi*Δt + 0.5*g*Δt^2 + Ri * ΔPij
    let p_predict = p_i + v_i * dt + 0.5 * g_w * dt * dt + q_i * preint.delta_p;

    
    // 预测公式: Vj = Vi + g*Δt + Ri * ΔVij
    let v_predict = v_i + g_w * dt + q_i * preint.delta_v;
    // 预测公式: Rj = Ri * ΔRij
    let q_predict = q_i * preint.delta_r;

    // 7. 输出结果对比
    let p_real = Vector3::new(gt_end.p_x, gt_end.p_y, gt_end.p_z);
    let v_real = Vector3::new(gt_end.v_x, gt_end.v_y, gt_end.v_z);
    let q_real = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
        gt_end.q_w, gt_end.q_x, gt_end.q_y, gt_end.q_z,
    ));

    println!("--- Test Result Over {}s ---", dt);
    println!("Pos Error: {:.4} m", (p_predict - p_real).norm());
    println!("Vel Error: {:.4} m/s", (v_predict - v_real).norm());
    // 旋转误差转成角度 (deg)
    let q_error = q_real.inverse() * q_predict;
    println!("Rot Error: {:.4} deg", q_error.angle().to_degrees());
}

#[test]
///imu预积分雅克比矩阵精度测试
fn test_imu_jacobians() {
    let imu_csv = "imu.csv";
    let gt_csv = "gt.csv";

    let mut imu_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(imu_csv)
        .unwrap();

    let mut gt_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(gt_csv)
        .unwrap();

    let imu_records: Vec<ImuData> = imu_reader.deserialize().collect::<Result<_, _>>().unwrap();
    let gt_records: Vec<GtData> = gt_reader.deserialize().collect::<Result<_, _>>().unwrap();

    let start_idx = 1000;
    let t_start = gt_records[start_idx].timestamp;
    let t_end = t_start + 500_000_000;

    let gt_start = &gt_records[start_idx];
    let gt_end = gt_records.iter().find(|r| r.timestamp >= t_end).unwrap();
    // 假设已经读好了 imu_records 和 gt_start

    let ba_nominal = Vector3::new(gt_start.ba_x, gt_start.ba_y, gt_start.ba_z);
    let bg_nominal = Vector3::new(gt_start.bg_x, gt_start.bg_y, gt_start.bg_z);

    // 1. 进行名义状态积分 (Nominal Integration)
    let mut preint_nom =
        integrator::Preintegration::new(ba_nominal, bg_nominal, 0.08, 0.004, 0.0001, 0.00001);

    // 2. 准备扰动状态积分 (Perturbed Integration)
    // 给 Bias 增加一个微小的扰动，例如加速度偏置增加 0.01，陀螺仪偏置增加 0.01
    let delta_ba = Vector3::new(0.01, 0.01, 0.01);
    let delta_bg = Vector3::new(0.01, 0.01, 0.01);

    let mut preint_perturbed = integrator::Preintegration::new(
        ba_nominal + delta_ba,
        bg_nominal + delta_bg,
        0.08,
        0.004,
        0.0001,
        0.00001,
    );

    // 对两份积分器输入完全相同的 IMU 数据
    let mut last_ts = 0u64;
    for imu in imu_records
        .iter()
        .filter(|r| r.timestamp >= t_start && r.timestamp <= gt_end.timestamp)
    {
        if last_ts == 0 {
            last_ts = imu.timestamp;
            continue;
        }
        let dt = (imu.timestamp - last_ts) as f32 * 1e-9;
        let acc = Vector3::new(imu.a_x, imu.a_y, imu.a_z);
        let gyro = Vector3::new(imu.w_x, imu.w_y, imu.w_z);

        preint_nom.integrate(&acc, &gyro, dt);
        preint_perturbed.integrate(&acc, &gyro, dt);

        last_ts = imu.timestamp;
    }

    // 3. 使用雅可比矩阵进行线性预测 (Linear Prediction using Jacobians)
    // 根据公式: Delta_P_new = Delta_P_old + JPa * delta_ba + JPg * delta_bg
    let p_linear_predict = preint_nom.delta_p
        + preint_nom.jacobians.pos_ba * delta_ba
        + preint_nom.jacobians.pos_bg * delta_bg;

    // 根据公式: Delta_V_new = Delta_V_old + JVa * delta_ba + JVg * delta_bg
    let v_linear_predict = preint_nom.delta_v
        + preint_nom.jacobians.vel_ba * delta_ba
        + preint_nom.jacobians.vel_bg * delta_bg;

    // 旋转比较特殊: Delta_R_new = Delta_R_old * Exp(JRg * delta_bg)
    let rot_error_vec = preint_nom.jacobians.rot_bg * delta_bg;
    let q_linear_predict = preint_nom.delta_r * UnitQuaternion::from_scaled_axis(rot_error_vec);

    // 4. 对比 数值积分结果 vs 雅可比线性近似结果
    println!("--- Jacobian Validity Check ---");

    let p_diff = (preint_perturbed.delta_p - p_linear_predict).norm();
    println!("Pos Jacobian Error (Linear vs Numerical): {:.6} m", p_diff);

    let v_diff = (preint_perturbed.delta_v - v_linear_predict).norm();
    println!(
        "Vel Jacobian Error (Linear vs Numerical): {:.6} m/s",
        v_diff
    );

    let q_diff = (preint_perturbed.delta_r.inverse() * q_linear_predict).angle();
    println!(
        "Rot Jacobian Error (Linear vs Numerical): {:.6} deg",
        q_diff.to_degrees()
    );

    // 判定标准：因为是 0.5s 的短程积分，线性近似和实际积分的差异应该极小（通常在 1e-4 或更小量级）
    assert!(p_diff < 1e-3, "Position Jacobian might be wrong!");
    assert!(
        q_diff.to_degrees() < 1e-2,
        "Rotation Jacobian might be wrong!"
    );
}

#[test]
fn test_covariance_consistency() {
    use rand::rngs::StdRng;
    use rand::{SeedableRng};
    use rand_distr::{Distribution, Normal};

    // ---------------------------------------------------
    let imu_csv = "imu.csv";
    let gt_csv = "gt.csv";
    let mut imu_reader = csv::ReaderBuilder::new().has_headers(true).from_path(imu_csv).unwrap();
    let mut gt_reader = csv::ReaderBuilder::new().has_headers(true).from_path(gt_csv).unwrap();
    let imu_records: Vec<ImuData> = imu_reader.deserialize().collect::<Result<_, _>>().unwrap();
    let gt_records: Vec<GtData> = gt_reader.deserialize().collect::<Result<_, _>>().unwrap();
    // ---------------------------------------------------

    let start_idx = 1000;
    let t_start = gt_records[start_idx].timestamp;
    let t_end = t_start + 500_000_000; // 0.5s
    let gt_start = &gt_records[start_idx];
    let gt_end = gt_records.iter().find(|r| r.timestamp >= t_end).unwrap();

    let ba = Vector3::new(gt_start.ba_x, gt_start.ba_y, gt_start.ba_z);
    let bg = Vector3::new(gt_start.bg_x, gt_start.bg_y, gt_start.bg_z);

    // 2. 噪声参数 
    // 注意：这里假设参数是 Discrete StdDev
    let acc_n = 0.08;   
    let gyr_n = 0.004;  
    let acc_w = 0.0001; 
    let gyr_w = 0.00001;

    let mut rng = StdRng::seed_from_u64(42); // 固定种子以复现结果
    let normal = Normal::new(0.0, 1.0).unwrap();

    // 3. 计算名义状态 (Nominal) - 无噪声
    let mut preint_nom = integrator::Preintegration::new(ba, bg, acc_n, gyr_n, acc_w, gyr_w);
    let mut last_ts = 0u64;
    for imu in imu_records.iter().filter(|r| r.timestamp >= t_start && r.timestamp <= gt_end.timestamp) {
        if last_ts == 0 { last_ts = imu.timestamp; continue; }
        let dt = (imu.timestamp - last_ts) as f32 * 1e-9;
        preint_nom.integrate(&Vector3::new(imu.a_x, imu.a_y, imu.a_z), &Vector3::new(imu.w_x, imu.w_y, imu.w_z), dt);
        last_ts = imu.timestamp;
    }

    // 4. 蒙特卡洛循环
    let mut sample_deltas_p = Vec::new();
    let mut sample_deltas_v = Vec::new();
    let mut sample_errors_phi = Vec::new(); // 存储旋转误差向量

    let num_samples = 1000; // 增加样本数以提高统计稳定性
    println!("Running {} Monte Carlo samples...", num_samples);

    for _ in 0..num_samples {
        let mut preint = integrator::Preintegration::new(ba, bg, acc_n, gyr_n, acc_w, gyr_w);
        let mut last_ts = 0u64;

        for imu in imu_records.iter().filter(|r| r.timestamp >= t_start && r.timestamp <= gt_end.timestamp) {
            if last_ts == 0 { last_ts = imu.timestamp; continue; }
            let dt = (imu.timestamp - last_ts) as f32 * 1e-9;

            let noisy_acc = Vector3::new(
                imu.a_x + normal.sample(&mut rng) * acc_n,
                imu.a_y + normal.sample(&mut rng) * acc_n,
                imu.a_z + normal.sample(&mut rng) * acc_n,
            );
            let noisy_gyro = Vector3::new(
                imu.w_x + normal.sample(&mut rng) * gyr_n,
                imu.w_y + normal.sample(&mut rng) * gyr_n,
                imu.w_z + normal.sample(&mut rng) * gyr_n,
            );

            preint.integrate(&noisy_acc, &noisy_gyro, dt);
            last_ts = imu.timestamp;
        }
        
        // 收集误差
        // 位置和速度误差可以直接做差 (因为是在 start_frame 下的向量)
        sample_deltas_p.push(preint.delta_p - preint_nom.delta_p);
        sample_deltas_v.push(preint.delta_v - preint_nom.delta_v);

        // *** 修正点: 旋转误差必须是在切空间 (Tangent Space) 的差异 ***
        // Error = Log( R_nom^T * R_noisy )
        let delta_r_err = preint_nom.delta_r.inverse() * preint.delta_r;
        sample_errors_phi.push(delta_r_err.scaled_axis());
    }

    // 5. 统计样本方差
    // 这里的 samples 已经是误差值了 (x - mean)，所以直接算 E[x*x^T]
    let calc_var_from_errors = |errors: &Vec<Vector3<f32>>| -> Vector3<f32> {
        let n = errors.len() as f32;
        errors.iter().map(|e| {
            Vector3::new(e.x.powi(2), e.y.powi(2), e.z.powi(2))
        }).sum::<Vector3<f32>>() / (n - 1.0)
    };

    let var_p_stat = calc_var_from_errors(&sample_deltas_p);
    let var_v_stat = calc_var_from_errors(&sample_deltas_v);
    let var_phi_stat = calc_var_from_errors(&sample_errors_phi);

    // 6. 理论协方差
    let cov_theory = preint_nom.covariance;
    let var_r_theory = cov_theory.fixed_view::<3, 3>(0, 0).diagonal();
    let var_v_theory = cov_theory.fixed_view::<3, 3>(3, 3).diagonal();
    let var_p_theory = cov_theory.fixed_view::<3, 3>(6, 6).diagonal();

    // 7. 对比输出
    println!("\n--- Covariance Consistency Check (0.5s) ---");
    println!("Position Variance (m^2):");
    println!("  Stat:   {:.6}, {:.6}, {:.6}", var_p_stat.x, var_p_stat.y, var_p_stat.z);
    println!("  Theory: {:.6}, {:.6}, {:.6}", var_p_theory.x, var_p_theory.y, var_p_theory.z);
    
    println!("Velocity Variance (m/s^2):");
    println!("  Stat:   {:.6}, {:.6}, {:.6}", var_v_stat.x, var_v_stat.y, var_v_stat.z);
    println!("  Theory: {:.6}, {:.6}, {:.6}", var_v_theory.x, var_v_theory.y, var_v_theory.z);

    println!("Rotation Variance (rad^2):");
    println!("  Stat:   {:.6}, {:.6}, {:.6}", var_phi_stat.x, var_phi_stat.y, var_phi_stat.z);
    println!("  Theory: {:.6}, {:.6}, {:.6}", var_r_theory.x, var_r_theory.y, var_r_theory.z);

    // 验证逻辑: 检查比值是否在合理范围 (例如 0.7 ~ 1.3)
    // 注意: 随机过程有波动，特别是样本数只有500时，放宽到 0.5 ~ 2.0
    let check_ratio = |stat: f32, theory: f32, name: &str| {
        let ratio = stat / theory;
        println!("{}: Ratio = {:.2}", name, ratio);
        assert!(ratio > 0.5 && ratio < 2.0, "{} covariance mismatch!", name);
    };

    check_ratio(var_p_stat.norm(), var_p_theory.norm(), "Pos Norm");
    check_ratio(var_v_stat.norm(), var_v_theory.norm(), "Vel Norm");
    check_ratio(var_phi_stat.norm(), var_r_theory.norm(), "Rot Norm");
}