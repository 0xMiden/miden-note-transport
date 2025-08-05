use crate::{
    Result,
    database::Database,
    types::{EncryptedDetails, NoteId},
};
use chrono::Utc;
use miden_objects::utils::{Deserializable, Serializable};
use miden_transport_proto::miden_transport::miden_transport_server::MidenTransportServer;
use miden_transport_proto::miden_transport::{
    EncryptedDetails as ProtoEncryptedDetails, FetchNotesRequest, FetchNotesResponse,
    HealthResponse, MarkReceivedRequest, MarkReceivedResponse, NoteInfo as ProtoNoteInfo,
    NoteStatus as ProtoNoteStatus, SendNoteRequest, SendNoteResponse, StatsResponse,
};
use std::{net::SocketAddr, sync::Arc};
use tonic::{Request, Response, Status};

pub struct GrpcServer {
    database: Arc<Database>,
    config: GrpcServerConfig,
}

#[derive(Clone, Debug)]
pub struct GrpcServerConfig {
    pub host: String,
    pub port: u16,
    pub max_note_size: usize,
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_note_size: 1024 * 1024,
        }
    }
}

impl GrpcServer {
    pub fn new(database: Arc<Database>, config: GrpcServerConfig) -> Self {
        Self { database, config }
    }

    pub fn into_service(self) -> MidenTransportServer<Self> {
        MidenTransportServer::new(self)
    }

    pub async fn serve(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| crate::Error::Internal(format!("Invalid address: {e}")))?;

        tonic::transport::Server::builder()
            .add_service(self.into_service())
            .serve(addr)
            .await
            .map_err(|e| crate::Error::Internal(format!("Server error: {e}")))
    }
}

#[tonic::async_trait]
impl miden_transport_proto::miden_transport::miden_transport_server::MidenTransport for GrpcServer {
    async fn send_note(
        &self,
        request: Request<SendNoteRequest>,
    ) -> std::result::Result<Response<SendNoteResponse>, Status> {
        let request = request.into_inner();

        // Validate note size
        if request
            .encrypted_details
            .as_ref()
            .map(|d| d.data.len())
            .unwrap_or(0)
            > self.config.max_note_size
        {
            return Err(Status::resource_exhausted("Note too large"));
        }

        // Convert protobuf request to internal types
        let header = miden_objects::note::NoteHeader::read_from_bytes(&request.header)
            .map_err(|e| Status::invalid_argument(format!("Invalid header: {e:?}")))?;

        let encrypted_details = request
            .encrypted_details
            .ok_or_else(|| Status::invalid_argument("Missing encrypted_details"))?;

        let encrypted_details = EncryptedDetails(encrypted_details.data);

        // Create note for database
        let note = crate::types::StoredNote {
            header,
            encrypted_data: encrypted_details,
            created_at: Utc::now(),
            received_by: None,
        };

        // Store the note
        self.database
            .store_note(&note)
            .await
            .map_err(|e| Status::internal(format!("Failed to store note: {e:?}")))?;

        Ok(Response::new(SendNoteResponse {
            id: note.header.id().to_hex(),
            status: ProtoNoteStatus::Sent as i32,
        }))
    }

    async fn fetch_notes(
        &self,
        request: Request<FetchNotesRequest>,
    ) -> std::result::Result<Response<FetchNotesResponse>, Status> {
        let request = request.into_inner();

        // Parse tag from hex string
        let tag = u32::from_str_radix(&request.tag, 16)
            .map_err(|e| Status::invalid_argument(format!("Invalid tag format: {e:?}")))?
            .into();

        // Fetch notes from database
        let notes = self
            .database
            .fetch_notes(tag, request.user_id.map(|id| id.into()))
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch notes: {e:?}")))?;

        // Convert to protobuf format
        let proto_notes = notes
            .into_iter()
            .map(|note| ProtoNoteInfo {
                header: note.header.to_bytes(),
                encrypted_data: Some(ProtoEncryptedDetails {
                    data: note.encrypted_data.0,
                }),
                created_at: Some(prost_types::Timestamp {
                    seconds: note.created_at.timestamp(),
                    nanos: note.created_at.timestamp_subsec_nanos() as i32,
                }),
            })
            .collect();

        Ok(Response::new(FetchNotesResponse { notes: proto_notes }))
    }

    async fn mark_received(
        &self,
        request: Request<MarkReceivedRequest>,
    ) -> std::result::Result<Response<MarkReceivedResponse>, Status> {
        let request = request.into_inner();

        // Parse note ID from hex string
        let note_ids: Vec<NoteId> = request
            .note_ids
            .iter()
            .map(|hex| {
                NoteId::try_from_hex(hex)
                    .map_err(|e| Status::invalid_argument(format!("Invalid note ID: {e:?}")))
            })
            .collect::<std::result::Result<Vec<_>, Status>>()?;
        let len = note_ids.len();

        let user_id = request
            .user_id
            .ok_or(Status::invalid_argument("User ID is absent".to_string()))?;

        // Mark note as received
        self.database
            .mark_received(note_ids, user_id.into())
            .await
            .map_err(|e| Status::internal(format!("Failed to mark note received: {e:?}")))?;

        // TODO more detailed response
        Ok(Response::new(MarkReceivedResponse {
            status: vec![ProtoNoteStatus::Marked as i32; len],
        }))
    }

    async fn health(
        &self,
        _request: Request<()>,
    ) -> std::result::Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            status: "healthy".to_string(),
            timestamp: Some(prost_types::Timestamp {
                seconds: Utc::now().timestamp(),
                nanos: Utc::now().timestamp_subsec_nanos() as i32,
            }),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn stats(
        &self,
        _request: Request<()>,
    ) -> std::result::Result<Response<StatsResponse>, Status> {
        // Get statistics from database
        let (total_notes, total_tags) = self
            .database
            .get_stats()
            .await
            .map_err(|e| Status::internal(format!("Failed to get stats: {e:?}")))?;

        Ok(Response::new(StatsResponse {
            total_notes,
            total_tags,
            notes_per_tag: Vec::new(), // TODO: Implement notes_per_tag
        }))
    }
}
