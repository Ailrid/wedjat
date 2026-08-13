/// 定义特征数据结构体并自动实现 `FromBackend` Trait。
///
/// 该宏支持两种形状模式：
/// 1. 二维形状：`(usize, usize)`，适用于矩阵或坐标对（如 `(N, 2)`）。
/// 2. 一维形状：`usize`，适用于得分或简单的列表。
///
/// # 参数
/// * `$name`: 要生成的结构体名称（如 `KeyPoints`, `Scores`）。
/// * `$t`: 数据元素的类型（如 `i64`, `f32`）。
/// * 形状描述：`(usize, usize)` 或 `usize`。
///
/// # 示例
/// ```
/// define_feature_data!(KeyPoints, i64, (usize, usize));
/// define_feature_data!(Scores, f32, usize);
/// ```

#[macro_export]
macro_rules! define_feature_data {
    // 匹配双维度 (usize, usize)
    ($name:ident, $t:ty, (usize, usize)) => {
        pub struct $name {
            pub shape: (usize, usize),
            pub data: std::vec::Vec<$t>,
        }
        // 调用下面的辅助宏实现 Trait
        $crate::from_backend!($name, $t, (usize, usize));
    };

    // 匹配单维度 usize
    ($name:ident, $t:ty, usize) => {
        pub struct $name {
            pub shape: usize,
            pub data: std::vec::Vec<$t>,
        }
        // 调用下面的辅助宏实现 Trait
        $crate::from_backend!($name, $t, usize);
    };
}

/// 为指定的结构体实现 `FromBackend` Trait，用于将后端（C++/ONNX/Vino）输出转换为 Rust 结构。
///
/// 该宏提供两种转换路径：
/// 1. `from_bytes`: 处理原始字节流，使用 `read_unaligned` 以确保在内存不对齐时的安全性。
/// 2. `from_data`: 处理已对齐的类型切片，通过 `to_vec` (memcpy) 实现高速拷贝。
///
/// # 注意
/// 通常不需要直接调用此宏，它由 `define_feature_data!` 内部自动调用。
#[macro_export]
macro_rules! from_backend {
    // 模式 1: (usize, usize)
    ($struct_name:ident, $data_type:ty, (usize, usize)) => {
        impl $crate::extractor::traits::FromBackend for $struct_name {
            type DataType = $data_type;
            type ShapeType = (usize, usize);

            fn from_bytes(raw_bytes: &[u8], shape: Self::ShapeType) -> Self {
                let count = shape.0 * shape.1;
                let mut data = Vec::with_capacity(count);
                // 将 u8 指针强转为目标类型指针
                let ptr = raw_bytes.as_ptr() as *const Self::DataType;

                for i in 0..count {
                    unsafe {
                        // 稳健路径：无视对齐，逐个读取字节
                        data.push(std::ptr::read_unaligned(ptr.add(i)));
                    }
                }
                Self { shape, data }
            }

            fn from_data(data_slice: &[Self::DataType], shape: Self::ShapeType) -> Self {
                // 快路径：ONNX 已经保证了对齐和类型，直接 to_vec() 触发 memcpy
                Self {
                    shape,
                    data: data_slice.to_vec(),
                }
            }
        }
    };

    // 模式 2: usize
    ($struct_name:ident, $data_type:ty, usize) => {
        impl $crate::extractor::traits::FromBackend for $struct_name {
            type DataType = $data_type;
            type ShapeType = usize;

            fn from_bytes(raw_bytes: &[u8], shape: Self::ShapeType) -> Self {
                let count = shape;
                let mut data = Vec::with_capacity(count);
                let ptr = raw_bytes.as_ptr() as *const Self::DataType;

                for i in 0..count {
                    unsafe {
                        data.push(std::ptr::read_unaligned(ptr.add(i)));
                    }
                }
                Self { shape, data }
            }

            fn from_data(data_slice: &[Self::DataType], shape: Self::ShapeType) -> Self {
                Self {
                    shape,
                    data: data_slice.to_vec(),
                }
            }
        }
    };
}

/// 高性能图像填充宏：将两个 OpenCV Mat 填充到预分配的 f32 切片中。
///
/// # 参数
/// * `$drone_img`: 无人机 Mat 图像
/// * `$sat_img`: 卫星 Mat 图像
/// * `$buffer`: 目标 `&mut [f32]`
/// * `$image_size`: 单张图像的像素总数 (H * W)
#[macro_export]
macro_rules! fill_images_to_buffer {
    // 分支 1：处理双图单通道灰度图像 (Drone & Sat)
    ($drone_img:expr, $sat_img:expr, $buffer:expr, $image_size:expr) => {{
        use opencv::{
            core::{CV_32F, Mat},
            imgproc,
            prelude::*,
        };

        let size = $image_size as i32;
        let expected_half_size = (size * size) as usize;

        // 扩容 buffer
        if $buffer.len() < 2 * expected_half_size {
            $buffer.resize(2 * expected_half_size, 0.0f32);
        }

        // 缩放到指定尺寸
        let process_and_fill = |src: &Mat, dest: &mut [f32]| -> Result<(), opencv::Error> {
            let mut gray = Mat::default();
            let mut resized = Mat::default();
            let mut normalized = Mat::default();

            // 转灰度
            imgproc::cvt_color(src, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
            // 强行缩放到模型要求的尺寸
            imgproc::resize(
                &gray,
                &mut resized,
                opencv::core::Size::new(size, size),
                0.0,
                0.0,
                imgproc::INTER_LINEAR,
            )?;
            // 归一化到 [0, 1]
            resized.convert_to(&mut normalized, CV_32F, 1.0 / 255.0, 0.0)?;

            // 拷贝数据
            if normalized.is_continuous() {
                let data = normalized.data_typed::<f32>()?;
                dest[..expected_half_size].copy_from_slice(&data[..expected_half_size]);
            } else {
                // 处理非连续内存的情况
                for r in 0..normalized.rows() {
                    let row = normalized.at_row::<f32>(r)?;
                    let offset = r as usize * size as usize;
                    dest[offset..offset + size as usize].copy_from_slice(row);
                }
            }
            Ok(())
        };

        let (first_half, second_half) = $buffer.split_at_mut(expected_half_size);
        process_and_fill($drone_img, first_half)?;
        process_and_fill($sat_img, second_half)?;
    }};
    // 分支 2：处理3通道无人机图像 (Drone & Sat)
    ($img:expr, $dest:expr) => {{
        use rayon::prelude::*;

        let channels = $img.channels() as usize;
        let width = $img.cols() as usize;
        let height = $img.rows() as usize;
        let plane_size = width * height;
        let expected_size = channels * plane_size;

        $dest.resize(expected_size, 0.0f32);

        // ImageNet标准化参数
        let mean = [0.485f32, 0.456, 0.406];
        let std = [0.229f32, 0.224, 0.225];

        // 预先转换为 f32 提高后续计算效率 (0.0 - 1.0)
        let mut float_img = opencv::core::Mat::default();
        $img.convert_to(&mut float_img, opencv::core::CV_32F, 1.0 / 255.0, 0.0)?;

        // 获取底层的裸指针用于跨线程写入 (dest 已预分配空间，安全受控)
        let dest_ptr = $dest.as_ptr() as usize;

        (0..height).into_par_iter().for_each(|y| {
            if let Ok(row) = float_img.at_row::<opencv::core::Vec3f>(y as i32) {
                let ptr = dest_ptr as *mut f32;
                for (x, pixel) in row.iter().enumerate() {
                    let idx = y * width + x;
                    unsafe {
                        // OpenCV 默认是 BGR (0:B, 1:G, 2:R)
                        // Python 代码期望 RGB 顺序且经过 (x - mean) / std

                        // R Channel
                        *ptr.add(0 * plane_size + idx) = (pixel[2] - mean[0]) / std[0];
                        // G Channel
                        *ptr.add(1 * plane_size + idx) = (pixel[1] - mean[1]) / std[1];
                        // B Channel
                        *ptr.add(2 * plane_size + idx) = (pixel[0] - mean[2]) / std[2];
                    }
                }
            }
        });
    }};
}
