use miden_objects::utils::{Deserializable, Serializable};
use std::time::Duration;
use tonic::{
    transport::{Channel, ClientTlsConfig},
    Request,
};
use tower::timeout::Timeout;

use crate::{
    types::{EncryptedDetails, NoteHeader, NoteId, NoteInfo, NoteTag},
    Error, Result,
};

use miden_transport_proto::miden_transport::miden_transport_client::MidenTransportClient;
use miden_transport_proto::miden_transport::{
    EncryptedDetails as ProtoEncryptedDetails, FetchNotesRequest, MarkReceivedRequest,
    SendNoteRequest,
};

pub struct GrpcClient {
    client: MidenTransportClient<Timeout<Channel>>,
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

        Ok(Self { client })
    }

    pub async fn send_note(
        &mut self,
        header: NoteHeader,
        encrypted_details: EncryptedDetails,
    ) -> Result<NoteId> {
        let request = SendNoteRequest {
            header: header.to_bytes(),
            encrypted_details: Some(ProtoEncryptedDetails {
                data: encrypted_details.0,
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
        let request = FetchNotesRequest {
            tag: format!("{:08x}", tag.as_u32()),
        };

        let response = self
            .client
            .clone()
            .fetch_notes(Request::new(request))
            .await
            .map_err(|e| Error::Internal(format!("Fetch notes failed: {e:?}")))?;

        let response = response.into_inner();

        // Convert protobuf notes to internal format
        let notes = response
            .notes
            .into_iter()
            .map(|proto_note| {
                let header = NoteHeader::read_from_bytes(&proto_note.header)
                    .map_err(|e| Error::Internal(format!("Invalid note header: {e:?}")))?;

                let encrypted_data = proto_note
                    .encrypted_data
                    .ok_or_else(|| Error::Internal("Missing encrypted data".to_string()))?;

                let created_at = proto_note
                    .created_at
                    .ok_or_else(|| Error::Internal("Missing created_at".to_string()))?;

                let created_at =
                    chrono::DateTime::from_timestamp(created_at.seconds, created_at.nanos as u32)
                        .ok_or_else(|| Error::Internal("Invalid timestamp".to_string()))?;

                Ok(NoteInfo {
                    header,
                    encrypted_data: EncryptedDetails(encrypted_data.data),
                    created_at,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(notes)
    }

    pub async fn mark_received(&mut self, note_id: NoteId, user_id: String) -> Result<()> {
        let request = MarkReceivedRequest {
            id: note_id.to_hex(),
            user_id,
        };

        self.client
            .clone()
            .mark_received(Request::new(request))
            .await
            .map_err(|e| Error::Internal(format!("Mark received failed: {e:?}")))?;

        Ok(())
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

        let timestamp = chrono::DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
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
                let tag = u32::from_str_radix(&tag_stats.tag, 16)
                    .map_err(|e| Error::Internal(format!("Invalid tag format: {e:?}")))?;

                let last_activity = tag_stats
                    .last_activity
                    .map(|ts| {
                        chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                            .ok_or_else(|| Error::Internal("Invalid timestamp".to_string()))
                    })
                    .transpose()?;

                Ok(crate::types::TagStats {
                    tag: tag.into(),
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
        encrypted_note: EncryptedDetails,
    ) -> Result<(NoteId, crate::types::NoteStatus)> {
        let note_id = self.send_note(header, encrypted_note).await?;
        Ok((note_id, crate::types::NoteStatus::Sent))
    }

    async fn fetch_notes(&mut self, tag: NoteTag) -> Result<Vec<crate::types::NoteInfo>> {
        self.fetch_notes(tag).await
    }

    async fn mark_received(&mut self, note_id: NoteId) -> Result<()> {
        self.mark_received(note_id, "default_user".to_string())
            .await
    }
}
