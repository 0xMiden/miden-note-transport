mod common;

use miden_private_transport_client::types::{
    NoteStatus, mock_note_p2id_with_addresses, mock_note_p2id_with_tag_and_addresses,
};

use self::common::*;

pub const TAG_LOCALANY: u32 = 0xc000_0000;

#[tokio::test]
async fn test_transport_note() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let port = 9627;
    let handle = spawn_test_server(port).await;

    let (mut client0, adr0) = test_client(port).await;
    let (mut client1, adr1) = test_client(port).await;

    let sent_tag = adr1.to_note_tag();

    let note = mock_note_p2id_with_addresses(&adr0, &adr1);
    let header = *note.header();

    let send_response = client0.send_note(note, Some(&adr1)).await?;
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
async fn test_transport_different_tags() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let port = 9629;
    let handle = spawn_test_server(port).await;

    let (mut client0, adr0) = test_client(port).await;
    let (mut client1, adr1) = test_client(port).await;
    let (mut client2, adr2) = test_client(port).await;

    let sent_tag0 = TAG_LOCALANY.into();
    let sent_tag1 = (TAG_LOCALANY + 1).into();

    let note0 = mock_note_p2id_with_tag_and_addresses(sent_tag0, &adr0, &adr2);
    let note1 = mock_note_p2id_with_tag_and_addresses(sent_tag1, &adr1, &adr2);

    let header0 = *note0.header();
    let header1 = *note1.header();

    // Send Note0
    let send_response = client0.send_note(note0, None).await?;
    let (id, status) = send_response;
    assert_eq!(id, header0.id());
    assert_eq!(status, NoteStatus::Sent);

    // Send Note1
    let send_response = client1.send_note(note1, None).await?;
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
