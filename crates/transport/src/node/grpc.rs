use std::{net::SocketAddr, sync::Arc};

use chrono::{DateTime, Utc};
use miden_objects::utils::{Deserializable, Serializable};
use miden_private_transport_proto::miden_transport::{
    EncryptedNoteTimestamped, FetchNotesRequest, FetchNotesResponse, HealthResponse,
    NoteStatus as ProtoNoteStatus, SendNoteRequest, SendNoteResponse, StatsResponse,
    miden_transport_server::MidenTransportServer,
};
use prost_types;
use tonic::{Request, Response, Status};

use crate::{Result, database::Database};

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
impl miden_private_transport_proto::miden_transport::miden_transport_server::MidenTransport
    for GrpcServer
{
    async fn send_note(
        &self,
        request: Request<SendNoteRequest>,
    ) -> std::result::Result<Response<SendNoteResponse>, Status> {
        let request = request.into_inner();

        let note = request.note.ok_or_else(|| Status::invalid_argument("Missing note"))?;

        // Validate note size
        if note.encrypted_details.len() > self.config.max_note_size {
            return Err(Status::resource_exhausted("Note too large"));
        }

        // Convert protobuf request to internal types
        let header = miden_objects::note::NoteHeader::read_from_bytes(&note.header)
            .map_err(|e| Status::invalid_argument(format!("Invalid header: {e:?}")))?;

        // Create note for database
        let note = crate::types::StoredNote {
            header,
            encrypted_data: note.encrypted_details,
            created_at: Utc::now(),
            received_at: Utc::now(),
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

        // Default to epoch start (1970-01-01) to fetch all notes if no timestamp provided
        let timestamp = if let Some(ts) = request.timestamp {
            DateTime::from_timestamp(
                ts.seconds,
                ts.nanos.try_into().map_err(|_| {
                    Status::invalid_argument("Negative timestamp nanoseconds".to_string())
                })?,
            )
            .ok_or_else(|| Status::invalid_argument("Invalid timestamp"))?
        } else {
            DateTime::from_timestamp(0, 0).unwrap()
        };

        // Fetch notes from database
        let notes = self
            .database
            .fetch_notes(request.tag.into(), timestamp)
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch notes: {e:?}")))?;

        // Convert to protobuf format
        let proto_notes: std::result::Result<Vec<_>, Status> = notes
            .into_iter()
            .map(|note| {
                let nanos = note.received_at.timestamp_subsec_nanos();
                let nanos_i32 = nanos
                    .try_into()
                    .map_err(|_| Status::internal("Timestamp nanoseconds too large".to_string()))?;

                Ok(EncryptedNoteTimestamped {
                    header: note.header.to_bytes(),
                    encrypted_details: note.encrypted_data,
                    timestamp: Some(prost_types::Timestamp {
                        seconds: note.received_at.timestamp(),
                        nanos: nanos_i32,
                    }),
                })
            })
            .collect();

        let proto_notes = proto_notes?;

        Ok(Response::new(FetchNotesResponse { notes: proto_notes }))
    }

    async fn health(
        &self,
        _request: Request<()>,
    ) -> std::result::Result<Response<HealthResponse>, Status> {
        let nanos = Utc::now().timestamp_subsec_nanos();
        let nanos_i32 = nanos
            .try_into()
            .map_err(|_| Status::internal("Timestamp nanoseconds too large".to_string()))?;

        Ok(Response::new(HealthResponse {
            status: "healthy".to_string(),
            timestamp: Some(prost_types::Timestamp {
                seconds: Utc::now().timestamp(),
                nanos: nanos_i32,
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
