//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core
use thiserror::Error;
#[derive(Error, Debug)]
pub enum RknnError {
    #[error("Operation failed (RKNN3_ERR_FAIL)")]
    Fail,

    #[error("Invalid argument (RKNN3_ERR_ARGUMENT_INVALID)")]
    ArgumentInvalid,

    #[error("Invalid model (RKNN3_ERR_MODEL_INVALID)")]
    ModelInvalid,

    #[error("Invalid context (RKNN3_ERR_CTX_INVALID)")]
    CtxInvalid,

    #[error("Run task failed (RKNN3_ERR_RUN_TASK_FAILED)")]
    RunTaskFailed,

    #[error("Out of memory (RKNN3_ERR_OUT_OF_MEMORY)")]
    OutOfMemory,

    #[error("Operation timed out (RKNN3_ERR_TIMEOUT)")]
    Timeout,

    #[error("Invalid input (RKNN3_ERR_INPUT_INVALID)")]
    InputInvalid,

    #[error("Invalid output (RKNN3_ERR_OUTPUT_INVALID)")]
    OutputInvalid,

    #[error("Device unavailable (RKNN3_ERR_DEVICE_UNAVAILABLE)")]
    DeviceUnavailable,

    #[error("Device unmatch (RKNN3_ERR_DEVICE_UNMATCH)")]
    DeviceUnmatch,

    #[error("Target platform unmatch (RKNN3_ERR_TARGET_PLATFORM_UNMATCH)")]
    TargetPlatformUnmatch,

    #[error("Communication error (RKNN3_ERR_COMMUNICATION)")]
    Communication,

    #[error("Memory sync failed (RKNN3_ERR_MEM_SYNC_FAILED)")]
    MemSyncFailed,

    #[error("Invalid key (RKNN3_ERR_KEY_INVALID)")]
    KeyInvalid,

    #[error("Decrypt failed (RKNN3_ERR_DECRYPT_FAILED)")]
    DecryptFailed,

    #[error("Weight load not prepared (RKNN3_ERR_WEIGHT_LOAD_NOT_PREPARED)")]
    WeightLoadNotPrepared,

    #[error("Incompatible version (RKNN3_ERR_INCOMPATIBLE_VERSION)")]
    IncompatibleVersion,

    #[error("Other error {0}")]
    OtherError(String),

    #[error("Unknown RKNN error code: {0}")]
    Unknown(i32),
}

impl RknnError {
    pub fn check(code: i32) -> Result<(), Self> {
        match code {
            0 => Ok(()),
            -1 => Err(Self::Fail),
            -2 => Err(Self::ArgumentInvalid),
            -3 => Err(Self::ModelInvalid),
            -4 => Err(Self::CtxInvalid),
            -5 => Err(Self::RunTaskFailed),
            -6 => Err(Self::OutOfMemory),
            -7 => Err(Self::Timeout),
            -8 => Err(Self::InputInvalid),
            -9 => Err(Self::OutputInvalid),
            -10 => Err(Self::DeviceUnavailable),
            -11 => Err(Self::DeviceUnmatch),
            -12 => Err(Self::TargetPlatformUnmatch),
            -13 => Err(Self::Communication),
            -14 => Err(Self::MemSyncFailed),
            -15 => Err(Self::KeyInvalid),
            -16 => Err(Self::DecryptFailed),
            -17 => Err(Self::WeightLoadNotPrepared),
            -18 => Err(Self::IncompatibleVersion),
            other => Err(Self::Unknown(other)),
        }
    }
}

pub trait RknnResultExt {
    fn check_rknn(self) -> Result<(), RknnError>;
}

impl RknnResultExt for i32 {
    fn check_rknn(self) -> Result<(), RknnError> {
        RknnError::check(self)
    }
}
