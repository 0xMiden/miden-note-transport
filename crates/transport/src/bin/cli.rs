use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use clap::{Parser, Subcommand};
use miden_objects::{note::Note, utils::Deserializable};
use miden_transport::{
    client::{crypto, grpc::GrpcClient, Client, FilesystemEncryptionStore},
    Result,
};
use tracing::info;

#[derive(Parser)]
#[command(name = "miden-transport-cli")]
#[command(about = "Miden Transport CLI - Test client for the transport layer")]
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
        /// Note data (base64 encoded)
        #[arg(long)]
        data: String,

        /// Recipient's public key (base64 encoded)
        #[arg(long)]
        recipient_key: String,
    },

    /// Fetch notes for a tag
    Fetch {
        /// Note tag
        #[arg(long)]
        tag: u32,

        /// Recipient's private key (base64 encoded)
        #[arg(long)]
        private_key: String,
    },

    /// Generate a new encryption key
    GenerateKey,

    /// Generate a new note tag
    GenerateTag,

    /// Check node health
    Health,

    /// Get node statistics
    Stats,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    info!("Miden Transport CLI");
    info!("Endpoint: {}", args.endpoint);

    // Create client
    let grpc = GrpcClient::connect(args.endpoint, args.timeout).await?;
    let encryption_store = FilesystemEncryptionStore::new("./keys")?;
    let mut client = Client::new(Box::new(grpc), Box::new(encryption_store));

    match args.command {
        Commands::Send {
            data,
            recipient_key,
        } => {
            send_note(&mut client, &data, &recipient_key).await?;
        }
        Commands::Fetch { tag, private_key } => {
            fetch_notes(&mut client, tag, &private_key).await?;
        }
        Commands::GenerateKey => {
            generate_key();
        }
        Commands::GenerateTag => {
            generate_tag();
        }
        Commands::Health => {
            health_check(&client).await?;
        }
        Commands::Stats => {
            get_stats(&client).await?;
        }
    }

    Ok(())
}

async fn send_note(client: &mut Client, data: &str, recipient_key: &str) -> Result<()> {
    let bytes = BASE64.decode(data).map_err(|e| {
        miden_transport::Error::InvalidNoteData(format!("Invalid base64 data: {e}"))
    })?;

    let note = Note::read_from_bytes(&bytes).unwrap();

    // Decode base64 recipient key
    let pub_key = BASE64
        .decode(recipient_key)
        .map_err(|e| miden_transport::Error::InvalidNoteData(format!("Invalid base64 key: {e}")))?;

    // Validate key
    if !crypto::is_valid_encryption_key(&pub_key) {
        return Err(miden_transport::Error::InvalidNoteData(
            "Invalid encryption key format".to_string(),
        ));
    }

    info!("Sending note to tag {}", note.header().metadata().tag());

    // Send the note
    client.send_note(note, &pub_key).await?;
    info!("Note sent successfully");

    Ok(())
}

async fn fetch_notes(client: &mut Client, tag: u32, private_key: &str) -> Result<()> {
    info!("Fetching notes for tag {}", tag);

    // Decode base64 private key
    let key = BASE64
        .decode(private_key)
        .map_err(|e| miden_transport::Error::InvalidNoteData(format!("Invalid base64 key: {e}")))?;

    // Validate key
    if !crypto::is_valid_encryption_key(&key) {
        return Err(miden_transport::Error::InvalidNoteData(
            "Invalid encryption key format".to_string(),
        ));
    }

    // Fetch notes
    let decrypted_notes = client.fetch_notes(tag.into(), &key).await?;

    info!("Found {} notes", decrypted_notes.len());

    for (i, (header, details)) in decrypted_notes.iter().enumerate() {
        println!(
            "Note {}:\n Header: {:?}\n Details: {:?}",
            i + 1,
            header,
            details
        );
    }

    Ok(())
}

fn generate_key() {
    let key = crypto::generate_key();
    let base64_key = BASE64.encode(&key);
    println!("Generated encryption key: {base64_key}");
}

fn generate_tag() {
    let tag = crypto::generate_note_tag();
    println!("Generated note tag: {tag}");
}

async fn health_check(_client: &Client) -> Result<()> {
    info!("Checking node health");

    // For now, we'll need to access the API client directly
    // This is a limitation of the current Client design
    println!("❌ Health check not implemented in Client");
    println!("Use ApiClient directly for health checks");

    Ok(())
}

async fn get_stats(_client: &Client) -> Result<()> {
    info!("Getting node statistics");

    // For now, we'll need to access the API client directly
    // This is a limitation of the current Client design
    println!("❌ Stats not implemented in Client");
    println!("Use ApiClient directly for statistics");

    Ok(())
}
