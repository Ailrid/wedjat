# wedjat

`wedjat`是一整套完整的包含模型训练导出、推理引擎、向量与图像数据库管理、硬件抽象、地图匹配定位算法、跟踪算法的，主要使用`Rust`、`Python`、`Typescript`编写的生产级拒止匹配定位系统。

## 项目构成

项目包含五个子包，分别负责不同功能。

- [wedjat-core](https://github.com/Ailrid/wedjat/tree/master/core)：`Rust`编写的核心定位引擎、包含`RKNN`推理引擎、匹配定位和跟踪算法、`imu`积分、`Qdrant`数据库与`tiff`读取。
- [wedjat-metric](https://github.com/Ailrid/wedjat/tree/master/metric)：度量学习网络部分的相关代码，包含模型训练、评估、保存、onnx导出的一站式集成子项目，使用[pyvirid](https://github.com/Ailrid/pyvirid/tree/master)驱动。
- [wedjat-rknn](https://github.com/Ailrid/wedjat/tree/master/rknn)：ONNX到`RKNN`(RK1828)的导出、量化测评代码。

- [rknn1828_installer](https://github.com/Ailrid/wedjat/tree/master/rk1828_installer)：一键安装`rk1828`驱动、`transfer_proxy`、`lib`运行库、`apt`源与开发工具包。

- [client](https://github.com/Ailrid/wedjat/tree/master/client)：`wedjat`项目的`web`管理客户端，负责渲染地理数据与定位状态，由[virid](https://github.com/Ailrid/virid)驱动。