use crate::config::Config;
use client::qdrant::{QdrantClient, QdrantResult};
use client::tiff::TiffClient;

use camera::Frame;
use client::qdrant::QdrantClientError;
use client::tiff::TiffClientError;

use engine::error::BackendError;
use engine::estimator::{Estimator, EstimatorConfig};
use engine::estimator::{FrameLocation, FramePoint, LocationResult};
use engine::lightglue::{LightGlue, LightGlueOutput};
use engine::metric::Metric;
use engine::superpoint::{SuperPoint, SuperPointOutput};
use opencv::calib3d::{RANSAC, find_homography};
use opencv::core::{Mat, MatTraitConst, Point2f, Vector, count_non_zero, perspective_transform};

use thiserror::Error;

const CROP_SIZE: u32 = 256;

#[derive(Error, Debug)]
pub enum WedjatError {
    #[error("Qdrant client error: {0}")]
    QdrantClientError(#[from] QdrantClientError),
    #[error("Tiff client error: {0}")]
    TiffClientError(#[from] TiffClientError),
    #[error("Backend error: {0}")]
    BackendError(#[from] BackendError),
    #[error("Opencv error: {0}")]
    OpenCVError(#[from] opencv::Error),
    #[error("Other error: {0}")]
    OtherError(String),
}

pub enum SystemState {
    Init,
    Tracking,
}

pub struct Core {
    state: SystemState,
    qdrant: QdrantClient,
    tiff: TiffClient,
    superpoint: SuperPoint,
    lightglue: LightGlue,
    metric: Metric,
    estimator: Estimator,
    pre_location: Option<FrameLocation>,
}

impl Core {
    pub fn new(config: Config) -> Result<Self, WedjatError> {
        Ok(Self {
            state: SystemState::Init,
            qdrant: QdrantClient::new(
                config.qdrant_url,
                config.qdrant_port,
                config.qdrant_collection_name,
                config.qdrant_query_limit,
                config.qdrant_query_radius,
            )?,
            tiff: TiffClient::new(config.tiff_folder),
            superpoint: SuperPoint::new(
                config.device_id.clone(),
                config.superpoint_rknn_path,
                config.superpoint_weight_path,
                config.num_keypoints,
                config.nms_radius,
                config.remove_borders,
            )?,
            lightglue: LightGlue::new(
                config.device_id.clone(),
                config.lightglue_rknn_path,
                config.lightglue_weight_path,
                config.match_threshold,
            )?,
            metric: Metric::new(
                config.device_id,
                config.metric_rknn_path,
                config.metric_weight_path,
            )?,
            estimator: Estimator::new(EstimatorConfig {
                hover_threshold: config.hover_threshold,
                init_hits: config.init_hits,
                dead_frames: config.dead_frames,
                match_distance_cutoff: config.match_distance_cutoff,
                nms_radius_meters: config.nms_radius_meters,
                min_span_required: config.min_span_required,
            }),
            pre_location: None,
        })
    }

    pub fn crop_img(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        tiff_name: impl Into<String>,
    ) -> Result<Mat, WedjatError> {
        Ok(self.tiff.crop(x, y, width, height, tiff_name)?)
    }

    pub fn infer_features(&mut self, img: &Mat) -> Result<Vec<f32>, WedjatError> {
        Ok(self.metric.infer(img)?)
    }

    pub fn query_global(&mut self, feature: Vec<f32>) -> Result<Vec<QdrantResult>, WedjatError> {
        Ok(self.qdrant.query_global(feature)?)
    }

    pub fn query_filter(&mut self, feature: Vec<f32>) -> Result<Vec<QdrantResult>, WedjatError> {
        let res = match &self.pre_location {
            Some(location) => {
                self.qdrant
                    .query_filter(feature, location.center.lon, location.center.lat)
            }
            None => self.qdrant.query_global(feature),
        }?;
        Ok(res)
    }

    pub fn refer_points(&mut self, img: &Mat) -> Result<SuperPointOutput, WedjatError> {
        Ok(self.superpoint.infer(img)?)
    }

    pub fn refer_matches(
        &mut self,
        point1: &SuperPointOutput,
        point2: &SuperPointOutput,
    ) -> Result<LightGlueOutput, WedjatError> {
        Ok(self.lightglue.infer(point1, point2)?)
    }

    pub fn possible_tiles(
        &mut self,
        frame: &Frame,
    ) -> Result<Vec<(Mat, QdrantResult)>, WedjatError> {
        match self.state {
            SystemState::Init => {
                let frame_feature = self.infer_features(&frame.image)?;
                let search = self.query_filter(frame_feature)?;
                let mut imgs = Vec::new();
                for item in &search {
                    let img = self.crop_img(
                        item.payload.x as u32,
                        item.payload.y as u32,
                        CROP_SIZE,
                        CROP_SIZE,
                        item.payload.src.clone(),
                    )?;
                    imgs.push(img);
                }
                Ok(imgs
                    .into_iter()
                    .zip(search)
                    .map(|(img, item)| (img, item))
                    .collect())
            }
            SystemState::Tracking => match &self.pre_location {
                Some(location) => {
                    let x = location.center.pixel_x.saturating_sub(CROP_SIZE / 2);
                    let y = location.center.pixel_y.saturating_sub(CROP_SIZE / 2);

                    let crop_img = self.crop_img(
                        x,
                        y,
                        CROP_SIZE,
                        CROP_SIZE,
                        location.item.payload.src.clone(),
                    )?;
                    Ok(vec![(crop_img, location.item.clone())])
                }
                None => {
                    // 丢失位置时重新从全局开始检索...
                    self.state = SystemState::Init;
                    self.possible_tiles(frame)
                }
            },
        }
    }

    pub fn locate(&mut self, frame: Frame) -> Result<LocationResult, WedjatError> {
        let tiles = self.possible_tiles(&frame)?;
        let frame_points = self.refer_points(&frame.image)?;

        let mut frame_matches = Vec::new();

        for (img, item) in tiles {
            let tile_points = self.refer_points(&img)?;
            let matches = self.refer_matches(&frame_points, &tile_points)?;

            if matches.matches.len() < 20 {
                continue;
            }
            frame_matches.push((tile_points.keypoints, matches.matches, item));
        }

        // 如果没有一个 tile 达到匹配阈值，定位失败，切回 Init
        if frame_matches.is_empty() {
            self.state = SystemState::Init;
            self.pre_location = None;
            return Err(WedjatError::OtherError(
                "No tiles met the minimum match threshold (20 points)".into(),
            ));
        }

        let frame_w = frame.image.cols() as f32;
        let frame_h = frame.image.rows() as f32;
        let mut frame_project: Vec<FramePoint> = Vec::new();

        for (keypoints, matches, item) in frame_matches {
            let mut src_pts = Vector::<Point2f>::new();
            let mut dst_pts = Vector::<Point2f>::new();
            src_pts.reserve(matches.len());
            dst_pts.reserve(matches.len());

            for m in matches {
                let src_idx = m.0 as usize;
                let dst_idx = m.1 as usize;

                let pt_src = frame_points.keypoints[src_idx];
                let pt_dst = keypoints[dst_idx];

                src_pts.push(Point2f::new(pt_src[0], pt_src[1]));
                dst_pts.push(Point2f::new(pt_dst[0], pt_dst[1]));
            }

            let mut mask = Mat::default();

            let homography = find_homography(&src_pts, &dst_pts, &mut mask, RANSAC, 10.0);

            if let Ok(h_matrix) = homography {
                let center_src = Point2f::new(frame_w / 2.0, frame_h / 2.0);
                let mut src_center_vec = Vector::<Point2f>::new();
                let mut dst_center_vec = Vector::<Point2f>::new();
                src_center_vec.push(center_src);

                perspective_transform(&src_center_vec, &mut dst_center_vec, &h_matrix)?;

                let inliers = count_non_zero(&mask)? as usize;
                let local_tile_center = dst_center_vec.get(0)?;

                frame_project.push(FramePoint::new(
                    inliers,
                    local_tile_center,
                    &h_matrix,
                    item,
                )?)
            } else {
                continue;
            }
        }

        if frame_project.is_empty() {
            return Ok(LocationResult::Uncertain(
                "Unable to find any matching tiles".into(),
            ));
        }

        Ok(self.estimator.update_frame(frame_project, &frame))
    }
}
