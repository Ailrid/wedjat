"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import torch
import time
import os
from torch import optim
from torch.optim.lr_scheduler import LinearLR, CosineAnnealingLR, SequentialLR
from virid.core import system, ViridApp, MessageWriter

from metric.core import (
    Shell,
    MetricEvaluator,
    MSLoss,
    ResNet50,
    ViT,
    ConvNeXtTiny,
)
from metric.core.dataloader import (
    get_cross_view_dataloader,
    get_tiff_dataloader,
)
from metric.core.nn import Classify

from ..messages.initialization import (
    CreateEvnMessage,
    CreateModelMessage,
    CreateDatasetMessage,
    CreateLoggerAndCheckpointMessage,
)

from ..components import (
    ModelConfig,
    EnvConfig,
    DatasetConfig,
    TrainingLogger,
    TrainingState,
    LightParameters,
)
from ..util import create_logger, save_light_params


@system(message_type=CreateLoggerAndCheckpointMessage)
def create_logger_and_checkpoint(
    logger: TrainingLogger,
    training_state: TrainingState,
    light_params: LightParameters,
):
    # 创建开始时间文件夹
    logger_folder = os.path.join(
        "./logs", time.strftime("%Y-%m-%d-%H-%M-%S", time.localtime())
    )
    os.makedirs(logger_folder, exist_ok=True)
    # 创建日志文件
    log_file_path = os.path.join(logger_folder, "training.txt")
    logger.writer = create_logger(log_file_path)
    # 创建checkpoint文件夹
    checkpoint_folder = os.path.join(
        "./checkpoints",
        # 精确到秒
        time.strftime("%Y-%m-%d-%H-%M-%S", time.localtime()),
    )
    os.makedirs(checkpoint_folder, exist_ok=True)
    training_state.log_folder = logger_folder
    training_state.checkpoint_folder = checkpoint_folder
    # 保存一份训练设置到checkpoint文件夹中
    save_light_params(checkpoint_folder, light_params)
    MessageWriter.info(
        "============ Create Logger And Checkpoint Done ============ \n"
        f"Logger Folder: {logger_folder}\n"
        f"Checkpoint Folder: {checkpoint_folder}\n"
    )


@system(message_type=CreateDatasetMessage)
def create_dataset(
    app: ViridApp,
    light_params: LightParameters,
) -> None:
    dataset_params = light_params.dataset_params
    dataset_type = dataset_params.dataset_type
    if dataset_type == "tiff":
        train_loader = get_tiff_dataloader(
            dataset_params.train_folder,
            dataset_params.input_size,
            dataset_params.batch_size,
            dataset_params.samples_per_yield,
            dataset_params.true_sample_number,
            is_train=True,
            num_workers=dataset_params.num_workers,
        )
        test_loader = get_tiff_dataloader(
            dataset_params.train_folder,
            dataset_params.input_size,
            dataset_params.batch_size,
            dataset_params.samples_per_yield,
            dataset_params.true_sample_number,
            is_train=False,
            num_workers=dataset_params.num_workers,
        )
    elif dataset_type == "cross_view":
        train_loader = get_cross_view_dataloader(
            dataset_params.train_folder,
            (dataset_params.input_size, dataset_params.input_size),
            dataset_params.batch_size,
            dataset_params.samples_per_yield,
            dataset_params.true_sample_number,
            is_train=True,
            num_workers=dataset_params.num_workers,
        )
        test_loader = get_cross_view_dataloader(
            dataset_params.test_folder,
            (dataset_params.input_size, dataset_params.input_size),
            dataset_params.batch_size,
            dataset_params.samples_per_yield,
            dataset_params.true_sample_number,
            is_train=False,
            num_workers=dataset_params.num_workers,
        )
    else:
        raise Exception(
            f"Invalid dataset type: {dataset_type}, please check your config file."
        )

    app.spawn(
        DatasetConfig(
            dataset_type=dataset_type,
            batch_size=dataset_params.batch_size,
            input_size=dataset_params.input_size,
            train_loader=train_loader,
            test_loader=test_loader,
            num_workers=dataset_params.num_workers,
        )
    )
    MessageWriter.info(
        "============ Create Dataset Done ============ \n"
        f"Train Folder: {dataset_params.train_folder}\n"
        f"Test Folder: {dataset_params.test_folder}\n"
        f"Iter Times: {dataset_params.iter_times}\n"
        f"Input Size: {dataset_params.input_size}\n"
    )


@system()
def create_model(
    message: CreateModelMessage,
    app: ViridApp,
    light_params: LightParameters,
) -> None:
    model_params = light_params.model_params
    env_params = light_params.env_params

    if message.model_type == "resnet":
        net = ResNet50(model_params.out_dims)
    elif message.model_type == "vit":
        net = ViT(model_params.out_dims)
    elif message.model_type == "convnext":
        net = ConvNeXtTiny(model_params.out_dims)
    else:
        raise ValueError(
            f"Invalid model type: {message.model_type}, please check your config file."
        )

    if (
        light_params.dataset_params.dataset_type == "cross_view"
        and model_params.num_classes is not None
    ):
        classify = Classify(model_params.out_dims, model_params.num_classes)
    else:
        classify = None

    model = Shell(net, classify).to(env_params.device)
    loss = MSLoss().to(env_params.device)

    app.spawn(
        ModelConfig(
            model=model,
            loss=loss,
        )
    )

    MessageWriter.info(
        "============ Create Model Done ============ \n"
        f"Oot Dims: {model_params.out_dims}\n"
        f"Dropout: {model_params.dropout}\n"
    )


@system(message_type=CreateEvnMessage)
def create_env(
    app: ViridApp,
    model_config: ModelConfig,
    light_params: LightParameters,
) -> None:
    env_params = light_params.env_params

    optimizer = optim.AdamW(
        model_config.model.parameters(),
        lr=env_params.lr,
        weight_decay=env_params.weight_decay,
    )
    # 先初始化余弦
    cosine_scheduler = CosineAnnealingLR(
        optimizer,
        T_max=(env_params.epochs - env_params.warmup_epochs),
        eta_min=5e-7,
    )

    # 后初始化 LinearLR
    warmup_scheduler = LinearLR(
        optimizer,
        start_factor=0.1,
        end_factor=1.0,
        total_iters=env_params.warmup_epochs,
    )
    scheduler = SequentialLR(
        optimizer,
        schedulers=[warmup_scheduler, cosine_scheduler],
        milestones=[env_params.warmup_epochs],
    )

    evaluator = MetricEvaluator()
    app.spawn(
        EnvConfig(
            lr=env_params.lr,
            epochs=env_params.epochs,
            warmup_epochs=env_params.warmup_epochs,
            evaluator=evaluator,
            scheduler=scheduler,
            optimizer=optimizer,
            device=env_params.device,
        )
    )

    MessageWriter.info(
        "============ Create Train Env Done ============ \n"
        f"Lr: {env_params.lr}\n"
        f"Weight Decay: {env_params.weight_decay}\n"
        f"Epochs: {env_params.epochs}\n"
        f"Warmup Epochs: {env_params.warmup_epochs}\n"
        f"Device: {env_params.device}, Device Name: {torch.cuda.get_device_name(env_params.device)}\n"
    )


def register_initialization_systems(app: ViridApp):
    app.register(create_dataset)
    app.register(create_model)
    app.register(create_env)
    app.register(create_logger_and_checkpoint)
