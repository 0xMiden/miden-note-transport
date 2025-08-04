use miden_transport::{
    client::{grpc::GrpcClient, Client, FilesystemEncryptionStore},
    node::grpc::GrpcServerConfig,
    types::{mock_note_p2id, NoteStatus, UserId},
    Node, NodeConfig,
};
use serial_test::serial;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[serial]
async fn test_transport_basic_note() -> Result<(), Box<dyn std::error::Error>> {
    let port = 9627;
    let timeout_ms = 1000;
    let url = format!("http://127.0.0.1:{port}");

    let config = NodeConfig {
        grpc: GrpcServerConfig {
            port,
            ..Default::default()
        },
        ..Default::default()
    };

    let node = Node::init(config).await.unwrap();
    let handle = tokio::spawn(node.entrypoint());

    sleep(Duration::from_millis(100)).await;

    let grpc_client = Box::new(GrpcClient::connect(url, timeout_ms, Some(UserId::random())).await?);
    let encryption_store = Box::new(FilesystemEncryptionStore::new("/tmp")?);
    let mut client = Client::new(grpc_client, encryption_store);
    // TODO make use of EncryptionStore
    let key = miden_transport::client::crypto::generate_key();

    // Send a note
    let note = mock_note_p2id();
    let header = *note.header();
    let sent_tag = header.metadata().tag();

    let send_response = client.send_note(note, &key).await?;
    let (id, status) = send_response;
    assert_eq!(id, header.id());
    assert_eq!(status, NoteStatus::Sent);

    let note_id = send_response.0;

    // Fetch note back
    let fetch_response = client.fetch_notes(sent_tag, &key).await?;
    let infos = fetch_response;
    assert_eq!(infos.len(), 1);
    let (header, _details) = &infos[0];

    let tag = header.metadata().tag();
    assert_eq!(tag, sent_tag);

    // Mark note as received
    client.mark_received(&[note_id]).await?;
    let notes = client.fetch_notes(sent_tag, &key).await?;
    assert_eq!(notes.len(), 0);

    handle.abort();

    Ok(())
}
