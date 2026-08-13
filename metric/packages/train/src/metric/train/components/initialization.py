"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
from metric.core import (
    TiffLoader,
    Network,
    MetricEvaluator,
    MSLoss,
)
from virid.core import component
from dataclasses import dataclass
from torch.utils.data import DataLoader
from ..params import ModelParameters, DatasetParameters, EnvParameters


@component()
@dataclass()
class LightParameters:
    dataset_params: DatasetParameters
    model_params: ModelParameters
    env_params: EnvParameters


@component()
@dataclass()
class ModelConfig:
    model: Network
    loss: MSLoss


@component()
@dataclass()
class DatasetConfig:
    batch_size: int
    input_size: int
    train_loader: DataLoader[TiffLoader]
    test_loader: DataLoader[TiffLoader]
    num_workers: int


@component()
@dataclass()
class EnvConfig:
    lr: float
    epochs: int
    warmup_epochs: int
    evaluator: MetricEvaluator
    optimizer: torch.optim.Optimizer
    scheduler: torch.optim.lr_scheduler.SequentialLR | torch.optim.lr_scheduler.LambdaLR
    device: str
