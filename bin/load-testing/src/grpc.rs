use anyhow::Result;
use miden_private_transport_proto::miden_private_transport::{
    miden_private_transport_client::MidenPrivateTransportClient,
    FetchNotesRequest,
};
use prost_types::Timestamp;
use rand::Rng;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tonic::Request;
use tracing::{info, warn};

use crate::{TestMetrics, RequestResult};
use super::utils::generate_dummy_notes;
use miden_private_transport_client::GrpcClient;

#[derive(Clone)]
pub struct GrpcStress {
    endpoint: String,
    workers: usize,
    requests: usize,
    rate: Option<f64>,
}

impl GrpcStress {
    pub fn new(endpoint: String, workers: usize, requests: usize, rate: Option<f64>) -> Self {
        Self {
            endpoint, workers, requests, rate
        }
    }

    pub async fn send_note(
        &self,
    ) -> Result<TestMetrics> {
        info!("Running SendNote load test");

        let (tx, mut rx) = mpsc::channel(1000);
        let mut handles = vec![];

        let start_time = Instant::now();

        // Spawn workers
        for _worker_id in 0..self.workers {
            let cfg = self.clone();
            let tx = tx.clone();

            let handle = tokio::spawn(async move {
                let mut client = GrpcClient::connect(cfg.endpoint, 1000).await.unwrap();

                let mut request_count = 0;

                loop {
                    // Generate test note
                    let test_note = &generate_dummy_notes(1)[0];

                    let request_start = Instant::now();
                    let result = client.send_note(test_note.0, test_note.1.clone()).await;
                    let latency = request_start.elapsed();

                    let success = result.is_ok();
                    let error = result.err().map(|e| e.to_string());

                    let _ = tx.send(RequestResult {
                        success,
                        latency,
                        error,
                    }).await;

                    request_count += 1;

                    // Rate limiting
                    if let Some(rate) = cfg.rate {
                        let delay = Duration::from_secs_f64(1.0 / rate);
                        sleep(delay).await;
                    }

                    // Check if we should stop
                    if request_count >= cfg.requests / cfg.workers {
                        break;
                    }
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut total_requests = 0;
        let mut successful_requests = 0;
        let mut failed_requests = 0;
        let mut min_latency = Duration::MAX;
        let mut max_latency = Duration::ZERO;
        let mut total_latency = Duration::ZERO;

        while let Some(result) = rx.recv().await {
            total_requests += 1;

            if result.success {
                successful_requests += 1;
            } else {
                failed_requests += 1;
                warn!("Request failed: {:?}", result.error);
            }

            min_latency = min_latency.min(result.latency);
            max_latency = max_latency.max(result.latency);
            total_latency += result.latency;

            if total_requests >= self.requests {
                break;
            }
        }

        // Wait for all workers to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed();
        let avg_latency = if total_requests > 0 {
            Duration::from_nanos(total_latency.as_nanos() as u64 / total_requests as u64)
        } else {
            Duration::ZERO
        };

        let requests_per_second = if total_duration.as_secs_f64() > 0.0 {
            total_requests as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(TestMetrics {
            total_requests,
            successful_requests,
            failed_requests,
            total_duration,
            min_latency,
            max_latency,
            avg_latency,
            requests_per_second,
        })
    }

    pub async fn fetch_notes(
        &self,
    ) -> Result<TestMetrics> {
        info!("Running FetchNotes load test");

        let (tx, mut rx) = mpsc::channel(1000);
        let mut handles = vec![];

        let start_time = Instant::now();

        // Spawn workers
        for _worker_id in 0..self.workers {
            let cfg = self.clone();
            let tx = tx.clone();

            let handle = tokio::spawn(async move {
                let mut client = GrpcClient::connect(cfg.endpoint, 1000).await.unwrap();

                let mut request_count = 0;
                let mut tag = super::utils::TAG_LOCAL_ANY;

                loop {
                    tag += 1;

                    let request_start = Instant::now();
                    let result = client.fetch_notes(tag.into()).await;
                    let latency = request_start.elapsed();

                    let success = result.is_ok();
                    let error = result.err().map(|e| e.to_string());

                    let _ = tx.send(RequestResult {
                        success,
                        latency,
                        error,
                    }).await;

                    request_count += 1;

                    // Rate limiting
                    if let Some(rate) = cfg.rate {
                        let delay = Duration::from_secs_f64(1.0 / rate);
                        sleep(delay).await;
                    }

                    // Check if we should stop
                    if request_count >= cfg.requests / cfg.workers {
                        break;
                    }
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut total_requests = 0;
        let mut successful_requests = 0;
        let mut failed_requests = 0;
        let mut min_latency = Duration::MAX;
        let mut max_latency = Duration::ZERO;
        let mut total_latency = Duration::ZERO;

        while let Some(result) = rx.recv().await {
            total_requests += 1;

            if result.success {
                successful_requests += 1;
            } else {
                failed_requests += 1;
                warn!("Request failed: {:?}", result.error);
            }

            min_latency = min_latency.min(result.latency);
            max_latency = max_latency.max(result.latency);
            total_latency += result.latency;

            if total_requests >= self.requests {
                break;
            }
        }

        // Wait for all workers to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed();
        let avg_latency = if total_requests > 0 {
            Duration::from_nanos(total_latency.as_nanos() as u64 / total_requests as u64)
        } else {
            Duration::ZERO
        };

        let requests_per_second = if total_duration.as_secs_f64() > 0.0 {
            total_requests as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(TestMetrics {
            total_requests,
            successful_requests,
            failed_requests,
            total_duration,
            min_latency,
            max_latency,
            avg_latency,
            requests_per_second,
        })
    }

    pub async fn mixed(
        &self
    ) -> Result<TestMetrics> {
        info!("Running mixed load test (SendNote + FetchNotes)");

        let cfg = Self::new(self.endpoint.clone(), self.workers / 2, self.requests / 2, self.rate);

        // Run both tests and combine metrics
        let send_note_metrics = cfg.send_note().await?;
        let fetch_notes_metrics = cfg.fetch_notes().await?;

        // Combine metrics
        Ok(TestMetrics {
            total_requests: send_note_metrics.total_requests + fetch_notes_metrics.total_requests,
            successful_requests: send_note_metrics.successful_requests + fetch_notes_metrics.successful_requests,
            failed_requests: send_note_metrics.failed_requests + fetch_notes_metrics.failed_requests,
            total_duration: send_note_metrics.total_duration.max(fetch_notes_metrics.total_duration),
            min_latency: send_note_metrics.min_latency.min(fetch_notes_metrics.min_latency),
            max_latency: send_note_metrics.max_latency.max(fetch_notes_metrics.max_latency),
            avg_latency: Duration::from_nanos(
                ((send_note_metrics.avg_latency.as_nanos() + fetch_notes_metrics.avg_latency.as_nanos()) / 2) as u64
            ),
            requests_per_second: send_note_metrics.requests_per_second + fetch_notes_metrics.requests_per_second,
        })
    }
}
