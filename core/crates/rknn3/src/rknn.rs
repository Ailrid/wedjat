//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core
use super::context::{Context, parse_c_string};
use super::error::*;
use rknn_sys::{
    _rknn3_tensor_type_RKNN3_TENSOR_FLOAT16, _rknn3_tensor_type_RKNN3_TENSOR_INT8,
    _rknn3_tensor_type_RKNN3_TENSOR_UINT8, rknn3_destroy_mem, rknn3_tensor,
};

pub struct Rknn3 {
    context: Context,
    input_tensors: Vec<rknn3_tensor>,
    output_tensors: Vec<rknn3_tensor>,
}

impl Rknn3 {
    pub fn new(
        device_id: impl Into<String>,
        model_path: impl Into<String>,
        weight_path: impl Into<String>,
    ) -> Result<Self, RknnError> {
        let context = Context::with_device(device_id)?;
        let model_name = model_path.into();
        context.load_model_from_path(model_name.clone(), weight_path)?;
        context.model_init()?;

        let io_num = context.query_in_out_num()?;

        println!("=== Model {} Summary ===", model_name);
        println!("=== RKNN IO Summary ===");
        println!("Input count: {}", io_num.n_input);
        println!("Output count: {}", io_num.n_output);

        println!("\n=== Input Tensor Attributes ===");
        for i in 0..io_num.n_input {
            let attr = context.query_input_attr(i)?;
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
            let attr = context.query_output_attr(i)?;
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

        let mut input_tensors = Vec::with_capacity(io_num.n_input as usize);
        for i in 0..io_num.n_input {
            let attr = context.query_input_attr(i)?;
            let mem = context.create_mem(&attr)?;

            let attr_ptr = Box::into_raw(Box::new(attr));
            input_tensors.push(rknn3_tensor {
                attr: attr_ptr,
                mem,
            });
        }

        let mut output_tensors = Vec::with_capacity(io_num.n_output as usize);
        for i in 0..io_num.n_output {
            let attr = context.query_output_attr(i)?;
            let mem = context.create_mem(&attr)?;

            let attr_ptr = Box::into_raw(Box::new(attr));

            output_tensors.push(rknn3_tensor {
                attr: attr_ptr,
                mem,
            });
        }

        Ok(Self {
            context,
            input_tensors,
            output_tensors,
        })
    }
    pub fn run<T: Copy>(&mut self, inputs: &[&[T]]) -> Result<(), RknnError> {
        for (i, input) in inputs.iter().enumerate() {
            self.context.input_sync(input, &self.input_tensors[i])?;
        }
        self.context
            .run(&self.input_tensors, &mut self.output_tensors)?;

        for output in self.output_tensors.iter() {
            self.context.output_sync(&output)?;
        }
        Ok(())
    }
    #[allow(non_upper_case_globals)]
    pub fn get_output(&self, index: usize) -> Result<Vec<f32>, RknnError> {
        let output = self
            .output_tensors
            .get(index)
            .ok_or(RknnError::ArgumentInvalid)?;

        let attr = unsafe { &*output.attr };
        let mem_ptr = unsafe { (*output.mem).virt_addr };

        if mem_ptr.is_null() {
            return Err(RknnError::ArgumentInvalid);
        }

        let len = attr.n_elems as usize;

        match attr.dtype {
            _rknn3_tensor_type_RKNN3_TENSOR_FLOAT16 => {
                let slice = unsafe { std::slice::from_raw_parts(mem_ptr as *const u16, len) };
                let output = slice
                    .iter()
                    .map(|&bits| half::f16::from_bits(bits).to_f32())
                    .collect();
                Ok(output)
            }
            _rknn3_tensor_type_RKNN3_TENSOR_UINT8 => {
                let slice = unsafe { std::slice::from_raw_parts(mem_ptr as *const u8, len) };
                let zero_point = attr.qnt_info.zero_point as f32;
                let scale = attr.qnt_info.scale;
                let output = slice
                    .iter()
                    .map(|&val| (val as f32 - zero_point) * scale)
                    .collect();
                Ok(output)
            }
            _rknn3_tensor_type_RKNN3_TENSOR_INT8 => {
                let slice = unsafe { std::slice::from_raw_parts(mem_ptr as *const i8, len) };
                let zero_point = attr.qnt_info.zero_point as f32;
                let scale = attr.qnt_info.scale;
                let output = slice
                    .iter()
                    .map(|&val| (val as f32 - zero_point) * scale)
                    .collect();
                Ok(output)
            }
            _ => Err(RknnError::OtherError("Unknown data type".to_string())),
        }
    }
}

impl Drop for Rknn3 {
    fn drop(&mut self) {
        unsafe {
            let context = self.context.context;

            for input in self.input_tensors.iter() {
                rknn3_destroy_mem(context, input.mem);
                if !input.attr.is_null() {
                    let _ = Box::from_raw(input.attr);
                }
            }

            for output in self.output_tensors.iter() {
                rknn3_destroy_mem(context, output.mem);
                if !output.attr.is_null() {
                    let _ = Box::from_raw(output.attr);
                }
            }
        }
    }
}
