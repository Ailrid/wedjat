//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core

use opencv::{
    Result,
    core::Mat,
    prelude::*,
    videoio::{
        CAP_ANY, CAP_FFMPEG, CAP_PROP_BUFFERSIZE, CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT,
        CAP_PROP_FRAME_WIDTH, VideoCapture,
    },
};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CameraError {
    #[error("Camera error: {0}")]
    CameraError(String),
    #[error("OpenCV Error: {0}")]
    OpenCVError(#[from] opencv::Error),
    #[error("Time Error: {0}")]
    TimeError(#[from] std::time::SystemTimeError),
}

pub struct FrameData {
    pub image: Mat,
    pub timestamp: f64,
}

pub struct USBCamera {
    cap: VideoCapture,
    device_index: i32,
}

impl USBCamera {
    pub fn new(
        device_index: i32,
        width: usize,
        height: usize,
        fps: usize,
    ) -> Result<Self, CameraError> {
        let mut cap = VideoCapture::new(device_index, CAP_ANY)?;

        if !VideoCapture::is_opened(&cap)? {
            return Err(CameraError::OpenCVError(opencv::Error::new(
                opencv::core::StsError,
                format!("Failed to open camera device at index {}", device_index),
            )));
        }

        // Configure camera parameters
        cap.set(CAP_PROP_FRAME_WIDTH, width as f64)?;
        cap.set(CAP_PROP_FRAME_HEIGHT, height as f64)?;
        cap.set(CAP_PROP_FPS, fps as f64)?;

        // Set the internal buffer to 1 to ensure that the read_frame blocks and always retrieves the latest image
        let _ = cap.set(CAP_PROP_BUFFERSIZE, 1.0);
        Ok(Self { cap, device_index })
    }

    /// Blocking reading an image.
    pub fn read_frame(&mut self) -> Result<FrameData, CameraError> {
        let mut temp_mat = Mat::default();

        // Record high-precision timestamps
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let timestamp = now.as_secs() as f64 + (now.subsec_nanos() as f64 * 1e-9);

        // Block capturing one frame of image
        let success = self.cap.read(&mut temp_mat)?;

        if !success || temp_mat.empty() {
            return Err(CameraError::OpenCVError(opencv::Error::new(
                opencv::core::StsError,
                format!("Failed to read frame from camera {}", self.device_index),
            )));
        }
        let image = temp_mat.try_clone()?;
        Ok(FrameData { image, timestamp })
    }

    /// Retrieve the device index number corresponding to the current camera
    pub fn device_index(&self) -> i32 {
        self.device_index
    }
}

pub struct EthCamera {
    cap: VideoCapture,
    rtsp_url: String,
}

impl EthCamera {
    pub fn new(rtsp_url: &str) -> Result<Self, CameraError> {
        let low_latency_url = format!("{}?tcp&fflags=nobuffer&max_delay=500000", rtsp_url);

        let mut cap = VideoCapture::from_file(&low_latency_url, CAP_FFMPEG)?;

        if !VideoCapture::is_opened(&cap)? {
            return Err(CameraError::CameraError(format!(
                "Failed to open RTSP stream at URL: {}",
                rtsp_url
            )));
        }
        let _ = cap.set(CAP_PROP_BUFFERSIZE, 1.0);
        Ok(Self {
            cap,
            rtsp_url: rtsp_url.to_string(),
        })
    }

    pub fn read_frame(&mut self) -> Result<FrameData, CameraError> {
        let mut temp_mat = Mat::default();

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let timestamp = now.as_secs() as f64 + (now.subsec_nanos() as f64 * 1e-9);

        let success = self.cap.read(&mut temp_mat)?;

        if !success || temp_mat.empty() {
            return Err(CameraError::CameraError(format!(
                "Failed to read or decode frame from RTSP stream: {}",
                self.rtsp_url
            )));
        }

        let image = temp_mat.try_clone()?;
        Ok(FrameData { image, timestamp })
    }

    pub fn rtsp_url(&self) -> &str {
        &self.rtsp_url
    }
}
