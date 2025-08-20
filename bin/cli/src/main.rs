use clap::{Parser, Subcommand};
use miden_objects::{note::Note, utils::Deserializable};
use miden_private_transport::{
    Error, Result,
    client::{
        FilesystemEncryptionStore, TransportLayerClient, crypto::aes::Aes256GcmKey,
        grpc::GrpcClient,
    },
    logging::{OpenTelemetry, setup_tracing},
};
use tracing::info;

#[derive(Parser)]
#[command(name = "miden-private-transport-cli")]
#[command(
    about = "Miden Private Transport CLI - Test client for the private notes transport layer"
)]
struct Args {
    /// Server endpoint
    #[arg(long, default_value = "http://localhost:8080")]
    endpoint: String,

    /// Request timeout (ms)
    #[arg(long, default_value = "1000")]
    timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a note to a recipient
    Send {
        /// Note data (hex encoded)
        #[arg(long)]
        note: String,

        /// Recipient's public key
        #[arg(long)]
        key: String,
    },

    /// Fetch notes for a tag
    Fetch {
        /// Note tag
        #[arg(long)]
        tag: u32,

        /// Recipient's private key
        #[arg(long)]
        key: String,
    },

    /// Generate a new encryption key
    GenerateKey,

    /// Check node health
    Health,

    /// Get node statistics
    Stats,

    /// Random note for testing purposes
    TestNote,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing(OpenTelemetry::Enabled)?;

    // Parse command line arguments
    let args = Args::parse();

    info!("Miden Transport CLI");
    info!("Endpoint: {}", args.endpoint);

    // Create client
    let grpc = GrpcClient::connect(args.endpoint, args.timeout).await?;
    let encryption_store = FilesystemEncryptionStore::new("./keys")?;
    let mut client = TransportLayerClient::new(Box::new(grpc), Box::new(encryption_store));

    match args.command {
        Commands::Send { note, key } => {
            send_note(&mut client, &note, &key).await?;
        },
        Commands::Fetch { tag, key } => {
            fetch_notes(&mut client, tag, &key).await?;
        },
        Commands::GenerateKey => {
            generate_key();
        },
        Commands::Health => {
            health_check(&client);
        },
        Commands::Stats => {
            get_stats(&client);
        },
        Commands::TestNote => {
            mock_note();
        },
    }

    Ok(())
}

async fn send_note(
    client: &mut TransportLayerClient,
    data: &str,
    recipient_key: &str,
) -> Result<()> {
    let bytes = hex::decode(data).map_err(|e| {
        miden_private_transport::Error::InvalidNoteData(format!("Invalid hex data: {e}"))
    })?;

    let note = Note::read_from_bytes(&bytes)
        .map_err(|e| Error::InvalidNoteData(format!("Failed to deserialize Note: {e}")))?;

    // Decode hex recipient key
    let pub_key = hex::decode(recipient_key).map_err(|e| {
        miden_private_transport::Error::InvalidNoteData(format!("Invalid hex key: {e}"))
    })?;

    info!("Sending note to tag {}", note.header().metadata().tag());

    // Send the note
    client.send_note(note, &pub_key).await?;
    info!("Note sent successfully");

    Ok(())
}

async fn fetch_notes(client: &mut TransportLayerClient, tag: u32, private_key: &str) -> Result<()> {
    info!("Fetching notes for tag {}", tag);

    // Decode hex private key
    let key = hex::decode(private_key).map_err(|e| {
        miden_private_transport::Error::InvalidNoteData(format!("Invalid hex key: {e}"))
    })?;

    // Fetch notes
    let decrypted_notes = client.fetch_notes(tag.into(), &key).await?;

    info!("Found {} notes", decrypted_notes.len());

    for (i, (header, details)) in decrypted_notes.iter().enumerate() {
        println!("Note {}:\n Header: {:?}\n Details: {:?}", i + 1, header, details);
    }

    Ok(())
}

fn generate_key() {
    let key = Aes256GcmKey::generate();
    let hex_key = hex::encode(key.as_bytes());
    println!("Generated encryption key: {hex_key}");
}

fn health_check(_client: &TransportLayerClient) {
    info!("Checking node health");

    // For now, we'll need to access the API client directly
    // This is a limitation of the current TransportLayerClient design
    println!("❌ Health check not implemented in TransportLayerClient");
    println!("Use GrpcClient directly for health checks");
}

fn get_stats(_client: &TransportLayerClient) {
    info!("Getting node statistics");

    // For now, we'll need to access the API client directly
    // This is a limitation of the current TransportLayerClient design
    println!("❌ Stats not implemented in TransportLayerClient");
    println!("Use GrpcClient directly for statistics");
}

fn mock_note() {
    use miden_objects::utils::Serializable;
    let note = miden_private_transport::types::mock_note_p2id();
    let hex_note = hex::encode(note.to_bytes());
    info!("Test note: {}", hex_note);
}
