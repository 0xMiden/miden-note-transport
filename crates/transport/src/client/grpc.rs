use std::time::Duration;

use chrono::{DateTime, Utc};
use miden_objects::utils::{Deserializable, Serializable};
use miden_private_transport_proto::miden_transport::{
    EncryptedNote, FetchNotesRequest, SendNoteRequest, miden_transport_client::MidenTransportClient,
};
use prost_types;
use tonic::{
    Request,
    transport::{Channel, ClientTlsConfig},
};
use tower::timeout::Timeout;

use crate::{
    Error, Result,
    types::{NoteHeader, NoteId, NoteInfo, NoteTag},
};

pub struct GrpcClient {
    client: MidenTransportClient<Timeout<Channel>>,
    // Last fetched timestamp
    lts: DateTime<Utc>,
}

impl GrpcClient {
    pub async fn connect(endpoint: String, timeout_ms: u64) -> Result<Self> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let channel = Channel::from_shared(endpoint.clone())
            .map_err(|e| Error::Internal(format!("Invalid endpoint URI: {e}")))?
            .tls_config(tls)?
            .connect()
            .await?;
        let timeout = Duration::from_millis(timeout_ms);
        let timeout_channel = Timeout::new(channel, timeout);
        let client = MidenTransportClient::new(timeout_channel);
        let lts = DateTime::from_timestamp(0, 0).unwrap();

        Ok(Self { client, lts })
    }

    pub async fn send_note(
        &mut self,
        header: NoteHeader,
        encrypted_details: Vec<u8>,
    ) -> Result<NoteId> {
        let request = SendNoteRequest {
            note: Some(EncryptedNote {
                header: header.to_bytes(),
                encrypted_details,
            }),
        };

        let response = self
            .client
            .clone()
            .send_note(Request::new(request))
            .await
            .map_err(|e| Error::Internal(format!("Send note failed: {e:?}")))?;

        let response = response.into_inner();

        // Parse note ID from hex string
        let note_id = NoteId::try_from_hex(&response.id)
            .map_err(|e| Error::Internal(format!("Invalid note ID: {e:?}")))?;

        Ok(note_id)
    }

    pub async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<NoteInfo>> {
        let request =
            FetchNotesRequest {
                tag: tag.as_u32(),
                timestamp: Some(prost_types::Timestamp {
                    seconds: self.lts.timestamp(),
                    nanos: self.lts.timestamp_subsec_nanos().try_into().map_err(|_| {
                        Error::Internal("Timestamp nanoseconds too large".to_string())
                    })?,
                }),
            };

        let response = self
            .client
            .clone()
            .fetch_notes(Request::new(request))
            .await
            .map_err(|e| Error::Internal(format!("Fetch notes failed: {e:?}")))?;

        let response = response.into_inner();

        // Convert protobuf notes to internal format and track the most recent received timestamp
        let mut notes = Vec::new();
        let mut latest_received_at = self.lts;

        for note in response.notes {
            let header = NoteHeader::read_from_bytes(&note.header)
                .map_err(|e| Error::Internal(format!("Invalid note header: {e:?}")))?;

            // Convert protobuf timestamp to DateTime
            let received_at = if let Some(timestamp) = note.timestamp {
                chrono::DateTime::from_timestamp(
                    timestamp.seconds,
                    timestamp.nanos.try_into().map_err(|_| {
                        Error::Internal("Negative timestamp nanoseconds".to_string())
                    })?,
                )
                .ok_or_else(|| Error::Internal("Invalid timestamp".to_string()))?
            } else {
                Utc::now() // Fallback to current time if timestamp is missing
            };

            // Update the latest received timestamp
            if received_at > latest_received_at {
                latest_received_at = received_at;
            }

            notes.push(NoteInfo {
                header,
                encrypted_data: note.encrypted_details,
                created_at: received_at,
            });
        }

        // Update the last timestamp to the most recent received timestamp
        self.lts = latest_received_at;

        Ok(notes)
    }

    /// Health check
    pub async fn health(&mut self) -> Result<crate::types::HealthResponse> {
        let response = self
            .client
            .health(tonic::Request::new(()))
            .await
            .map_err(|e| Error::Network(format!("Health check failed: {e:?}")))?
            .into_inner();

        let timestamp = response
            .timestamp
            .ok_or_else(|| Error::Internal("Missing timestamp".to_string()))?;

        let timestamp = chrono::DateTime::from_timestamp(
            timestamp.seconds,
            timestamp
                .nanos
                .try_into()
                .map_err(|_| Error::Internal("Negative nanoseconds".to_string()))?,
        )
        .ok_or_else(|| Error::Internal("Invalid timestamp".to_string()))?;

        Ok(crate::types::HealthResponse {
            status: response.status,
            timestamp,
            version: response.version,
        })
    }

    /// Get server statistics
    pub async fn stats(&mut self) -> Result<crate::types::StatsResponse> {
        let response = self
            .client
            .stats(tonic::Request::new(()))
            .await
            .map_err(|e| Error::Network(format!("Stats request failed: {e:?}")))?
            .into_inner();

        let notes_per_tag: Vec<crate::types::TagStats> = response
            .notes_per_tag
            .into_iter()
            .map(|tag_stats| {
                let last_activity = tag_stats
                    .last_activity
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(
                            ts.seconds,
                            ts.nanos
                                .try_into()
                                .map_err(|_| Error::Internal("Negative nanoseconds".to_string()))?,
                        )
                        .ok_or_else(|| Error::Internal("Invalid timestamp".to_string()))
                    })
                    .transpose()?;

                Ok(crate::types::TagStats {
                    tag: tag_stats.tag.into(),
                    note_count: tag_stats.note_count,
                    last_activity,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(crate::types::StatsResponse {
            total_notes: response.total_notes,
            total_tags: response.total_tags,
            notes_per_tag,
        })
    }
}

#[async_trait::async_trait]
impl super::TransportClient for GrpcClient {
    async fn send_note(
        &mut self,
        header: NoteHeader,
        encrypted_note: Vec<u8>,
    ) -> Result<(NoteId, crate::types::NoteStatus)> {
        let note_id = self.send_note(header, encrypted_note).await?;
        Ok((note_id, crate::types::NoteStatus::Sent))
    }

    async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<crate::types::NoteInfo>> {
        self.fetch_notes(tag).await
    }
}
