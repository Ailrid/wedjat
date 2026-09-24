//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core

use opencv::core::Mat;

pub mod trigger;
pub mod camera;

#[derive(Debug, Clone)]
pub struct Frame {
    pub image: Mat,
    pub time: i64,
    pub flight_height: f32,
    pub yaw: Option<f32>, // Yaw angle relative to the north direction, angle system, clockwise direction is positive
    pub relative_move: Option<f32>, // The displacement given by other sensors relative to the last photo taken
}
