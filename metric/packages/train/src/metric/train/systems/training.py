"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import json
import torch
from tqdm import tqdm
import numpy as np
from typing import cast
from dataclasses import asdict
from virid.core import system, ViridApp, MessageWriter
from virid.std import execute_block

from metric.core.dataloader import TiffLoader

from ..messages.training import (
    TrainingLightingMessage,
    StartTrainingMessage,
    SaveCheckPointMessage,
    OneEpochMessage,
    EvalMessage,
    LoadCheckPointMessage,
    PlotStateMessage,
)
from ..components import (
    ModelConfig,
    EnvConfig,
    DatasetConfig,
    TrainingState,
    LightParameters,
)
from ..messages.initialization import (
    CreateEvnMessage,
    CreateModelMessage,
    CreateDatasetMessage,
    CreateLoggerAndCheckpointMessage,
)
from ..util import confirm_light_params, plot_training_state


class Color:
    GREEN = "\033[92m"
    CYAN = "\033[96m"
    RED = "\033[91m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    ORANGE = "\033[33m"
    GREY = "\033[90m"
    BOLD = "\033[1m"
    END = "\033[0m"  # 用来结束颜色，否则后面的文本都会变色


@system()
def training_lighting(message: TrainingLightingMessage, app: ViridApp):
    """启动训练流程"""
    # 动态插入 LightParameters 组件
    app.spawn(
        LightParameters(
            message.dataset_params,
            message.model_params,
            message.env_params,
        )
    )

    def callback(success: bool):
        if success:
            MessageWriter.info("Training initialized successfully")
        else:
            MessageWriter.error(RuntimeError("Training initialized failed"))

    with execute_block(group_id="startup", callback=callback):

        CreateLoggerAndCheckpointMessage.send()

        # 在日志初始化之后才能开始打印
        MessageWriter.info(
            "============ Start Up Training ============ \n"
            f"Model Params: {json.dumps(asdict(message.model_params),indent=4, ensure_ascii=False)}\n"
            f"Training Params: {json.dumps(asdict(message.env_params),indent=4, ensure_ascii=False)}\n"
            f"Dataset Params: {json.dumps(asdict(message.dataset_params),indent=4, ensure_ascii=False)}\n"
        )

        CreateDatasetMessage.send()
        # 创建不同模型
        if (
            message.model_params.model_type == "vit"
            or message.model_params.model_type == "resnet"
            or message.model_params.model_type == "convnext"
        ):
            CreateModelMessage.send(message.model_params.model_type)
        else:
            raise ValueError(
                "Invalid model type, only cnn and transformer are supported"
            )

        CreateEvnMessage.send()

        if message.model_params.checkpoint_folder is not None:
            LoadCheckPointMessage.send()

        StartTrainingMessage.send()


@system(message_type=SaveCheckPointMessage)
def save_checkpoint(
    training_state: TrainingState,
    model_config: ModelConfig,
) -> None:
    checkpoint_folder = training_state.checkpoint_folder
    current_metrics = training_state.current_metrics
    best_metrics = training_state.best_metrics

    # 只保存最好的一轮
    if current_metrics.max_accuracy > best_metrics.max_accuracy:
        training_state.best_metrics = current_metrics
        model_config.model.save_checkpoint(
            checkpoint_folder, training_state.best_metrics
        )


@system(message_type=LoadCheckPointMessage)
def load_checkpoint(
    light_params: LightParameters,
    model_config: ModelConfig,
) -> None:
    checkpoint_folder = light_params.model_params.checkpoint_folder
    if checkpoint_folder is None:
        return
    check_result = confirm_light_params(checkpoint_folder, light_params)
    model_config.model.load_checkpoint(checkpoint_folder)

    MessageWriter.info(
        "============ Load CheckPoint Successfully ============ \n"
        f"From Checkpoint Folder: {checkpoint_folder}\n"
        f"Confirmed Light Parameters:\n{check_result}\n"
    )


@system()
def one_epoch(
    message: OneEpochMessage,
    dataset_config: DatasetConfig,
    model_config: ModelConfig,
    env_config: EnvConfig,
    training_state: TrainingState,
) -> None:
    device = env_config.device

    evaluator = env_config.evaluator
    evaluator.reset()

    loss = model_config.loss
    model = model_config.model
    optimizer = env_config.optimizer
    scheduler = env_config.scheduler

    train_loss = training_state.train_loss
    train_max_accuracy = training_state.train_max_accuracy
    MessageWriter.info(
        f"\n{Color.ORANGE}{Color.BOLD} ------------------------------- Train  -------------------------------- {Color.END}\n"
    )

    model.train()
    with tqdm(
        dataset_config.train_loader,
        desc=f"Epoch {message.epoch}",
        total=len(cast(TiffLoader, dataset_config.train_loader.dataset))
        * dataset_config.num_workers
        // dataset_config.batch_size,
    ) as pbar:
        l_statistic = []
        for anchor, positive in pbar:

            anchor = anchor.to(device).to(torch.float32)
            positive = positive.to(device).to(torch.float32)

            optimizer.zero_grad()
            feat_anchor, feat_true_sample = model(anchor, positive)

            l = loss(feat_anchor, feat_true_sample)
            l_statistic.append(l.cpu().item())
            l.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
            optimizer.step()
            evaluator.update(feat_anchor, feat_true_sample)
            pbar.set_description(
                f"Loss: {np.mean(l_statistic):.5f},  Lr: {scheduler.get_last_lr()[0]:.6f}"
            )

    scheduler.step()
    current_metrics, report_str = evaluator.print_metrics()
    MessageWriter.info(report_str)
    train_loss.append(np.mean(l_statistic).item())
    train_max_accuracy.append(current_metrics.max_accuracy)


@system()
def eval_net(
    message: EvalMessage,
    dataset_config: DatasetConfig,
    model_config: ModelConfig,
    env_config: EnvConfig,
    training_state: TrainingState,
) -> None:
    device = env_config.device
    evaluator = env_config.evaluator
    evaluator.reset()

    model = model_config.model
    model.eval()

    test_max_accuracy = training_state.test_max_accuracy

    MessageWriter.info(
        f"\n{Color.BLUE}{Color.BOLD} ------------------------------- Test -------------------------------- {Color.END}\n"
    )
    with torch.no_grad():
        with tqdm(
            dataset_config.test_loader,
            desc=f"Eval {message.epoch}",
            total=len(cast(TiffLoader, dataset_config.test_loader.dataset))
            // dataset_config.batch_size,
        ) as pbar:
            for anchor, positive in pbar:

                anchor = anchor.to(device).to(torch.float32)
                positive = positive.to(device).to(torch.float32)

                feat_anchor, feat_true_sample = model(anchor, positive)

                evaluator.update(feat_anchor, feat_true_sample)

        current_metrics, report_str = evaluator.print_metrics()
        training_state.current_metrics = current_metrics
        test_max_accuracy.append(current_metrics.max_accuracy)
        MessageWriter.info(report_str)


@system(message_type=PlotStateMessage)
def plot_state(
    training_state: TrainingState,
) -> None:
    plot_training_state(training_state)


@system(message_type=StartTrainingMessage)
def start_training(env_config: EnvConfig, train_state: TrainingState) -> None:
    def callback(success: bool):
        if success:
            # 敲重点，重新发送该消息以开启下一个轮训练
            if train_state.current_epoch == env_config.epochs:
                return
            StartTrainingMessage.send()
        else:
            MessageWriter.error(
                RuntimeError(
                    f"\n{Color.RED}{Color.BOLD} ------------------------------- Epoch {train_state.current_epoch} Failed -------------------------------- {Color.END}\n"
                )
            )

    # Start 绿色 + 加粗
    MessageWriter.info(
        f"\n{Color.GREEN}{Color.BOLD} ------------------------------- Epoch {train_state.current_epoch} Start -------------------------------- {Color.END}\n"
    )

    with execute_block(
        group_id=f"epoch-{train_state.current_epoch}", callback=callback
    ):
        train_state.current_epoch += 1
        OneEpochMessage.send(train_state.current_epoch)
        EvalMessage.send(train_state.current_epoch)
        SaveCheckPointMessage.send()
        PlotStateMessage.send()


def register_training_systems(app: ViridApp):
    app.register(training_lighting)
    app.register(start_training)

    app.register(save_checkpoint)
    app.register(load_checkpoint)

    app.register(one_epoch)
    app.register(eval_net)
    app.register(plot_state)
