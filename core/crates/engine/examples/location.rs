use map_matching::estimator::Estimator;
use map_matching::extractor::types::ExtractorCfg;
use map_matching::location::Location;
use map_matching::matcher::types::MatcherCfg;
use map_matching::types::FramePriori;
use opencv::core::MatTraitConst;
use opencv::imgcodecs::{IMREAD_COLOR, imread};
use serde::Serialize;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

#[derive(Serialize)]
struct FinalResult {
    frame_id: usize,
    x: f64,
    y: f64,
    z: f64,
}

pub fn run_location_batch_test(
    img_dir: &str,
    output_json: &str,
    m_cfg: MatcherCfg,
    e_cfg: ExtractorCfg,
) -> anyhow::Result<()> {
    // 1. 初始化定位模块
    let mut locator = Location::new(m_cfg, e_cfg).map_err(|e| anyhow::anyhow!(e))?;

    // 2. 获取并排序所有图片路径
    let mut entries: Vec<_> = fs::read_dir(img_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "png"))
        .collect();

    // 按照文件名里的数字进行排序 (例如 "1.png", "2.png"...)
    entries.sort_by_key(|e| {
        e.file_name()
            .to_string_lossy()
            .trim_end_matches(".png")
            .parse::<usize>()
            .unwrap_or(0)
    });

    let mut results = Vec::new();

    let mut estimator = Estimator::new(20.0, 150.0);
    let mut global_priori = FramePriori::default();
    // 3. 循环处理
    let timer = Instant::now();
    let total_len = entries.len();
    for entry in entries.into_iter() {
        let path = entry.path();
        let frame_id = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".png")
            .parse::<usize>()?;

        println!("正在处理帧: {}", frame_id);

        // 读取图片
        let img = imread(path.to_str().unwrap(), IMREAD_COLOR)?;
        if img.empty() {
            continue;
        }

        // 这里需要一个先验信息
        let priori = global_priori.clone();

        // 执行定位
        let start_time: Instant = Instant::now();
        if global_priori.crs.is_empty() {
            match locator.frame_location(&img, start_time, frame_id, priori) {
                Ok(predict_points) => {
                    estimator.update(predict_points, None);
                    if let Some(estimated_point) = estimator.estimate() {
                        results.push(FinalResult {
                            frame_id,
                            x: estimated_point.x,
                            y: estimated_point.y,
                            z: estimated_point.z,
                        });

                        global_priori = FramePriori {
                            utm_x: estimated_point.utm_x as f32,
                            utm_y: estimated_point.utm_y as f32,
                            crs: estimated_point.crs,
                            radius: 150.0,
                            k: 5,
                            pixel_x: estimated_point.pixel_x,
                            pixel_y: estimated_point.pixel_y,
                            src: estimated_point.src,
                        };
                    } else {
                        results.push(FinalResult {
                            frame_id,
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        });
                        println!("帧 {:?} 无法估计结果,不确定当前位置", frame_id);
                    }
                }
                Err(e) => {
                    global_priori = FramePriori::default();
                    eprintln!("帧 {} 定位失败: {:?}", frame_id, e);
                }
            }
        } else {
            //查询抠图
            let sat_img =
                self.matcher
                    .crop_pos(item.pixel_x, item.pixel_y, 256, item.src.clone())?;
            //计算每个图像的单应性矩阵
            let result = self.extractor.homography_matrix(drone_img, &sat_img);
            match result {
                Ok((_h_mat, inlier_count, center_point)) => {
                    // 能得到单应性矩阵结果的话，就保存这张图像的数据
                    // 计算这个中心点反演之后的位置坐标在utm下的坐标
                    let utm_x = item.utm_x + (center_point.x * item.res[0]);
                    let utm_y = item.utm_y - (center_point.y * item.res[1]);
                    // 转换为WGPS下的空间直角坐标系
                    let pipeline = format!("inv utm zone={} | cart", item.utm_zone);
                    let op = self.minimal.op(&pipeline)?;
                    // Coor3D(Easting, Northing, Height)
                    let mut data = [Coor3D::raw(utm_x as f64, utm_y as f64, 0.0)];
                    // 执行转换到空间直角坐标系
                    self.minimal.apply(op, Fwd, &mut data)?;
                    let ecef_center = data[0].0;
                    // 保存这帧的位置
                    frame_pos.push(PredictPoint {
                        x: ecef_center[0],
                        y: ecef_center[1],
                        z: ecef_center[2],
                        utm_x: utm_x as f64,
                        utm_y: utm_y as f64,
                        crs: item.crs.clone(),
                        pixel_x: center_point.x as f64,
                        pixel_y: center_point.y as f64,
                        src: item.src.clone(),
                        time: time,
                        frame_id: frame_id,
                        inlier_count: inlier_count,
                        score: item.score,
                    });
                }
                Err(e) => {
                    // 计算单应性矩阵错误表明不可能匹配，直接丢掉
                    // tracing::warn!("Homography matrix error:{:?}", e);
                    println!("Homography matrix error:{:?}", e)
                }
            }
        }
    }

    println!("耗时: {:?}", timer.elapsed());
    // 输出平均fps
    println!(
        "平均FPS: {}",
        total_len as f64 / timer.elapsed().as_secs_f64()
    );

    // 4. 保存为 JSON
    let json_data = serde_json::to_string_pretty(&results)?;
    let mut file = File::create(output_json)?;
    file.write_all(json_data.as_bytes())?;

    println!("✔ 所有结果已保存至: {}", output_json);
    Ok(())
}

fn main() {
    //初始化tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true) // 是否显示代码模块路径
        .init();

    let img_dir = "/home/shiraha_yuki/文档/deeplearning/database/rgb/3cm";
    let output_json = "output.json";
    let m_cfg = MatcherCfg {
        model_path: "assets/twinnet_inference.onnx".to_string(),
        backend_type: "onnx".to_string(),
        threads: 10,
        device: "cuda".to_string(),
        ..Default::default()
    };
    let e_cfg = ExtractorCfg {
        model_path: "assets/superpoint_lightglue_pipeline.onnx".to_string(),
        backend_type: "onnx".to_string(),
        threads: 10,
        device: "cuda".to_string(),
        ..Default::default()
    };

    run_location_batch_test(img_dir, output_json, m_cfg, e_cfg).unwrap();
}
