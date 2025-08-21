use std::time::Duration;

use miden_objects::account::AccountId;
use miden_private_transport::{
    Node, NodeConfig,
    client::{
        EncryptionStore, FilesystemEncryptionStore, TransportLayerClient, crypto::SerializableKey,
        grpc::GrpcClient,
    },
    node::grpc::GrpcServerConfig,
    types::{
        NoteStatus, mock_account_id, mock_note_p2id_with_accounts,
        mock_note_p2id_with_tag_and_accounts,
    },
};
use rand::Rng;
use tokio::{task::JoinHandle, time::sleep};

const TAG_LOCALANY: u32 = 0xc000_0000;

enum EncryptionScheme {
    Aes,
    X25519,
}

async fn spawn_server(port: u16) -> JoinHandle<()> {
    let config = NodeConfig {
        grpc: GrpcServerConfig { port, ..Default::default() },
        ..Default::default()
    };

    let server = Node::init(config).await.unwrap();
    tokio::spawn(server.entrypoint())
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
    let handle = spawn_server(port).await;

    sleep(Duration::from_millis(100)).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::Aes).await;
    let (mut client1, accid1, pubkey1) = test_client(port, EncryptionScheme::Aes).await;
    client0.add_key(&pubkey1, &accid1).unwrap();

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
    let handle = spawn_server(port).await;

    sleep(Duration::from_millis(100)).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::X25519).await;
    let (mut client1, accid1, pubkey1) = test_client(port, EncryptionScheme::X25519).await;
    client0.add_key(&pubkey1, &accid1).unwrap();

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

#[tokio::test]
async fn test_transport_different_tags() -> Result<(), Box<dyn std::error::Error>> {
    let port = 9629;
    let handle = spawn_server(port).await;

    sleep(Duration::from_millis(100)).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::X25519).await;
    let (mut client1, accid1, _) = test_client(port, EncryptionScheme::X25519).await;
    let (mut client2, accid2, pubkey2) = test_client(port, EncryptionScheme::X25519).await;

    let sent_tag0 = TAG_LOCALANY.into();
    let sent_tag1 = (TAG_LOCALANY + 1).into();

    client2.register_tag(sent_tag0, None);
    client2.register_tag(sent_tag1, None);

    let note0 = mock_note_p2id_with_tag_and_accounts(sent_tag0, accid0, accid2);
    let note1 = mock_note_p2id_with_tag_and_accounts(sent_tag1, accid1, accid2);

    client0.add_key(&pubkey2, &accid2).unwrap();
    client1.add_key(&pubkey2, &accid2).unwrap();

    let header0 = *note0.header();
    let header1 = *note1.header();

    // Send Note0
    let send_response = client0.send_note(note0, &accid2).await?;
    let (id, status) = send_response;
    assert_eq!(id, header0.id());
    assert_eq!(status, NoteStatus::Sent);

    // Send Note1
    let send_response = client1.send_note(note1, &accid2).await?;
    let (id, status) = send_response;
    assert_eq!(id, header1.id());
    assert_eq!(status, NoteStatus::Sent);

    // Fetch Tag0 (Note0)
    let fetch_response = client2.fetch_notes(sent_tag0).await?;
    let infos = fetch_response;
    assert_eq!(infos.len(), 1);
    let (header, _details) = &infos[0];
    let tag = header.metadata().tag();
    assert_eq!(tag, sent_tag0);

    // Fetch Tag1 (Note1)
    let fetch_response = client2.fetch_notes(sent_tag1).await?;
    let infos = fetch_response;
    assert_eq!(infos.len(), 1);
    let (header, _details) = &infos[0];
    let tag = header.metadata().tag();
    assert_eq!(tag, sent_tag1);

    handle.abort();

    Ok(())
}
