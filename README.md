# wedjat

`wedjat`（ 荷鲁斯之眼）是一整套完整的包含模型训练导出、推理引擎、向量与图像数据库管理、硬件抽象、地图匹配定位算法、跟踪算法的，主要使用Rust和Python编写的生产级拒止匹配定位系统。

## 项目构成

项目包含三个子包，分别负责不同功能。

- [wedjat-core](https://github.com/Ailrid/wedjat/tree/master/core)：`Rust`编写的核心定位引擎、包含`RKNN`推理引擎、匹配定位和跟踪算法、imu积分、Qdrant数据库与tiff读取。
- [wedjat-metric](https://github.com/Ailrid/wedjat/tree/master/metric)：度量学习网络部分的相关代码，包含模型训练、评估、保存、onnx导出的一站式集成子项目，使用[pyvirid](https://github.com/Ailrid/pyvirid/tree/master)驱动。
- [wedjat-rknn](https://github.com/Ailrid/wedjat/tree/master/rknn)：ONNX到`RKNN`(RK1828)的导出、量化测评代码。