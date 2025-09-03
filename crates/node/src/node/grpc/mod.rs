mod streaming;

use std::{net::SocketAddr, sync::Arc};

use chrono::{DateTime, Utc};
use miden_objects::utils::Deserializable;
use miden_private_transport_proto::miden_private_transport::{
    FetchNotesRequest, FetchNotesResponse, HealthResponse, SendNoteRequest, SendNoteResponse,
    StatsResponse, StreamNotesRequest, TransportNoteTimestamped,
    miden_private_transport_server::MidenPrivateTransportServer,
};
use rand::Rng;
use tokio::sync::mpsc;
use tonic::Status;

use self::streaming::{NoteStreamer, StreamerMessage, Sub, Subface};
use crate::{database::Database, metrics::MetricsGrpc};

/// Miden Private Transport gRPC server
pub struct GrpcServer {
    database: Arc<Database>,
    config: GrpcServerConfig,
    streamer: StreamerCtx,
    metrics: MetricsGrpc,
}

/// [`GrpcServer`] configuration
#[derive(Clone, Debug)]
pub struct GrpcServerConfig {
    pub host: String,
    pub port: u16,
    pub max_note_size: usize,
}

/// Streaming task interface context
pub(super) struct StreamerCtx {
    tx: mpsc::Sender<StreamerMessage>,
    handle: tokio::task::JoinHandle<()>,
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
    pub fn new(database: Arc<Database>, config: GrpcServerConfig, metrics: MetricsGrpc) -> Self {
        let streamer = StreamerCtx::spawn(database.clone());
        Self { database, config, streamer, metrics }
    }

    pub fn into_service(self) -> MidenPrivateTransportServer<Self> {
        MidenPrivateTransportServer::new(self)
    }

    pub async fn serve(self) -> crate::Result<()> {
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

impl StreamerCtx {
    /// Spawn a [`NoteStreamer`] task
    ///
    /// Returns related context composed of the handle and `mpsc::Sender` `tx` for control messages.
    pub(super) fn spawn(database: Arc<Database>) -> Self {
        let (tx, rx) = mpsc::channel(128);
        let handle = tokio::spawn(NoteStreamer::new(database, rx).stream());
        Self { tx, handle }
    }
}

#[tonic::async_trait]
impl miden_private_transport_proto::miden_private_transport::miden_private_transport_server::MidenPrivateTransport
    for GrpcServer
{
    #[tracing::instrument(skip(self), fields(operation = "grpc.send_note.request"))]
    async fn send_note(
        &self,
        request: tonic::Request<SendNoteRequest>,
    ) -> Result<tonic::Response<SendNoteResponse>, tonic::Status> {
        let request_data = request.into_inner();
        let pnote = request_data.note.ok_or_else(|| Status::invalid_argument("Missing note"))?;

        let timer = self.metrics.grpc_send_note_request((pnote.header.len() + pnote.details.len()) as u64);

        // Validate note size
        if pnote.details.len() > self.config.max_note_size {
            return Err(Status::resource_exhausted(format!("Note too large ({})", pnote.details.len())));
        }

        // Convert protobuf request to internal types
        let header = miden_objects::note::NoteHeader::read_from_bytes(&pnote.header)
            .map_err(|e| Status::invalid_argument(format!("Invalid header: {e:?}")))?;

        // Create note for database
        let note_for_db = crate::types::StoredNote {
            header,
            details: pnote.details,
            created_at: Utc::now(),
        };

        self.database
            .store_note(&note_for_db)
            .await.map_err(|e| tonic::Status::internal(format!("Failed to store note: {e:?}")))?;

        timer.finish("ok");

        Ok(tonic::Response::new(SendNoteResponse {
            id: note_for_db.header.id().to_hex(),
            status: miden_private_transport_proto::miden_private_transport::NoteStatus::Sent as i32,
        }))
    }

    #[tracing::instrument(skip(self), fields(operation = "grpc.fetch_notes.request"))]
    async fn fetch_notes(
        &self,
        request: tonic::Request<FetchNotesRequest>,
    ) -> Result<tonic::Response<FetchNotesResponse>, tonic::Status> {
        let timer = self.metrics.grpc_fetch_notes_request();

        let request_data = request.into_inner();
        let tag = request_data.tag;

        // Default to epoch start (1970-01-01) to fetch all notes if no timestamp provided
        let timestamp = if let Some(ts) = request_data.timestamp {
            DateTime::from_timestamp(
                ts.seconds,
                ts.nanos.try_into().map_err(|_| {
                    tonic::Status::invalid_argument("Negative timestamp nanoseconds".to_string())
                })?,
            )
            .ok_or_else(|| tonic::Status::invalid_argument("Invalid timestamp"))?
        } else {
            DateTime::from_timestamp(0, 0).unwrap()
        };

        let notes = self
            .database
            .fetch_notes(tag.into(), timestamp)
            .await.map_err(|e| tonic::Status::internal(format!("Failed to fetch notes: {e:?}")))?;

        // Convert to protobuf format
        let proto_notes: Result<Vec<_>, _> = notes
            .into_iter()
            .map(TransportNoteTimestamped::try_from)
            .collect();
        let proto_notes = proto_notes.map_err(|e| tonic::Status::internal(format!("Failed converting into proto TransportNoteTimestamped: {e}")))?;

        timer.finish("ok");

        let proto_notes_size = proto_notes.iter().map(|tsnote| tsnote.note.as_ref().map_or(0, |pnote| (pnote.header.len() + pnote.details.len()) as u64)).sum();
        self.metrics.grpc_fetch_notes_response(
            proto_notes.len() as u64,
            proto_notes_size,
        );

        Ok(tonic::Response::new(FetchNotesResponse { notes: proto_notes }))
    }

    type StreamNotesStream = Sub;
    #[tracing::instrument(skip(self), fields(operation = "grpc.stream_notes.request"))]
    async fn stream_notes(
        &self,
        request: tonic::Request<StreamNotesRequest>,
    ) -> Result<tonic::Response<Self::StreamNotesStream>, tonic::Status> {
        let request_data = request.into_inner();
        let tag = request_data.tag;
        let id = rand::rng().random();
        let (sub_tx, sub_rx) = mpsc::channel(32);
        let sub = Sub::new(id, sub_rx, self.streamer.tx.clone());
        let subf = Subface::new(id, tag.into(), sub_tx);
        self.streamer.tx.try_send(StreamerMessage::Sub(subf))
                    .map_err(|e| tonic::Status::internal(format!("Failed sending internal streamer message: {e}")))?;

        Ok(tonic::Response::new(sub))
    }

    #[tracing::instrument(skip(self), fields(operation = "health"))]
    async fn health(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<HealthResponse>, tonic::Status> {
        let now = Utc::now();
        let timestamp = prost_types::Timestamp {
            seconds: now.timestamp(),
            nanos: now.timestamp_subsec_nanos()
                    .try_into()
                    .map_err(|_| tonic::Status::internal("Timestamp nanoseconds too large".to_string()))?,
        };

        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: Some(timestamp),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        tracing::info!(operation = "health", event = "completed", status = "success");
        Ok(tonic::Response::new(response))
    }

    #[tracing::instrument(skip(self), fields(operation = "stats"))]
    async fn stats(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<StatsResponse>, tonic::Status> {
        let (total_notes, total_tags) = self
            .database
            .get_stats()
            .await.map_err(|e| tonic::Status::internal(format!("Failed to get stats: {e:?}")))?;

        let response = StatsResponse {
            total_notes,
            total_tags,
            notes_per_tag: Vec::new(), // TODO: Implement notes_per_tag
        };

        Ok(tonic::Response::new(response))
    }
}

impl Drop for StreamerCtx {
    fn drop(&mut self) {
        if let Err(e) = self.tx.try_send(StreamerMessage::Shutdown) {
            tracing::error!("Streamer shutdown message sending failure: {e}");
            self.handle.abort();
        }
    }
}
