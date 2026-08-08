"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from dataclasses import dataclass
from virid.core import EventMessage
from ..params import ModelParameters, DatasetParameters, EnvParameters


@dataclass
class TrainingLightingMessage(EventMessage):
    dataset_params: DatasetParameters
    model_params: ModelParameters
    env_params: EnvParameters


class StartTrainingMessage(EventMessage): ...


class SaveCheckPointMessage(EventMessage): ...


@dataclass
class OneEpochMessage(EventMessage):
    epoch: int


@dataclass
class EvalMessage(EventMessage):
    epoch: int


class LoadCheckPointMessage(EventMessage): ...


class PlotStateMessage(EventMessage): ...
