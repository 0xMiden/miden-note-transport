mod common;

use futures::StreamExt;
use miden_private_transport_client::types::{mock_address, mock_note_p2id_with_addresses};
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
