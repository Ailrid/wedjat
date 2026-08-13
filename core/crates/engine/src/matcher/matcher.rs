/*
 * @Author: ShirahaYuki  shirhayuki2002@gmail.com
 * @Date: 2026-01-15 15:03:13
 * @LastEditors: ShirahaYuki  shirhayuki2002@gmail.com
 * @LastEditTime: 2026-01-19 11:58:51
 * @FilePath: /map_matching/src/matcher/matcher.rs
 * @Description: 地图匹配模块，负责查询地图数据库并返回匹配结果还有抠图
 *
 * Copyright (c) 2026 by ShirahaYuki, All Rights Reserved.
 */
use crate::{
    matcher::{
        backend::{onnx_backend::OnnxBackend, vino_backend::OpenVinoBackend},
        errors::MatcherError,
        traits::MatcherBackend,
        types::{CropRequest, MatcherCfg, SearchRequest, SearchResponse},
        vector_client::VectorClient,
    },
    types::FramePriori,
};

use opencv::prelude::*;
use tracing::info;

pub struct Matcher {
    backend: Box<dyn MatcherBackend>,
    client: VectorClient,
}

impl Matcher {
    #[tracing::instrument(level = "info")]
    pub fn new(cfg: MatcherCfg) -> Result<Self, MatcherError> {
        info!("▶Creating Matcher");
        let backend: Box<dyn MatcherBackend> = match cfg.backend_type.as_str() {
            "onnx" => {
                let onnx_backend = OnnxBackend::new(
                    ort::session::builder::GraphOptimizationLevel::Level3,
                    cfg.threads,
                    &cfg.model_path,
                )?;
                Box::new(onnx_backend)
            }
            "vino" => {
                let vino_backend =
                    OpenVinoBackend::new(&cfg.model_path, "", cfg.device.as_str().into())?;
                Box::new(vino_backend)
            }
            _ => {
                return Err(MatcherError::ModelTypeError(cfg.backend_type.to_string()));
            }
        };
        let client = VectorClient::new(cfg.client_addr);
        info!("✔Creating Matcher has been completed");
        Ok(Self { backend, client })
    }

    pub fn query_img(
        &mut self,
        drone_img: &Mat,
        frame_priori: FramePriori,
    ) -> Result<SearchResponse, MatcherError> {
        // 先拿图像处理得到特征向量
        let feature_vec = self.backend.forword(drone_img)?;
        //到数据库里查询
        let search_res = self.client.search(SearchRequest {
            vector: feature_vec,
            utm_x: frame_priori.utm_x,
            utm_y: frame_priori.utm_y,
            crs: frame_priori.crs,
            radius: frame_priori.radius,
            k: frame_priori.k,
        })?;
        Ok(search_res)
    }
    pub fn crop_pos(
        &mut self,
        pixel_x: usize,
        pixel_y: usize,
        patch_size: usize,
        src: String,
    ) -> Result<Mat, MatcherError> {
        let crop_res = self.client.crop(CropRequest {
            pixel_x,
            pixel_y,
            patch_size,
            src,
        })?;
        Ok(crop_res)
    }
}
