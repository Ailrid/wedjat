"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

from virid.core import component, ViridApp
from dataclasses import dataclass, field
from metric.core import Metric, RankMetric


@component()
@dataclass()
class TrainingState:
    # 此次训练的开始时间
    timestamp: str = ""
    # 当前的轮数
    current_epoch: int = 0
    # 当前epoch的评估指标
    current_metrics: RankMetric = field(default_factory=RankMetric)
    # 最好的一次评估指标
    best_metrics: RankMetric = field(default_factory=RankMetric)
    # 历史评估指标
    metrics_history: Metric = field(default_factory=Metric)
    #  train_loss
    train_loss: list[float] = field(default_factory=lambda: list())
    # 日志路径
    log_folder: str = ""
    # 模型保存路径
    checkpoint_folder: str = ""


def bind_training_components(app: ViridApp):
    app.bind(TrainingState)
