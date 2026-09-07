//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core
use super::error::*;
use rknn_sys::{
    _rknn3_mem_alloc_flags_RKNN3_FLAG_MEMORY_CACHEABLE,
    _rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_FROM_DEVICE,
    _rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_TO_DEVICE, _rknn3_query_cmd_RKNN3_QUERY_IN_OUT_NUM,
    _rknn3_query_cmd_RKNN3_QUERY_INPUT_ATTR, _rknn3_query_cmd_RKNN3_QUERY_OUTPUT_ATTR,
    _rknn3_tensor_memory, rknn3_config, rknn3_context, rknn3_destroy, rknn3_devices,
    rknn3_find_devices, rknn3_init, rknn3_init_extend, rknn3_input_output_num,
    rknn3_load_model_from_path, rknn3_mem_sync, rknn3_query, rknn3_run, rknn3_tensor,
    rknn3_tensor_attr,
};
use rknn_sys::{rknn3_create_mem, rknn3_model_init};
use std::ffi::{CStr, CString};

pub struct Context {
    pub context: rknn3_context,
}

impl Context {
    /// find all devices
    pub fn find_devices() -> Result<Vec<String>, RknnError> {
        let mut devs: rknn3_devices = unsafe { std::mem::zeroed() };
        let ret = unsafe { rknn3_find_devices(&mut devs) };
        RknnError::check(ret)?;

        let mut device_ids = Vec::with_capacity(devs.n_devices as usize);

        for i in 0..devs.n_devices as usize {
            let ptr = devs.devices[i].id.as_ptr();

            let id = unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned();

            println!("Device {}: {}", i, id);
            device_ids.push(id);
        }

        Ok(device_ids)
    }
    // create context with device id
    pub fn with_device(device_id: impl Into<String>) -> Result<Self, RknnError> {
        Context::find_devices()?;
        let mut raw_ctx: rknn3_context = 0;
        let c_device_id = CString::new(device_id.into()).map_err(|_| RknnError::ArgumentInvalid)?;

        let mut init_extend = rknn3_init_extend {
            device_id: c_device_id.as_ptr() as *mut _,
            reserved: [0; 128],
        };
        let ret = unsafe { rknn3_init(&mut raw_ctx, &mut init_extend) };
        RknnError::check(ret)?;
        Ok(Self { context: raw_ctx })
    }
    /// load model
    pub fn load_model_from_path(
        &self,
        model_path: impl Into<String>,
        weight_path: impl Into<String>,
    ) -> Result<(), RknnError> {
        let c_model_path =
            CString::new(model_path.into()).map_err(|_| RknnError::ArgumentInvalid)?;
        let c_weight_path =
            CString::new(weight_path.into()).map_err(|_| RknnError::ArgumentInvalid)?;

        let ret = unsafe {
            rknn3_load_model_from_path(self.context, c_model_path.as_ptr(), c_weight_path.as_ptr())
        };
        RknnError::check(ret)
    }
    /// init model on device
    pub fn model_init(&self) -> Result<(), RknnError> {
        let mut raw_config: rknn3_config = unsafe { std::mem::zeroed() };
        raw_config.run_core_mask = 0x1;
        let ret = unsafe { rknn3_model_init(self.context, &mut raw_config) };
        RknnError::check(ret)
    }
    /// query inout and output info

    pub fn query_in_out_num(&self) -> Result<rknn3_input_output_num, RknnError> {
        let mut raw_io: rknn3_input_output_num = unsafe { std::mem::zeroed() };

        let ret = unsafe {
            rknn3_query(
                self.context,
                _rknn3_query_cmd_RKNN3_QUERY_IN_OUT_NUM,
                &mut raw_io as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<rknn3_input_output_num>() as u64,
            )
        };
        RknnError::check(ret)?;
        Ok(raw_io)
    }

    pub fn query_input_attr(&self, index: u32) -> Result<rknn3_tensor_attr, RknnError> {
        self.query_tensor_attr(index, _rknn3_query_cmd_RKNN3_QUERY_INPUT_ATTR)
    }

    pub fn query_output_attr(&self, index: u32) -> Result<rknn3_tensor_attr, RknnError> {
        self.query_tensor_attr(index, _rknn3_query_cmd_RKNN3_QUERY_OUTPUT_ATTR)
    }

    fn query_tensor_attr(&self, index: u32, cmd: u32) -> Result<rknn3_tensor_attr, RknnError> {
        let mut raw_attr: rknn3_tensor_attr = unsafe { std::mem::zeroed() };
        // Before querying, the index of the target Tensor must be set
        raw_attr.index = index;

        let ret = unsafe {
            rknn3_query(
                self.context,
                cmd,
                &mut raw_attr as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<rknn3_tensor_attr>() as u64,
            )
        };
        RknnError::check(ret)?;
        Ok(raw_attr)
    }
    /// create memory for input tensor
    pub fn create_mem(
        &self,
        attr: &rknn3_tensor_attr,
    ) -> Result<*mut _rknn3_tensor_memory, RknnError> {
        let mem = unsafe {
            rknn3_create_mem(
                self.context,
                attr.aligned_size,
                attr.core_id,
                _rknn3_mem_alloc_flags_RKNN3_FLAG_MEMORY_CACHEABLE,
            )
        };
        if mem.is_null() {
            Err(RknnError::Fail)
        } else {
            Ok(mem)
        }
    }
    /// sync input to device
    pub fn input_sync<T: Copy>(
        &self,
        inputs_data: &[T],
        input_tensor: &rknn3_tensor,
    ) -> Result<(), RknnError> {
        unsafe {
            let virt_addr = (*input_tensor.mem).virt_addr as *mut u8;
            std::ptr::copy_nonoverlapping(
                inputs_data.as_ptr() as *const u8,
                virt_addr,
                (*input_tensor.attr).aligned_size as usize,
            );
        }

        let ret = unsafe {
            rknn3_mem_sync(
                self.context,
                input_tensor.mem,
                _rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_TO_DEVICE,
            )
        };

        RknnError::check(ret)?;
        Ok(())
    }
    /// refer
    pub fn run(
        &self,
        inputs: &[rknn3_tensor],
        outputs: &mut [rknn3_tensor],
    ) -> Result<(), RknnError> {
        let ret = unsafe {
            let n_inputs = inputs.len();
            let n_outputs = outputs.len();
            rknn3_run(
                self.context,
                inputs.as_ptr(),
                n_inputs as u32,
                outputs.as_mut_ptr(),
                n_outputs as u32,
            )
        };
        RknnError::check(ret)
    }
    /// sync output from device
    pub fn output_sync(&self, output_tensor: &rknn3_tensor) -> Result<(), RknnError> {
        let ret = unsafe {
            rknn3_mem_sync(
                self.context,
                output_tensor.mem,
                _rknn3_mem_sync_mode_RKNN3_MEMORY_SYNC_FROM_DEVICE,
            )
        };

        RknnError::check(ret)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.context != 0 {
            unsafe {
                rknn3_destroy(self.context);
            }
        }
    }
}
// Helper function to safely convert raw C char array to Rust String
pub fn parse_c_string(raw_name: &[std::os::raw::c_char]) -> String {
    let u8_slice =
        unsafe { std::slice::from_raw_parts(raw_name.as_ptr() as *const u8, raw_name.len()) };
    CStr::from_bytes_until_nul(u8_slice)
        .map(|cstr| cstr.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "Unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_context_load_and_query() -> Result<(), RknnError> {
        let device_id = "0000:01:00.0";
        let model_path = "assets/model.rknn";
        let weight_path = "assets/model.weight";

        Context::find_devices();

        let ctx = Context::with_device(device_id)?;
        ctx.load_model_from_path(model_path, weight_path)?;
        ctx.model_init()?;

        let io_num = ctx.query_in_out_num()?;
        println!("=== RKNN IO Summary ===");
        println!("Input count: {}", io_num.n_input);
        println!("Output count: {}", io_num.n_output);

        println!("\n=== Input Tensor Attributes ===");
        for i in 0..io_num.n_input {
            let attr = ctx.query_input_attr(i)?;
            let tensor_name = parse_c_string(&attr.name);
            let valid_dims = &attr.shape[..attr.n_dims as usize];

            println!("Input [{}]:", i);
            println!("  Name: {}", tensor_name);
            println!("  Index: {}", attr.index);
            println!("  Core ID: {}", attr.core_id);
            println!("  Data Type: {:?}", attr.dtype);
            println!("  Quantitative Type: {:?}", attr.qnt_type);
            println!("  Aligned Size: {} bytes", attr.aligned_size);
            println!("  Element Number: {}", attr.n_elems);
            println!("  Dimensions (n_dims={}): {:?}", attr.n_dims, valid_dims);
        }

        println!("\n=== Output Tensor Attributes ===");
        for i in 0..io_num.n_output {
            let attr = ctx.query_output_attr(i)?;
            let tensor_name = parse_c_string(&attr.name);
            let valid_dims = &attr.shape[..attr.n_dims as usize];

            println!("Output [{}]:", i);
            println!("  Name: {}", tensor_name);
            println!("  Index: {}", attr.index);
            println!("  Core ID: {}", attr.core_id);
            println!("  Data Type: {:?}", attr.dtype);
            println!("  Quantitative Type: {:?}", attr.qnt_type);
            println!("  Aligned Size: {} bytes", attr.aligned_size);
            println!("  Element Number: {}", attr.n_elems);
            println!("  Dimensions (n_dims={}): {:?}", attr.n_dims, valid_dims);
        }

        Ok(())
    }
}
