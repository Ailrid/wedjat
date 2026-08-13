/*
 * @Author: ShirahaYuki  shirhayuki2002@gmail.com
 * @Date: 2026-01-14 09:44:27
 * @LastEditors: ShirahaYuki  shirhayuki2002@gmail.com
 * @LastEditTime: 2026-01-19 12:07:44
 * @FilePath: /map_matching/src/matcher/types.rs
 * @Description:设定和服务器发送和响应的类型
 *
 * Copyright (c) 2026 by ShirahaYuki, All Rights Reserved.
 */
use serde::{Deserialize, Serialize};

fn deault_radius() -> f32 {
    500.0
}
fn default_k() -> usize {
    2
}

#[derive(Serialize, Deserialize)]
pub struct SearchRequest {
    // 特征向量
    pub vector: Vec<f32>,
    // 指定的查询原点和带号
    #[serde(default)]
    pub utm_x: f32,
    #[serde(default)]
    pub utm_y: f32,
    #[serde(default)]
    pub crs: String,
    // 指定的查询半径
    #[serde(default = "deault_radius")]
    pub radius: f32,
    // 返回的匹配结果数量
    #[serde(default = "default_k")]
    pub k: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchItem {
    // 匹配得分
    pub score: f32,
    // 匹配图像的中心坐标数据和带号、坐标系
    pub utm_x: f32,
    pub utm_y: f32,
    pub utm_zone: i32,
    pub crs: String,
    // 匹配图像的像素坐标数据（左上角坐标）
    pub pixel_x: usize,
    pub pixel_y: usize,
    // 匹配图像的分辨率数据[f32,f32]
    pub res: Vec<f32>,
    // 匹配到的图像的文件名
    pub src: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResponse {
    // 匹配结果列表
    pub items: Vec<SearchItem>,
}

#[derive(Serialize, Deserialize)]
pub struct CropRequest {
    pub pixel_x: usize,
    pub pixel_y: usize,
    pub patch_size: usize,
    pub src: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MatcherCfg {
    // 后端类型,onnx或者vino
    pub backend_type: String,
    pub model_path: String,
    // 线程数
    pub threads: usize,
    // 设备类型,CPU / GPU / NPU
    pub device: String,
    // 向量查询服务器地址
    pub client_addr: String,
}
impl Default for MatcherCfg {
    fn default() -> Self {
        Self {
            backend_type: "onnx".to_string(),
            model_path: "".to_string(),
            threads: 4,
            device: "CPU".to_string(),
            client_addr: "http://127.0.0.1:8000".to_string(),
        }
    }
}
