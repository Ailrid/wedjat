use crate::errors::BackendError;
use opencv::core::Mat;
pub trait MatcherBackend {
    /// Describe this function.
    ///
    /// # Arguments
    ///
    /// - `drone_img` (`&DMatrix<f32>`) - 无人机当前的图像.
    ///
    /// # Returns
    ///
    /// - `Output` - 后端网络提取的特征点结果.一个FeaturePoints类型
    ///
    fn forword(&mut self, drone_img: &Mat) -> Result<Vec<f32>, BackendError>;
}
