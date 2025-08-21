use std::time::Duration;

use miden_objects::account::AccountId;
use miden_private_transport::{
    Node, NodeConfig,
    client::{
        EncryptionStore, FilesystemEncryptionStore, TransportLayerClient, crypto::SerializableKey,
        grpc::GrpcClient,
    },
    node::grpc::GrpcServerConfig,
    types::{NoteStatus, mock_account_id, mock_note_p2id_with_accounts},
};
use rand::Rng;
use tokio::time::sleep;

enum EncryptionScheme {
    Aes,
    X25519,
}

async fn test_client(
    port: u16,
    scheme: EncryptionScheme,
) -> (TransportLayerClient, AccountId, SerializableKey) {
    let timeout_ms = 1000;
    let url = format!("http://127.0.0.1:{port}");

    let grpc_client = Box::new(GrpcClient::connect(url, timeout_ms).await.unwrap());
    let mut rng = rand::rng();
    let num: u32 = rng.random();
    let encryption_store =
        Box::new(FilesystemEncryptionStore::new(format!("/tmp/mptl-keystore-{num:08x}")).unwrap());

    let key = match scheme {
        EncryptionScheme::Aes => SerializableKey::generate_aes(),
        EncryptionScheme::X25519 => SerializableKey::generate_x25519(),
    };
    let account_id = mock_account_id();

    encryption_store.add_key(&account_id, &key).unwrap();

    let client = TransportLayerClient::new(grpc_client, encryption_store, vec![account_id]);

    (client, account_id, key.public_key().unwrap())
}

#[tokio::test]
async fn test_transport_aes_note() -> Result<(), Box<dyn std::error::Error>> {
    let port = 9627;
    let config = NodeConfig {
        grpc: GrpcServerConfig { port, ..Default::default() },
        ..Default::default()
    };

    let server = Node::init(config).await.unwrap();
    let handle = tokio::spawn(server.entrypoint());

    sleep(Duration::from_millis(100)).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::Aes).await;
    let (mut client1, accid1, pubkey1) = test_client(port, EncryptionScheme::Aes).await;
    client0.add_key(&accid1, &pubkey1).unwrap();

    let sent_tag = miden_objects::note::NoteTag::from_account_id(accid1);

    let note = mock_note_p2id_with_accounts(accid0, accid1);
    let header = *note.header();

    let send_response = client0.send_note(note, &accid1).await?;
    let (id, status) = send_response;
    assert_eq!(id, header.id());
    assert_eq!(status, NoteStatus::Sent);

    // Fetch note back
    let fetch_response = client1.fetch_notes(sent_tag).await?;
    let infos = fetch_response;
    assert_eq!(infos.len(), 1);
    let (header, _details) = &infos[0];

    let tag = header.metadata().tag();
    assert_eq!(tag, sent_tag);

    handle.abort();

    Ok(())
}

#[tokio::test]
async fn test_transport_x25519_note() -> Result<(), Box<dyn std::error::Error>> {
    let port = 9628;
    let config = NodeConfig {
        grpc: GrpcServerConfig { port, ..Default::default() },
        ..Default::default()
    };

    let server = Node::init(config).await.unwrap();
    let handle = tokio::spawn(server.entrypoint());

    sleep(Duration::from_millis(100)).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::X25519).await;
    let (mut client1, accid1, pubkey1) = test_client(port, EncryptionScheme::X25519).await;
    client0.add_key(&accid1, &pubkey1).unwrap();

    let sent_tag = miden_objects::note::NoteTag::from_account_id(accid1);

    let note = mock_note_p2id_with_accounts(accid0, accid1);
    let header = *note.header();

    let send_response = client0.send_note(note, &accid1).await?;
    let (id, status) = send_response;
    assert_eq!(id, header.id());
    assert_eq!(status, NoteStatus::Sent);

    let fetch_response = client1.fetch_notes(sent_tag).await?;
    let infos = fetch_response;
    assert_eq!(infos.len(), 1);
    let (header, _details) = &infos[0];

    let tag = header.metadata().tag();
    assert_eq!(tag, sent_tag);

    handle.abort();

    Ok(())
}
