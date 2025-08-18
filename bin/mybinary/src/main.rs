use miden_mycrate::logging::{OpenTelemetry, setup_tracing};

// TODO(template) update for the binary
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing(OpenTelemetry::Enabled)?;
    tracing::info!("hello Miden!");
    Ok(())
}
