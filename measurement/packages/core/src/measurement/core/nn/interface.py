"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

from abc import abstractmethod
from ..structs import Metric, Tensor5D, Tensor6D
import torch


class Network(torch.nn.Module):

    def __init__(self):
        super().__init__()

    @abstractmethod
    def refer(self, anchor: Tensor5D, true_samples: Tensor6D):
        """ """
        raise NotImplementedError

    @abstractmethod
    def load_checkpoint(self, path: str) -> None:
        raise NotImplementedError

    @abstractmethod
    def save_checkpoint(self, path: str, metric: Metric) -> None:
        raise NotImplementedError
