use std::path::PathBuf;

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use miden_objects::{account::NetworkId, address::Address, note::Note, utils::Deserializable};
use miden_private_transport_client::{
    Error, Result, TransportLayerClient,
    database::DatabaseConfig,
    grpc::GrpcClient,
    logging::{OpenTelemetry, setup_tracing},
    types::{mock_address, mock_note_p2id_with_addresses},
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

    /// Database path for persistence
    #[arg(long, default_value = "cli-db.sqlite")]
    database: PathBuf,

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

        /// Recipient's address (bech32)
        #[arg(long)]
        recipient: String,
    },

    /// Fetch notes for a tag
    Fetch {
        /// Note tag
        #[arg(long)]
        tag: u32,
    },

    /// Initialize the client with an address
    Init {
        /// Listening address (bech32)
        #[arg(long)]
        address: String,
    },

    /// Clean up old data
    Cleanup {
        /// Retention period in days
        days: u32,
    },

    /// Register a tag for listening
    RegisterTag {
        /// Tag to register
        #[arg(long)]
        tag: u32,
    },

    /// Random note for testing purposes
    TestNote {
        /// Recipient account ID
        #[arg(long)]
        recipient: String,
    },

    /// Random address (bech32) for testing purposes
    TestAddress,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing(OpenTelemetry::Enabled)?;

    // Parse command line arguments
    let args = Args::parse();

    info!("Miden Transport CLI");
    info!("Endpoint: {}", args.endpoint);
    info!("Database: {:?}", args.database);
    info!("Database path: {:?}", args.database);

    let db_config = DatabaseConfig {
        url: args.database.to_string_lossy().to_string(),
        max_note_size: 1024 * 1024, // 1MB
    };

    // Create client
    let grpc = GrpcClient::connect(args.endpoint, args.timeout).await?;
    let mut client = TransportLayerClient::init(Box::new(grpc), vec![], Some(db_config)).await?;

    match args.command {
        Commands::Send { note, recipient } => {
            send_note(&mut client, &note, &recipient).await?;
        },
        Commands::Fetch { tag } => {
            fetch_notes(&mut client, tag).await?;
        },
        Commands::Init { address } => {
            init(&mut client, &address)?;
        },
        Commands::Cleanup { days } => {
            cleanup_old_data(&client, days).await?;
        },
        Commands::RegisterTag { tag } => {
            client.register_tag(tag.into())?;
            println!("✅ Tag {tag} registered successfully");
        },
        Commands::TestNote { recipient } => {
            mock_note(&recipient)?;
        },
        Commands::TestAddress => {
            test_address();
        },
    }

    Ok(())
}

async fn send_note(
    client: &mut TransportLayerClient,
    data: &str,
    recipient_address_bech32: &str,
) -> Result<()> {
    let bytes =
        hex::decode(data).map_err(|e| Error::InvalidNoteData(format!("Invalid hex data: {e}")))?;

    let note = Note::read_from_bytes(&bytes)
        .map_err(|e| Error::InvalidNoteData(format!("Failed to deserialize Note: {e}")))?;

    info!("Sending note to tag {}", note.header().metadata().tag());

    let (_, address) = Address::from_bech32(recipient_address_bech32)
        .map_err(|e| anyhow!("Invalid recipient address {recipient_address_bech32}: {e}"))?;

    // Send the note
    client.send_note(note, Some(&address)).await?;
    info!("Note sent successfully");

    Ok(())
}

async fn fetch_notes(client: &mut TransportLayerClient, tag: u32) -> Result<()> {
    // Fetch notes
    let decrypted_notes = client.fetch_notes(tag.into()).await?;

    info!("Found {} notes", decrypted_notes.len());

    for (i, (header, details)) in decrypted_notes.iter().enumerate() {
        println!("Note {}:\n Header: {:?}\n Details: {:?}", i + 1, header, details);
    }

    Ok(())
}

fn init(client: &mut TransportLayerClient, address_bech32: &str) -> Result<()> {
    let (_, address) = Address::from_bech32(address_bech32)
        .map_err(|e| anyhow!("Invalid recipient address {address_bech32}: {e}"))?;

    let tag = address.to_note_tag();
    client.add_address(address);
    // By default, register NoteTag associated with this Address
    client.register_tag(tag)?;

    info!("Successfully initialized client with address {address_bech32} (tag: {tag})");

    Ok(())
}

async fn cleanup_old_data(client: &TransportLayerClient, days: u32) -> Result<()> {
    info!("Cleaning up data older than {} days", days);

    match client.cleanup_old_data(days).await {
        Ok(deleted_count) => {
            println!("✅ Cleaned up {deleted_count} old records");
        },
        Err(e) => {
            println!("❌ Cleanup failed: {e}");
        },
    }

    Ok(())
}

fn mock_note(recipient_address_bech32: &str) -> Result<()> {
    use miden_objects::utils::Serializable;
    let (_, address) = Address::from_bech32(recipient_address_bech32)
        .map_err(|e| anyhow!("Invalid recipient address {recipient_address_bech32}: {e}"))?;
    let note = mock_note_p2id_with_addresses(&mock_address(), &address);
    let hex_note = hex::encode(note.to_bytes());
    info!("Test note: {}", hex_note);
    Ok(())
}

fn test_address() {
    let address = mock_address();
    println!("Test address: {}", address.to_bech32(NetworkId::Testnet));
}
