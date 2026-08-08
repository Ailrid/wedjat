"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

from .initialization import *
from .logger import *
from .training import *
from virid.core import ViridApp


def bind_components(app: ViridApp) -> None:
    bind_training_components(app)
    bind_logger_components(app)
