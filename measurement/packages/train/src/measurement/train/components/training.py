"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

from virid.core import component, ViridApp
from dataclasses import dataclass, field
from measurement.core import Metric


@component()
@dataclass()
class TrainingState:
    # 此次训练的开始时间
    timestamp: str = ""
    # 当前的轮数
    current_epoch: int = 0
    # 当前epoch的评估指标
    current_metrics: Metric = field(default_factory=Metric)
    # 最好的一次评估指标
    best_metrics: Metric = field(default_factory=Metric)
    # 最好的一次的混淆矩阵
    best_hist_matrix: list[list[int]] = field(default_factory=lambda: list())
    #  train_loss
    train_loss: list[float] = field(default_factory=lambda: list())
    train_max_accuracy: list[float] = field(default_factory=lambda: list())
    test_max_accuracy: list[float] = field(default_factory=lambda: list())
    # 日志路径
    log_folder: str = ""
    # 模型保存路径
    checkpoint_folder: str = ""


def bind_training_components(app: ViridApp):
    app.bind(TrainingState)
