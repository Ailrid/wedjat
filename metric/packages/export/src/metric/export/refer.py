import os
import torch
import numpy as np

# 1. 动态借调 Torch 的库路径 (解决 libcublas.so 找不到的问题)
torch_lib_path: str = os.path.join(os.path.dirname(torch.__file__), "lib")
if os.path.exists(torch_lib_path):
    os.environ["LD_LIBRARY_PATH"] = (
        f"{torch_lib_path}:{os.environ.get('LD_LIBRARY_PATH', '')}"
    )

import onnxruntime as ort

class ONNXInferencer:
    """
    ONNX model inference wrapper with CUDA and CPU execution providers.
    Handles dynamic batch dimension, normalization, and memory optimization.
    """

    def __init__(self, model_path: str, device_id: int = 0) -> None:
        """
        Initialize the inference session with specified providers.

        Args:
            model_path (str): Path to the ONNX model file.
            device_id (int): CUDA device index for GPU inference.
        """
        self.model_path = model_path

        providers = [
            "CUDAExecutionProvider",
            "CPUExecutionProvider",
        ]

        try:
            self.session = ort.InferenceSession(model_path, providers=providers)
            print(f"Session initialized with providers: {self.session.get_providers()}")
        except Exception as e:
            print(f"Failed to initialize CUDA provider: {e}. Falling back to CPU...")
            self.session = ort.InferenceSession(
                model_path, providers=["CPUExecutionProvider"]
            )

        # Cache input and output binding names
        self.input_name: str = self.session.get_inputs()[0].name
        self.output_name: str = self.session.get_outputs()[0].name

        # ImageNet normalization statistics pre-formatted for broadcasting [1, 3, 1, 1]
        self.mean: np.ndarray = np.array(
            [0.485, 0.456, 0.406], dtype=np.float32
        ).reshape(1, 3, 1, 1)
        self.std: np.ndarray = np.array(
            [0.229, 0.224, 0.225], dtype=np.float32
        ).reshape(1, 3, 1, 1)

    def _preprocess(self, x: np.ndarray) -> np.ndarray:
        """
        Preprocess input array with batch dimensioning, scaling, and normalization.

        Args:
            x (np.ndarray): Input image array with shape [C, H, W] or [B, C, H, W].

        Returns:
            np.ndarray: Normalized float32 tensor with shape [B, 3, H, W].
        """
        # Ensure 4D tensor input [B, C, H, W]
        if x.ndim == 3:
            x = np.expand_dims(x, axis=0)

        # Convert to float32 if necessary
        if x.dtype != np.float32:
            x = x.astype(np.float32)

        # Scale uint8 values to [0.0, 1.0]
        if x.max() > 1.0:
            x = x / 255.0

        # Standardize using pre-cached ImageNet mean and std
        x = (x - self.mean) / self.std

        # Ensure array memory is contiguous for C-backend compatibility
        return np.ascontiguousarray(x, dtype=np.float32)

    def infer(self, input_data: np.ndarray) -> np.ndarray:
        """
        Execute forward pass on preprocessed image tensor.

        Args:
            input_data (np.ndarray): Raw input image tensor or numpy array.

        Returns:
            np.ndarray: Model output feature matrix.
        """
        processed_data = self._preprocess(input_data)

        # Execute ONNX runtime forward pass
        outputs = self.session.run(
            [self.output_name], {self.input_name: processed_data}
        )

        return np.asarray(outputs[0], dtype=np.float32)
