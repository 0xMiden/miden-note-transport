//! Load Testing Tool for Miden Private Transport

use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tracing::info;

pub mod grpc;
pub mod utils;

use grpc::GrpcStress;

#[derive(Parser)]
#[command(name = "miden-load-test")]
#[command(about = "Load testing tool for Miden Private Transport")]
struct Args {
    /// Server host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Number of concurrent workers
    #[arg(long, default_value = "10")]
    workers: usize,

    /// Total number of requests to send
    #[arg(long, default_value = "1000")]
    requests: usize,

    /// Test scenario to run
    #[arg(long, default_value = "mixed")]
    scenario: String,

    /// Request rate (requests per second)
    #[arg(long)]
    rate: Option<f64>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone)]
struct TestMetrics {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    total_duration: Duration,
    min_latency: Duration,
    max_latency: Duration,
    avg_latency: Duration,
    requests_per_second: f64,
}

#[derive(Debug)]
struct RequestResult {
    success: bool,
    latency: Duration,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::init();
    }

    let endpoint = format!("http://{}:{}", args.host, args.port);
    info!("Starting load test against: {}", endpoint);

    // Run the load test
    let metrics = match args.scenario.as_str() {
        "send_note" => GrpcStress::new(endpoint, args.workers, args.requests, args.rate).send_note().await?,
        "fetch_notes" => GrpcStress::new(endpoint, args.workers, args.requests, args.rate).fetch_notes().await?,
        "mixed" => GrpcStress::new(endpoint, args.workers, args.requests, args.rate).mixed().await?,
        _ => {
            eprintln!("Unknown scenario: {}", args.scenario);
            eprintln!("Available scenarios: send_note, fetch_notes, mixed");
            return Ok(());
        }
    };

    // Print results
    print_metrics(&metrics);

    Ok(())
}


fn print_metrics(metrics: &TestMetrics) {
    println!("\n=== LOAD TEST RESULTS ===");
    println!("Total Requests: {}", metrics.total_requests);
    println!("Successful: {} ({:.1}%)", 
        metrics.successful_requests, 
        (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
    );
    println!("Failed: {} ({:.1}%)", 
        metrics.failed_requests, 
        (metrics.failed_requests as f64 / metrics.total_requests as f64) * 100.0
    );
    println!("Total Duration: {:.2}s", metrics.total_duration.as_secs_f64());
    println!("Requests/sec: {:.2}", metrics.requests_per_second);
    println!("Min Latency: {:.2}ms", metrics.min_latency.as_secs_f64() * 1000.0);
    println!("Max Latency: {:.2}ms", metrics.max_latency.as_secs_f64() * 1000.0);
    println!("Avg Latency: {:.2}ms", metrics.avg_latency.as_secs_f64() * 1000.0);
    println!("========================");
}
