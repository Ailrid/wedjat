"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: metric
"""

from .logger import *
from .initialization import *
from .training import *
from virid.core import ViridApp


def register_systems(app: ViridApp) -> None:
    register_logger_systems(app)
    register_initialization_systems(app)
    register_training_systems(app)
