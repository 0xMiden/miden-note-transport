use std::path::PathBuf;

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use miden_objects::{account::AccountId, note::Note, utils::Deserializable};
use miden_private_transport_client::{
    Error, FilesystemEncryptionStore, Result, TransportLayerClient,
    crypto::{SerializableKey, aes::Aes256GcmKey},
    database::ClientDatabaseConfig,
    grpc::GrpcClient,
    logging::{OpenTelemetry, setup_tracing},
    types::{NoteTag, mock_account_id, mock_note_p2id_with_accounts},
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

    /// Keys directory
    #[arg(long, default_value = "./keys")]
    keys_dir: PathBuf,

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

        /// Recipient's account ID
        #[arg(long)]
        account_id: String,
    },

    /// Fetch notes for a tag
    Fetch {
        /// Note tag
        #[arg(long)]
        tag: u32,
    },

    /// Initialize the client with an account
    Init {
        /// Listening account ID
        #[arg(long)]
        account_id: String,
        /// Decryption key (hex encoded)
        #[arg(long)]
        key: String,
    },

    /// Generate a new encryption key
    GenerateKey {
        /// Key type: aes or x25519
        key_type: String,
    },

    /// Add a key associated to an account ID
    AddKey {
        /// Key
        #[arg(long)]
        key: String,
        /// Account ID
        #[arg(long)]
        account_id: String,
    },

    /// Clean up old data
    Cleanup {
        /// Retention period in days
        days: u32,
    },

    /// List all stored keys
    ListKeys,

    /// Register a tag for listening
    RegisterTag {
        /// Tag to register
        #[arg(long)]
        tag: u32,
        /// Account ID
        #[arg(long)]
        account_id: Option<String>,
    },

    /// Random note for testing purposes
    TestNote {
        /// Recipient account ID
        #[arg(long)]
        recipient: String,
    },

    /// Random account ID for testing purposes
    TestAccountId,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing(OpenTelemetry::Enabled)?;

    // Parse command line arguments
    let args = Args::parse();

    info!("Miden Transport CLI");
    info!("Endpoint: {}", args.endpoint);
    info!("Database: {:?}", args.database);
    info!("Keys directory: {:?}", args.keys_dir);
    info!("Database path: {:?}", args.database);

    let db_config = ClientDatabaseConfig {
        url: args.database.to_string_lossy().to_string(),
        max_note_size: 1024 * 1024, // 1MB
    };

    // Create client
    let grpc = GrpcClient::connect(args.endpoint, args.timeout).await?;
    let encryption_store = FilesystemEncryptionStore::new(args.keys_dir)?;
    let mut client = TransportLayerClient::init(
        Box::new(grpc),
        Box::new(encryption_store),
        vec![],
        Some(db_config),
    )
    .await?;

    match args.command {
        Commands::Send { note, account_id } => {
            send_note(&mut client, &note, &account_id).await?;
        },
        Commands::Fetch { tag } => {
            fetch_notes(&mut client, tag).await?;
        },
        Commands::Init { account_id, key } => {
            init(&mut client, account_id, key).await?;
        },
        Commands::GenerateKey { key_type } => {
            generate_key(&key_type);
        },
        Commands::AddKey { key, account_id } => {
            add_key(&mut client, &key, &account_id).await?;
        },
        Commands::Cleanup { days } => {
            cleanup_old_data(&client, days).await?;
        },
        Commands::ListKeys => {
            list_keys(&client).await?;
        },
        Commands::RegisterTag { tag, account_id } => {
            let account_id = account_id
                .map(|id| {
                    AccountId::from_hex(&id)
                        .map_err(|e| Error::Generic(anyhow!("Invalid recipient Account ID: {e}")))
                })
                .transpose()?;
            client.register_tag(tag.into(), account_id).await?;
            println!("✅ Tag {tag} registered successfully");
        },
        Commands::TestNote { recipient } => {
            mock_note(&recipient)?;
        },
        Commands::TestAccountId => {
            test_account_id();
        },
    }

    Ok(())
}

async fn send_note(
    client: &mut TransportLayerClient,
    data: &str,
    recipient_account_id: &str,
) -> Result<()> {
    let bytes =
        hex::decode(data).map_err(|e| Error::InvalidNoteData(format!("Invalid hex data: {e}")))?;

    let note = Note::read_from_bytes(&bytes)
        .map_err(|e| Error::InvalidNoteData(format!("Failed to deserialize Note: {e}")))?;

    info!("Sending note to tag {}", note.header().metadata().tag());

    let account_id = AccountId::from_hex(recipient_account_id)
        .map_err(|e| Error::Generic(anyhow!("Invalid recipient Account ID: {e}")))?;
    // Send the note
    client.send_note(note, &account_id).await?;
    info!("Note sent successfully");

    Ok(())
}

async fn fetch_notes(client: &mut TransportLayerClient, tag: u32) -> Result<()> {
    info!("Fetching notes for tag {}", tag);

    // Fetch notes
    let decrypted_notes = client.fetch_notes(tag.into()).await?;

    info!("Found {} notes", decrypted_notes.len());

    for (i, (header, details)) in decrypted_notes.iter().enumerate() {
        println!("Note {}:\n Header: {:?}\n Details: {:?}", i + 1, header, details);
    }

    Ok(())
}

async fn init(client: &mut TransportLayerClient, account_id: String, key: String) -> Result<()> {
    let account_id = AccountId::from_hex(&account_id)
        .map_err(|e| Error::Generic(anyhow!("Invalid recipient Account ID: {e}")))?;

    // Parse the key from hex
    let key_bytes =
        hex::decode(&key).map_err(|e| Error::Generic(anyhow!("Invalid hex key: {e}")))?;

    // Try to deserialize as SerializableKey
    let serializable_key = match serde_json::from_slice::<SerializableKey>(&key_bytes) {
        Ok(key) => key,
        Err(_) => {
            // If JSON deserialization fails, try to create an AES key from the bytes
            if key_bytes.len() == 32 {
                let aes_key = Aes256GcmKey::new(key_bytes.try_into().unwrap());
                SerializableKey::Aes256Gcm(aes_key)
            } else {
                return Err(Error::Generic(anyhow!("Invalid key format or length")));
            }
        },
    };

    client.add_account_id(&account_id);
    client.add_key(&serializable_key, &account_id).await?;
    // By default, register NoteTag derived from this Account Id
    client.register_tag(NoteTag::from_account_id(account_id), None).await?;

    info!("Successfully initialized client with account {} and key", account_id);

    Ok(())
}

fn generate_key(key_type: &str) {
    let key = match key_type {
        "aes" => SerializableKey::generate_aes(),
        "x25519" => SerializableKey::generate_x25519(),
        _ => {
            println!("❌ Invalid key type '{key_type}'. Use 'aes' or 'x25519'");
            return;
        },
    };

    let hex_key = hex::encode(serde_json::to_vec(&key).unwrap());
    println!("Generated {key_type} encryption key: {hex_key}");

    if let Some(public_key) = key.public_key() {
        let pub_hex = hex::encode(serde_json::to_vec(&public_key).unwrap());
        println!("Public key: {pub_hex}");
    }
}

async fn add_key(client: &mut TransportLayerClient, key: &str, account_id: &str) -> Result<()> {
    let key_bytes =
        hex::decode(key).map_err(|e| Error::Generic(anyhow!("Invalid hex key: {e}")))?;

    // Try to deserialize as SerializableKey
    let serializable_key = match serde_json::from_slice::<SerializableKey>(&key_bytes) {
        Ok(key) => key,
        Err(_) => {
            // If JSON deserialization fails, try to create an AES key from the bytes
            if key_bytes.len() == 32 {
                let aes_key = Aes256GcmKey::new(key_bytes.try_into().unwrap());
                SerializableKey::Aes256Gcm(aes_key)
            } else {
                return Err(Error::Generic(anyhow!("Invalid key format or length")));
            }
        },
    };

    let account_id = AccountId::from_hex(account_id)
        .map_err(|e| Error::Generic(anyhow!("Invalid recipient Account ID: {e}")))?;

    client.add_account_id(&account_id);
    client.add_key(&serializable_key, &account_id).await?;

    info!("Successfully added key for account {}", account_id);

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

async fn list_keys(client: &TransportLayerClient) -> Result<()> {
    info!("Listing all stored keys");

    match client.get_all_keys().await {
        Ok(keys) => {
            if keys.is_empty() {
                println!("No keys stored");
            } else {
                println!("Stored keys:");
                for (account_id, key) in keys {
                    println!(
                        "  Account: {} -> Key type: {:?}",
                        account_id,
                        std::mem::discriminant(&key)
                    );
                }
            }
        },
        Err(e) => {
            println!("❌ Failed to list keys: {e}");
        },
    }

    Ok(())
}

fn mock_note(recipient: &str) -> Result<()> {
    use miden_objects::utils::Serializable;
    let account_id = AccountId::from_hex(recipient)
        .map_err(|e| Error::Generic(anyhow!("Invalid recipient Account ID: {e}")))?;
    let note = mock_note_p2id_with_accounts(mock_account_id(), account_id);
    let hex_note = hex::encode(note.to_bytes());
    info!("Test note: {}", hex_note);
    Ok(())
}

fn test_account_id() {
    let account_id = mock_account_id();
    println!("Test account ID: {account_id}");
}
