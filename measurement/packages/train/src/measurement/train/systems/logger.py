"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""

from virid.core import system, InfoMessage, WarnMessage, ErrorMessage, ViridApp
from ..components import TrainingLogger
import traceback


# 不要设置优先级，否则必须要等其他system全部都执行完了才会打印
@system()
def info(message: InfoMessage, logger: TrainingLogger) -> None:
    if logger.writer is None:
        return
    logger.writer.info(message.context)


@system()
def warn(message: WarnMessage, logger: TrainingLogger) -> None:
    if logger.writer is None:
        return
    logger.writer.warning(message.context)


@system()
def error(message: ErrorMessage, logger: TrainingLogger) -> None:
    if logger.writer is None:
        return
    if message.error:
        # 打印完整调用栈
        error = message.error
        traceback.print_exception(type(error), error, error.__traceback__)
        logger.writer.error(str(message.error), extra={"msg_type": "error"})
    if message.context:
        logger.writer.error(str(message.context), extra={"msg_type": "context"})


def register_logger_systems(app: ViridApp) -> None:
    app.register(info)
    app.register(warn)
    app.register(error)
