"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from virid.core import create_virid
from virid.std import StdPlugin
from metric.train import (
    bind_components,
    DatasetParameters,
    ModelParameters,
    EnvParameters,
    register_systems,
    TrainingLightingMessage,
)

virid = create_virid(max_depth=10000, enable_logging=False).use(StdPlugin, None)

bind_components(virid)
register_systems(virid)

# 点火启动
TrainingLightingMessage.send(
    dataset_params=DatasetParameters(
        train_folder="experiment/train",
        test_folder="experiment/test",
        batch_size=4,  # 建议设置为1
        num_workers=4,  # 数据加载线程
        iter_times=1000,
        input_size=256,  # 输入给网络的大小
        true_sample_number=2,
        samples_per_yield=2,
    ),
    model_params=ModelParameters(
        checkpoint_folder=None,
        model_type="convnext",  # resnet, vit, convnext
        out_dims=512,
        dropout=0.1,
    ),
    env_params=EnvParameters(
        lr=5e-5,
        epochs=100,
        warmup_epochs=3,
        weight_decay=1e-4,
        device="cuda:0",
    ),
)

if __name__ == "__main__":
    virid.tick()
