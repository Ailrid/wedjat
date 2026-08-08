"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from virid.core import EventMessage
from dataclasses import dataclass


@dataclass
class CreateModelMessage(EventMessage):
    model_type: str


class CreateDatasetMessage(EventMessage): ...


class CreateEvnMessage(EventMessage): ...


class CreateLoggerAndCheckpointMessage(EventMessage): ...
