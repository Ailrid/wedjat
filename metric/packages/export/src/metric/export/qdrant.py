"""
Copyright (c) 2026-present Ailrid.
Licensed under the Apache License, Version 2.0.
Project: wedjat-metric
"""

import os
import time
import uuid
from typing import List, Dict, Optional
import random
from xml.dom import NotFoundErr
import numpy as np
import rasterio
from rasterio.warp import transform
from rasterio.windows import Window
from qdrant_client import QdrantClient
from qdrant_client.models import (
    Distance,
    HnswConfigDiff,
    PayloadSchemaType,
    PointStruct,
    VectorParams,
    PointStruct,
)
from tqdm import tqdm
from .refer import ONNXInferencer
import matplotlib.pyplot as plt


class QdrantClientManager:
    """
    Qdrant database management class with geo-indexing and disk-mapped vector support.
    """

    def __init__(self, host: str = "127.0.0.1", port: int = 6333) -> None:
        """
        Initialize Qdrant client.

        Args:
            host (str): Qdrant server hostname.
            port (int): HTTP port number.
        """
        self.client: QdrantClient = QdrantClient(host=host, port=port)

    def setup_collection(self, collection_name: str, vector_size: int) -> None:
        """
        Create collection and setup geo-spatial and keyword payload indices.

        Args:
            collection_name (str): Name of the collection.
            vector_size (int): Dimension of feature vectors.
        """
        if not self.client.collection_exists(collection_name):
            self.client.create_collection(
                collection_name=collection_name,
                vectors_config=VectorParams(
                    size=vector_size,
                    distance=Distance.COSINE,
                    on_disk=True,  # Store vectors on disk to reduce RAM footprint
                ),
                hnsw_config=HnswConfigDiff(on_disk=True),
            )

            # Create GEO payload index for spatial radius/bounding-box filtering
            self.client.create_payload_index(
                collection_name=collection_name,
                field_name="location",
                field_schema=PayloadSchemaType.GEO,
            )

            # Create KEYWORD payload index for source filename filtering
            self.client.create_payload_index(
                collection_name=collection_name,
                field_name="src",
                field_schema=PayloadSchemaType.KEYWORD,
            )

            print(f"Collection {collection_name} initialized with GEO payload index.")

    def upsert_batch(self, collection_name: str, points: List[PointStruct]) -> None:
        """
        Upsert a batch of points to Qdrant.

        Args:
            collection_name (str): Target collection name.
            points (List[PointStruct]): List of point structures.
        """
        if points:
            self.client.upsert(collection_name=collection_name, points=points)

    def upload_all_parallel(
        self, collection_name: str, points: List[PointStruct]
    ) -> None:
        """
        Parallel upload points using Qdrant client's high-performance batching interface.

        Args:
            collection_name (str): Name of the target collection.
            points (List[PointStruct]): Points to upload.
        """
        if not points:
            return

        self.client.upload_points(
            collection_name=collection_name,
            points=points,
            batch_size=128,
            parallel=4,
            wait=False,
        )


class TiffProcessor:
    """
    TIFF image processing and vector ingestion processor.
    """

    def __init__(
        self, model_path: str, crop_size: int = 256, overlap: float = 0.5
    ) -> None:
        """
        Initialize processor with feature inference engine and patch configurations.
        """
        self.inferencer: ONNXInferencer = ONNXInferencer(model_path)
        self.crop_size: int = crop_size
        self.stride: int = int(crop_size * (1 - overlap))
        self.db: QdrantClientManager = QdrantClientManager()

    def run_ingestion(
        self, tif_path: str, collection_name: str, vector_size: int = 512
    ) -> None:
        """
        Extract patch features and ingest into Qdrant with WGS84 geographic coordinates.

        Args:
            tif_path (str): Path to input GeoTIFF file.
            collection_name (str): Target Qdrant collection name.
            vector_size (int): Output embedding dimension size.
        """
        if not os.path.exists(tif_path):
            raise NotFoundErr(f"Error: File not found {tif_path}")

        with rasterio.open(tif_path) as src:
            src_crs = src.crs
            res_x, res_y = src.res
            width, height = src.width, src.height

            if src_crs is None:
                raise NotFoundErr("Error: CRS not found")

            self.db.setup_collection(collection_name, vector_size=vector_size)

            x_steps: List[int] = list(range(0, width - self.crop_size + 1, self.stride))
            y_steps: List[int] = list(
                range(0, height - self.crop_size + 1, self.stride)
            )
            total_steps: int = len(x_steps) * len(y_steps)

            buffer_pool: List[PointStruct] = []
            processed_count: int = 0
            skipped_count: int = 0

            print(
                f"Processing: {os.path.basename(tif_path)} ({total_steps} blocks total)"
            )

            with tqdm(total=total_steps, desc="Ingesting", unit="patch") as pbar:
                for y in y_steps:
                    for x in x_steps:
                        window = Window(x, y, self.crop_size, self.crop_size)  # type: ignore
                        data: np.ndarray = src.read([1, 2, 3], window=window)

                        if np.all(data == 0):
                            skipped_count += 1
                            pbar.update(1)
                            continue

                        # Extract feature vector using ONNX inferencer
                        feat: np.ndarray = self.inferencer.infer(data)
                        assert feat.shape == (vector_size,)

                        # Compute local image coordinate to CRS coordinate
                        projected_x, projected_y = src.xy(y, x)

                        # Convert native CRS coordinate to WGS84 (EPSG:4326) longitude and latitude
                        longitudes, latitudes = transform(  # type: ignore
                            src_crs, "EPSG:4326", [projected_x], [projected_y]
                        )
                        lon: float = float(longitudes[0])
                        lat: float = float(latitudes[0])
                        buffer_pool.append(
                            PointStruct(
                                id=str(uuid.uuid4()),
                                vector=feat.flatten().tolist(),
                                payload={
                                    "location": {
                                        "lon": lon,
                                        "lat": lat,
                                    },
                                    "x": x,
                                    "y": y,
                                    "src": os.path.basename(tif_path),
                                    "res": [float(res_x), float(res_y)],
                                },
                            )
                        )
                        processed_count += 1

                        # Ingest via parallel queue once batch buffer is filled
                        if len(buffer_pool) >= 5000:
                            self.db.upload_all_parallel(collection_name, buffer_pool)
                            buffer_pool = []
                            pbar.set_postfix({"in-db": processed_count})

                        pbar.update(1)

                # Flush remaining buffer points
                if buffer_pool:
                    self.db.upload_all_parallel(collection_name, buffer_pool)

            print(f"\nIngestion completed. Total valid points: {processed_count}")


class QdrantPerformanceTester:
    """
    Qdrant performance and real-time inference retrieval benchmark suite.
    """

    def __init__(
        self,
        model_path: str,
        tif_path: str,
        collection_name: str,
        crop_size: int = 256,
        host: str = "127.0.0.1",
        port: int = 6333,
    ) -> None:
        """
        Initialize the benchmark tester instance.

        Args:
            model_path (str): Path to the ONNX model file.
            tif_path (str): Path to the target GeoTIFF file.
            collection_name (str): Target Qdrant collection name.
            crop_size (int): Image patch crop size in pixels.
            host (str): Qdrant host address.
            port (int): Qdrant HTTP port.
        """
        self.tif_path: str = tif_path
        self.collection_name: str = collection_name
        self.crop_size: int = crop_size

        # Initialize ONNX inference engine and Qdrant client
        self.inferencer: ONNXInferencer = ONNXInferencer(model_path)
        self.client: QdrantClient = QdrantClient(host=host, port=port)

    def test_performance_and_recall(self, num_queries: int = 500) -> None:
        """
        Test 1: Global vector search latency, throughput (QPS), and Top-1 recall
        using pre-extracted vectors from Qdrant.

        Args:
            num_queries (int): Number of sample vectors to fetch and test.
        """
        print(f"=== Test 1: Fetching {num_queries} sample vectors from Qdrant ===")
        scroll_res, _ = self.client.scroll(
            collection_name=self.collection_name,
            limit=num_queries,
            with_vectors=True,
            with_payload=True,
        )

        if not scroll_res:
            print("Error: Database collection is empty. Aborting Test 1.")
            return

        print(
            f"=== Executing global search performance benchmark (n={len(scroll_res)}) ==="
        )
        latencies: List[float] = []
        hits: int = 0

        for test_point in scroll_res:
            query_vector = test_point.vector
            expected_id = test_point.id

            query_start = time.perf_counter()

            # Global vector search without filter
            search_result = self.client.query_points(
                collection_name=self.collection_name,
                query=query_vector,  # type: ignore
                limit=1,
            )

            query_end = time.perf_counter()
            latencies.append(query_end - query_start)

            # Top-1 accuracy verification
            if search_result.points and search_result.points[0].id == expected_id:
                hits += 1

        # Output performance report
        avg_latency = float(np.mean(latencies)) * 1000.0
        p95_latency = float(np.percentile(latencies, 95)) * 1000.0
        p99_latency = float(np.percentile(latencies, 99)) * 1000.0
        qps = 1.0 / np.mean(latencies) if np.mean(latencies) > 0 else 0.0
        accuracy = (hits / len(scroll_res)) * 100.0

        print("\n" + "=" * 40)
        print("     TEST 1: PERFORMANCE REPORT     ")
        print("=" * 40)
        print(f"Total Queries:     {len(scroll_res)}")
        print(f"Average Latency:   {avg_latency:.2f} ms")
        print(f"P95 Latency:       {p95_latency:.2f} ms")
        print(f"P99 Latency:       {p99_latency:.2f} ms")
        print(f"Queries / Sec:     {qps:.2f} QPS")
        print(f"Top-1 Recall Rate: {accuracy:.2f}%")
        print("=" * 40 + "\n")

    def test_realtime_crop_and_rank(
        self,
        num_samples: int = 200,
        visualize_top_k: int = 3,
        output_dir: str = "search_visualizations",
    ) -> None:
        """
        Test 2: Randomly crop patches from GeoTIFF, run real-time ONNX inference,
        perform global Rank-5 vector search, evaluate Rank 1-5 hit rates, and
        visualize query vs. top-5 search results.
        """
        if not os.path.exists(self.tif_path):
            print(f"Error: TIFF file not found: {self.tif_path}")
            return

        print(
            f"=== Test 2: Crop {num_samples} patches, run ONNX inference, and search Rank-5 ==="
        )

        rank_hits: Dict[int, int] = {1: 0, 2: 0, 3: 0, 4: 0, 5: 0}
        valid_queries: int = 0
        visualized_count: int = 0

        # Create output directory for plots if visualization is enabled
        if visualize_top_k > 0:
            os.makedirs(output_dir, exist_ok=True)

        with rasterio.open(self.tif_path) as src:
            width, height = src.width, src.height
            src_filename = os.path.basename(self.tif_path)

            if width < self.crop_size or height < self.crop_size:
                print("Error: TIFF dimensions smaller than patch size.")
                return

            for _ in range(num_samples):
                # Randomly select top-left pixel coordinates
                x = random.randint(0, width - self.crop_size)
                y = random.randint(0, height - self.crop_size)

                window = Window(x, y, self.crop_size, self.crop_size)  # type: ignore
                data: np.ndarray = src.read([1, 2, 3], window=window)

                # Skip completely black/nodata patches
                if np.all(data == 0):
                    continue

                valid_queries += 1

                # Real-time inference
                feat: np.ndarray = self.inferencer.infer(data)
                query_vector = feat.flatten().tolist()

                # Execute global Top-5 query
                search_result = self.client.query_points(
                    collection_name=self.collection_name,
                    query=query_vector,
                    limit=5,
                )

                points = search_result.points
                hit_rank: Optional[int] = None

                # Check if matches come from the current source file and nearby pixel location
                for rank_idx, point in enumerate(points, start=1):
                    payload = point.payload or {}
                    match_src = payload.get("src")
                    px = payload.get("col")
                    py = payload.get("row")

                    # Check source file match and bounding pixel offset tolerance (within crop_size)
                    if (
                        match_src == src_filename
                        and px is not None
                        and py is not None
                        and abs(px - x) < self.crop_size
                        and abs(py - y) < self.crop_size
                    ):
                        hit_rank = rank_idx
                        # Accumulate hit for current rank and all subsequent ranks (Cumulative Hit Rate)
                        for r in range(rank_idx, 6):
                            rank_hits[r] += 1
                        break

                # Render and save search visualization plot for first N valid queries
                if visualized_count < visualize_top_k:
                    visualized_count += 1
                    self._plot_search_result(
                        src_raster=src,
                        query_data=data,
                        query_pos=(x, y),
                        search_points=points,
                        hit_rank=hit_rank,
                        sample_idx=visualized_count,
                        output_dir=output_dir,
                    )

        if valid_queries == 0:
            print("Error: No valid non-zero image patches cropped.")
            return

        print("\n" + "=" * 40)
        print("   TEST 2: REAL-TIME RANK-5 HIT REPORT   ")
        print("=" * 40)
        print(f"Valid Test Crops:  {valid_queries}")
        for r in range(1, 6):
            hit_rate = (rank_hits[r] / valid_queries) * 100.0
            print(
                f"Top-{r} Hit Rate:     {hit_rate:.2f}% ({rank_hits[r]}/{valid_queries})"
            )
        print("=" * 40)

    def _plot_search_result(
        self,
        src_raster: rasterio.DatasetReader,
        query_data: np.ndarray,
        query_pos: tuple,
        search_points: List,
        hit_rank: Optional[int],
        sample_idx: int,
        output_dir: str,
    ) -> None:
        """
        Helper method to render and save Query image vs. Top-5 retrieved patches side-by-side.
        """
        fig, axes = plt.subplots(1, 6, figsize=(18, 3.5))

        # Format Query Patch (CHW -> HWC)
        query_img = np.transpose(query_data, (1, 2, 0))
        if query_img.max() > 1.0:
            query_img = query_img / 255.0
        query_img = np.clip(query_img, 0, 1)

        qx, qy = query_pos
        axes[0].imshow(query_img)
        axes[0].set_title(f"Query Patch\nPos: ({qx}, {qy})", fontsize=10, color="blue")
        axes[0].axis("off")

        src_filename = os.path.basename(self.tif_path)

        # Plot Top-5 Retrieved Patches
        for idx in range(5):
            ax = axes[idx + 1]

            if idx < len(search_points):
                point = search_points[idx]
                payload = point.payload or {}
                px = payload.get("pixel_x")
                py = payload.get("pixel_y")
                match_src = payload.get("src")
                score = getattr(point, "score", 0.0)

                # Read patch image from GeoTIFF if pixel coordinates exist
                if px is not None and py is not None:
                    window = Window(
                        px, py, self.crop_size, self.crop_size  # type: ignore
                    )  # type: ignore
                    patch_data = src_raster.read([1, 2, 3], window=window)
                    patch_img = np.transpose(patch_data, (1, 2, 0))
                    if patch_img.max() > 1.0:
                        patch_img = patch_img / 255.0
                    patch_img = np.clip(patch_img, 0, 1)

                    ax.imshow(patch_img)

                    # Determine if current patch is a true ground-truth hit
                    is_hit = (
                        match_src == src_filename
                        and abs(px - qx) < self.crop_size
                        and abs(py - qy) < self.crop_size
                    )

                    title_color = "green" if is_hit else "red"
                    status_text = "MATCH" if is_hit else "MISS"

                    ax.set_title(
                        f"Rank {idx+1} [{status_text}]\nScore: {score:.4f}\nPos: ({px}, {py})",
                        fontsize=9,
                        color=title_color,
                    )
                else:
                    ax.text(0.5, 0.5, "No Location", ha="center", va="center")
                    ax.set_title(f"Rank {idx+1}", fontsize=9)
            else:
                ax.text(0.5, 0.5, "Empty", ha="center", va="center")
                ax.set_title(f"Rank {idx+1}", fontsize=9)

            ax.axis("off")

        plt.suptitle(
            f"Sample #{sample_idx} - Realtime Query Search (Hit Rank: {hit_rank if hit_rank else 'None'})",
            fontsize=12,
            fontweight="bold",
        )
        plt.tight_layout()

        # Save image to specified directory
        save_path = os.path.join(output_dir, f"query_sample_{sample_idx}_rank5.png")
        plt.savefig(save_path, dpi=150, bbox_inches="tight")
        plt.close(fig)
        print(f"Saved visualization plot: {save_path}")
