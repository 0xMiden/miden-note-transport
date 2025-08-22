mod common;

use miden_objects::note::NoteTag;
use miden_private_transport::types::{
    NoteStatus, mock_note_p2id_with_accounts, mock_note_p2id_with_tag_and_accounts,
};

use self::common::*;

#[tokio::test]
async fn test_transport_client_note_fetch_tracking()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let port = 9728;
    let handle = spawn_test_server(port).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::X25519).await;
    let (mut client1, accid1, pubkey1) = test_client(port, EncryptionScheme::X25519).await;

    let tag = TAG_LOCALANY.into();

    client0.add_key(&pubkey1, &accid1).await.unwrap();
    client1.register_tag(tag, None).await.unwrap();

    // Verify the key is stored in the database
    let retrieved_key = client0.get_key(&accid1).await.unwrap();
    assert!(retrieved_key.is_some());

    // Create and send a note
    let note = mock_note_p2id_with_tag_and_accounts(tag, accid0, accid1);
    let (note_id, status) = client0.send_note(note, &accid1).await.unwrap();
    assert!(matches!(status, NoteStatus::Sent));

    // Test note fetching recording

    // Initially, the note should not be marked as fetched
    assert!(!client1.note_fetched(&note_id).await.unwrap());

    let _notes = client1.fetch_notes(tag).await?;
    assert!(client1.note_fetched(&note_id).await.unwrap());

    handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_transport_client_key_management()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let port = 9729;
    let handle = spawn_test_server(port).await;

    let (mut client0, _accid0, _) = test_client(port, EncryptionScheme::X25519).await;
    let (_client1, accid1, pubkey1) = test_client(port, EncryptionScheme::X25519).await;
    let (_client2, accid2, pubkey2) = test_client(port, EncryptionScheme::X25519).await;

    // Test storing multiple keys
    client0.add_key(&pubkey1, &accid1).await.unwrap();
    client0.add_key(&pubkey2, &accid2).await.unwrap();

    // Verify both keys are stored
    let retrieved_key_1 = client0.get_key(&accid1).await.unwrap();
    let retrieved_key_2 = client0.get_key(&accid2).await.unwrap();

    assert!(retrieved_key_1.is_some());
    assert!(retrieved_key_2.is_some());

    // Test getting all public keys
    let all_keys = client0.get_all_keys().await.unwrap();
    assert_eq!(all_keys.len(), 2);

    let account_ids: Vec<miden_objects::account::AccountId> =
        all_keys.iter().map(|(id, _)| *id).collect();
    assert!(account_ids.contains(&accid1));
    assert!(account_ids.contains(&accid2));

    handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_transport_client_note_storage() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let port = 9730;
    let handle = spawn_test_server(port).await;

    let (mut client0, accid0, _) = test_client(port, EncryptionScheme::X25519).await;
    let (mut client1, accid1, pubkey1) = test_client(port, EncryptionScheme::X25519).await;

    // Add the public key
    client0.add_key(&pubkey1, &accid1).await.unwrap();

    // Send a note
    let note = mock_note_p2id_with_accounts(accid0, accid1);
    let (note_id, note_status) = client0.send_note(note, &accid1).await.unwrap();
    assert!(matches!(note_status, NoteStatus::Sent));

    // Fetch
    let sent_tag = NoteTag::from_account_id(accid1);
    let fetched_notes = client1.fetch_notes(sent_tag).await.unwrap();
    assert_eq!(fetched_notes.len(), 1);

    // Verify marked as fetched in the DB
    let fetched_note_ids = client1.get_fetched_notes_for_tag(sent_tag).await.unwrap();
    assert_eq!(fetched_note_ids.len(), 1);
    assert_eq!(fetched_note_ids[0], note_id);

    // Verify database statistics
    let stats = client1.get_database_stats().await.unwrap();
    assert_eq!(stats.fetched_notes_count, 1);
    assert_eq!(stats.encrypted_notes_count, 1);
    assert_eq!(stats.unique_tags_count, 1);

    handle.abort();
    Ok(())
}
