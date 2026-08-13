"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

from virid.core import component, ViridApp
from dataclasses import dataclass
import logging


@component()
@dataclass()
class TrainingLogger:
    writer: logging.Logger = None  # type: ignore


def bind_logger_components(app: ViridApp):
    app.bind(TrainingLogger)
