use crate::errors::BackendError;
use opencv::core::Mat;

pub trait ExtractorBackend {
    type Output;
    /// Describe this function.
    ///
    /// # Arguments
    ///
    /// - `drone_img` (`&DMatrix<f32>`) - 无人机当前的图像.
    /// - `sat_img` (`&DMatrix<f32>`) - 要匹配的卫星图像.
    ///
    /// # Returns
    ///
    /// - `Output` - 后端网络提取的特征点结果.一个FeaturePoints类型
    ///
    fn forward(&mut self, drone_img: &Mat, sat_img: &Mat) -> Result<Self::Output, BackendError>;
}

pub trait FromBackend {
    type DataType: Copy;
    type ShapeType;
    /// Describe this function.
    ///
    /// # Arguments
    ///
    /// - `raw_bytes` (`&[u8]`) - u8字节流
    /// - `shape` (`Self`) - 输出的形状
    ///
    /// # Returns
    ///
    /// - `Self` - Describe the return value.
    ///
    fn from_bytes(raw_bytes: &[u8], shape: Self::ShapeType) -> Self;
    fn from_data(data: &[Self::DataType], shape: Self::ShapeType) -> Self;
}
