"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: measurement
"""
import time
import torch
from measurement.core import ConvNeXtTiny, ResNet50, ViT


def benchmark_model(
    model: torch.nn.Module,
    input_tensor: torch.Tensor,
    warm_up: int = 20,
    runs: int = 100,
    device: torch.device = torch.device("cpu"),
) -> tuple[float, float]:
    """Measures average latency (ms) and throughput (FPS) for a given model."""
    model = model.to(device)
    model.eval()
    input_tensor = input_tensor.to(device)

    # Warm-up iterations to stabilize hardware clocks and GPU state
    with torch.no_grad():
        for _ in range(warm_up):
            _ = model(input_tensor)

    if device.type == "cuda":
        torch.cuda.synchronize()

    # Time measurement loop
    start_time = time.perf_counter()
    with torch.no_grad():
        for _ in range(runs):
            _ = model(input_tensor)
            if device.type == "cuda":
                torch.cuda.synchronize()
    end_time = time.perf_counter()

    avg_latency_ms = ((end_time - start_time) / runs) * 1000
    fps = (1000 / avg_latency_ms) * input_tensor.size(0)
    return avg_latency_ms, fps


def main():
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Running benchmark on device: {device}\n")

    batch_size = 1

    # Define models alongside dummy inputs matching required resolutions
    models_config = {
        "ResNet50": (
            ResNet50(),
            torch.randn(batch_size, 3, 256, 256),
        ),
        "ConvNeXtTiny": (
            ConvNeXtTiny(),
            torch.randn(batch_size, 3, 256, 256),
        ),
        "ViT": (
            ViT(),
            torch.randn(batch_size, 3, 224, 224),
        ),
    }

    header = f"{'Model':<15} | {'Input Shape':<18} | {'Latency (ms)':<15} | {'FPS':<10}"
    print(header)
    print("-" * len(header))

    for name, (model, dummy_input) in models_config.items():
        latency, fps = benchmark_model(model, dummy_input, device=device)
        shape_str = f"{list(dummy_input.shape)}"
        print(f"{name:<15} | {shape_str:<18} | {latency:<15.2f} | {fps:<10.2f}")


if __name__ == "__main__":
    main()
