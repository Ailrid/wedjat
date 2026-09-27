# wedjat-core

`wedjat-core`是`wedjat`项目的rust推理引擎与跟踪算法的相关代码，包含rk1828底层推理引擎封装、多假设跟踪算法、相机对接、tiff与qdrant读取、imu积分等。

## 项目构成

项目包含多个子包，在crates下分别负责不同的功能。

| 名称     | 核心功能                                              |
| -------- | ----------------------------------------------------- |
| camera   | 相机与外部数据读取、基础帧数据定义                    |
| client   | tiff流式分割与读取、qdrant数据库对接                  |
| engine   | superpoint、lightglue、metric网络推理、多假设跟踪算法 |
| imu      | imu预积分逻辑                                         |
| rknn-sys | rknn底层库封装                                        |
| rknn3    | rknn接口安全封装                                      |

## 其他

[wedjat-rnkk](https://github.com/Ailrid/wedjat/tree/master/rknn)：ONNX到`RKNN`(RK1828)的导出、量化测评代码。

[wedjat-metirc](https://github.com/Ailrid/wedjat/tree/master/metirc)：度量学习与其他网络训练和onnx导出代码