# wedjat-metric

`wedjat-metric`是`wedjat`项目的度量学习网络部分的相关代码，包含模型训练、评估、保存、onnx导出的一站式集成子项目，使用[pyvirid](https://github.com/Ailrid/pyvirid/tree/master)驱动。

## 项目构成

项目包含三个子包，相关使用示例在**examples**下。

| 名称          | 核心功能                                             |
| ------------- | ---------------------------------------------------- |
| metric.core   | 模型定义、损失定义、数据加载、精度评估               |
| metric.export | 度量模型、superpoint、lightglue的ONNX模型导出、ONNX推理、Qdrant数据库入库、速度与精确度评估 |
| metric.train  | 模型训练流程、训练日志、模型保存加载                 |

## 其他

[wedjat-core](https://github.com/Ailrid/wedjat/tree/master/core)：`Rust`编写的核心定位引擎、包含`RKNN`推理引擎、匹配定位和跟踪算法、imu积分、Qdrant数据库与tiff读取

[wedjat-rnkk](https://github.com/Ailrid/wedjat/tree/master/rknn)：ONNX到`RKNN`(RK1828)的导出、量化测评代码。