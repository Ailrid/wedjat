"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

from typing import Optional
from dataclasses import dataclass


@dataclass()
class ModelParameters:
    checkpoint_folder: Optional[str] = None
    model_type: str = "cnn"
    out_dims: int = 2048
    dropout: float = 0.1


@dataclass()
class DatasetParameters:
    train_folder: str
    test_folder: str
    batch_size: int = 32
    num_workers: int = 1
    iter_times: int = 1
    input_size: int = 128
    true_sample_number: int = 8
    samples_per_yield: int = 8


@dataclass()
class EnvParameters:
    lr: float = 1e-4
    epochs: int = 100
    warmup_epochs: int = 5
    weight_decay: float = 0.05
    device: str = "cpu"
