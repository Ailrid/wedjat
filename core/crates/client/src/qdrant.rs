//! Copyright (c) 2026-present Ailrid.
//!
//! Licensed under the Apache License, Version 2.0.
//!
//! Project: wedjat-core

use qdrant_client::{
    Qdrant,
    qdrant::{
        Condition, FieldCondition, Filter, GeoPoint, GeoRadius, QueryPointsBuilder, QueryResponse,
        ScrollPointsBuilder, condition::ConditionOneOf, point_id::PointIdOptions,
        vector_output::Vector,
    },
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::runtime::Runtime;
#[derive(Error, Debug)]
pub enum QdrantClientError {
    #[error("Qdrant error: {0}")]
    Qdrant(#[from] qdrant_client::QdrantError),
    #[error("Failed to parse payload: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Location {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub location: Location,
    pub x: u32,
    pub y: u32,
    pub src: String,
    pub res: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct QdrantResult {
    pub id: String,
    pub score: f32,
    pub payload: Payload,
}

/// Qdrant Vector Database Client.
///
/// # Fields
///
/// - `qdrant` (`Qdrant`) - Qdrant client.
/// - `collection_name` (`String`) - Collection name.
/// - `limit` (`usize`) - Limit of results.
/// - `radius` (`f32`) - Radius of the search area.
/// - `runtime` (`Runtime`) - Tokio runtime.
pub struct QdrantClient {
    qdrant: Qdrant,
    collection_name: String,
    limit: usize,
    radius: f32,
    runtime: Runtime,
}

impl QdrantClient {
    pub fn new(
        url: impl AsRef<str>,
        port: usize,
        collection_name: impl AsRef<str>,
        limit: usize,
        radius: f32,
    ) -> Result<Self, QdrantClientError> {
        let client = Qdrant::from_url(format!("{}:{}", url.as_ref(), port).as_str()).build()?;
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        Ok(Self {
            qdrant: client,
            collection_name: String::from(collection_name.as_ref()),
            limit,
            radius,
            runtime,
        })
    }
    fn convert_response(
        &self,
        response: QueryResponse,
    ) -> Result<Vec<QdrantResult>, QdrantClientError> {
        let mut results = Vec::new();

        for point in response.result {
            let json_value = serde_json::Value::Object(
                point
                    .payload
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect(),
            );

            let payload: Payload = serde_json::from_value(json_value)?;

            let point_id = point
                .id
                .and_then(|id| match id.point_id_options {
                    Some(PointIdOptions::Num(num)) => Some(num.to_string()),
                    Some(PointIdOptions::Uuid(uuid_str)) => Some(uuid_str),
                    None => None,
                })
                .unwrap_or_default();

            results.push(QdrantResult {
                id: point_id,
                score: point.score,
                payload,
            });
        }

        Ok(results)
    }
    /// Retrieve vectors from the global database.
    ///
    /// # Arguments
    ///
    /// - `vec` (`Vec<f32>`) - Feature vector.
    pub fn query_global(&self, vec: Vec<f32>) -> Result<Vec<QdrantResult>, QdrantClientError> {
        self.runtime.block_on(async {
            let query_request = QueryPointsBuilder::new(self.collection_name.clone())
                .query(vec)
                .limit(self.limit as u64)
                .with_payload(true);
            let response = self.qdrant.query(query_request).await?;
            self.convert_response(response)
        })
    }
    /// Using an index to query vectors from a database.
    ///
    /// # Arguments
    ///
    /// - `vec` (`Vec<f32>`) - Feature vector.
    /// - `lon` (`f64`) - Longitude.
    /// - `lat` (`f64`) - Latitude.

    pub fn query_filter(
        &self,
        vec: Vec<f32>,
        lon: f64,
        lat: f64,
    ) -> Result<Vec<QdrantResult>, QdrantClientError> {
        let geo_condition = Condition {
            condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                key: "location".to_string(),
                geo_radius: Some(GeoRadius {
                    center: Some(GeoPoint { lon, lat }),
                    radius: self.radius,
                }),
                ..Default::default()
            })),
        };

        let filter = Filter::must(vec![geo_condition]);

        self.runtime.block_on(async {
            let query_request = QueryPointsBuilder::new(self.collection_name.clone())
                .query(vec)
                .limit(self.limit as u64)
                .filter(filter)
                .with_payload(true);

            let response = self.qdrant.query(query_request).await?;
            self.convert_response(response)
        })
    }
}

#[derive(Debug, Clone)]
pub struct SamplePoint {
    pub id: String,
    pub vector: Vec<f32>,
}

impl QdrantClient {
    /// For testing purposes, randomly retrieve scroll data from the vector library.
    pub fn scroll_sample_points(&self, limit: u32) -> Result<Vec<SamplePoint>, QdrantClientError> {
        self.runtime.block_on(async {
            let request = ScrollPointsBuilder::new(self.collection_name.clone())
                .limit(limit)
                .with_vectors(true);

            let response = self.qdrant.scroll(request).await?;
            let mut samples = Vec::new();

            for point in response.result {
                let point_id = point
                    .id
                    .and_then(|id| match id.point_id_options {
                        Some(PointIdOptions::Num(num)) => Some(num.to_string()),
                        Some(PointIdOptions::Uuid(uuid_str)) => Some(uuid_str),
                        None => None,
                    })
                    .unwrap_or_default();

                if let Some(vectors) = point.vectors {
                    if let Some(Vector::Dense(dense)) = vectors.get_vector() {
                        samples.push(SamplePoint {
                            id: point_id,
                            vector: dense.data,
                        });
                    }
                }
            }

            Ok(samples)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[derive(Debug)]
    pub struct PerformanceReport {
        pub total_queries: usize,
        pub avg_latency_ms: f64,
        pub p95_latency_ms: f64,
        pub p99_latency_ms: f64,
        pub qps: f64,
        pub top1_recall_rate: f64,
    }

    fn print_performance_report(report: &PerformanceReport) {
        println!("\n{}", "=".repeat(40));
        println!("     TEST 1: PERFORMANCE REPORT     ");
        println!("{}", "=".repeat(40));
        println!("Total Queries:     {}", report.total_queries);
        println!("Average Latency:   {:.2} ms", report.avg_latency_ms);
        println!("P95 Latency:       {:.2} ms", report.p95_latency_ms);
        println!("P99 Latency:       {:.2} ms", report.p99_latency_ms);
        println!("Queries / Sec:     {:.2} QPS", report.qps);
        println!("Top-1 Recall Rate: {:.2}%", report.top1_recall_rate);
        println!("{}\n", "=".repeat(40));
    }

    fn run_performance_and_recall_test(
        client: &QdrantClient,
        num_queries: u32,
    ) -> Result<PerformanceReport, QdrantClientError> {
        println!(
            "=== Test 1: Fetching {} sample vectors from Qdrant ===",
            num_queries
        );

        let samples = client.scroll_sample_points(num_queries)?;

        if samples.is_empty() {
            println!("Error: Database collection is empty. Aborting Test 1.");
            return Ok(PerformanceReport {
                total_queries: 0,
                avg_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                qps: 0.0,
                top1_recall_rate: 0.0,
            });
        }

        println!(
            "=== Executing global search performance benchmark (n={}) ===",
            samples.len()
        );

        let mut latencies_sec: Vec<f64> = Vec::with_capacity(samples.len());
        let mut hits = 0;

        for sample in &samples {
            let start = Instant::now();

            let search_results = client.query_global(sample.vector.clone())?;
            let elapsed = start.elapsed().as_secs_f64();
            latencies_sec.push(elapsed);

            if let Some(top1_item) = search_results.first() {
                if top1_item.id == sample.id {
                    hits += 1;
                }
            }
        }

        let total_queries = samples.len();
        let total_time_sec: f64 = latencies_sec.iter().sum();
        let avg_latency_sec = total_time_sec / (total_queries as f64);
        let avg_latency_ms = avg_latency_sec * 1000.0;

        let mut sorted_latencies = latencies_sec;
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p95_index = ((total_queries as f64) * 0.95) as usize;
        let p99_index = ((total_queries as f64) * 0.99) as usize;

        let p95_latency_ms = sorted_latencies[p95_index.min(total_queries - 1)] * 1000.0;
        let p99_latency_ms = sorted_latencies[p99_index.min(total_queries - 1)] * 1000.0;

        let qps = if avg_latency_sec > 0.0 {
            1.0 / avg_latency_sec
        } else {
            0.0
        };

        let top1_recall_rate = (hits as f64 / total_queries as f64) * 100.0;

        let report = PerformanceReport {
            total_queries,
            avg_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            qps,
            top1_recall_rate,
        };

        print_performance_report(&report);

        Ok(report)
    }

    #[test]
    fn test_performance_and_recall() {
        let url = "http://127.0.0.1";
        let port = 6334;
        let collection_name = "test_tiff";
        let limit = 1;
        let radius = 500.0;

        let client = QdrantClient::new(url, port, collection_name, limit, radius)
            .expect("Failed to initialize QdrantClient");

        let report = run_performance_and_recall_test(&client, 500)
            .expect("Performance benchmark test failed");

        if report.total_queries > 0 {
            assert!(
                report.top1_recall_rate >= 99.0,
                "Recall rate is too low: {:.2}%",
                report.top1_recall_rate
            );
        }
    }
}
