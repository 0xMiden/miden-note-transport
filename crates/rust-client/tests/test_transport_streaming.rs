mod common;

use futures::StreamExt;
use miden_private_transport_client::test_utils::{mock_address, mock_note_p2id_with_addresses};
use tokio::time::{Duration, sleep};

use self::common::*;

#[tokio::test]
async fn test_transport_stream() -> Result<(), Box<dyn std::error::Error>> {
    let port = 9631;
    let handle = spawn_test_server(port).await;

    let (mut client0, adr0) = test_client(port).await;
    let (mut client1, adr1) = test_client(port).await;
    let adr2 = mock_address();

    let tag = adr1.to_note_tag();

    let note0 = mock_note_p2id_with_addresses(&adr0, &adr1);
    let note1 = mock_note_p2id_with_addresses(&adr0, &adr1);
    let note2 = mock_note_p2id_with_addresses(&adr0, &adr2);

    let mut stream = client1.stream_notes(tag).await?;

    let count = tokio::spawn(async move {
        let mut count = 0;
        let timeout = sleep(Duration::from_secs(5));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                res = stream.next() => {
                    if let Some(Ok(notes)) = res {
                        count += notes.len();
                    }
                }
                _ = &mut timeout => {
                    break;
                }
            }
        }
        count
    });

    client0.send_note(note0, &adr1).await.unwrap();
    sleep(Duration::from_secs(1)).await;

    client0.send_note(note1, &adr1).await.unwrap();
    sleep(Duration::from_secs(1)).await;

    // Ignored note
    client0.send_note(note2, &adr2).await.unwrap();

    assert_eq!(count.await.unwrap(), 2);

    handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_transport_stream_multiple_receivers() -> Result<(), Box<dyn std::error::Error>> {
    let port = 9632;
    let handle = spawn_test_server(port).await;

    let (mut client0, adr0) = test_client(port).await;
    let (mut client1, adr1) = test_client(port).await;
    let (mut client2, adr2) = test_client(port).await;

    let note0 = mock_note_p2id_with_addresses(&adr0, &adr1);
    let note1 = mock_note_p2id_with_addresses(&adr0, &adr1);
    let note2 = mock_note_p2id_with_addresses(&adr0, &adr2);

    let mut stream1 = client1.stream_notes(adr1.to_note_tag()).await?;
    let mut stream2 = client2.stream_notes(adr2.to_note_tag()).await?;

    let counts_handle = tokio::spawn(async move {
        let mut count1 = 0;
        let mut count2 = 0;
        let timeout = sleep(Duration::from_secs(5));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                res = stream1.next() => {
                    if let Some(Ok(notes)) = res {
                        count1 += notes.len();
                    }
                }
                res = stream2.next() => {
                    if let Some(Ok(notes)) = res {
                        count2 += notes.len();
                    }
                }
                _ = &mut timeout => {
                    break;
                }
            }
        }
        (count1, count2)
    });

    client0.send_note(note0, &adr1).await.unwrap();
    client0.send_note(note1, &adr1).await.unwrap();
    client0.send_note(note2, &adr2).await.unwrap();

    let (count1, count2) = counts_handle.await.unwrap();
    assert_eq!(count1, 2);
    assert_eq!(count2, 1);

    handle.abort();
    Ok(())
}
