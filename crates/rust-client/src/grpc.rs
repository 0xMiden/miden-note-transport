#[cfg(all(feature = "tonic", feature = "web-tonic"))]
compile_error!("features `tonic` and `web-tonic` are mutually exclusive");

#[cfg(all(not(target_arch = "wasm32"), feature = "web-tonic"))]
compile_error!("The `web-tonic` feature is only supported when targeting wasm32.");

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;
use miden_objects::utils::{Deserializable, Serializable};
use miden_private_transport_proto::miden_private_transport::{
    FetchNotesRequest, SendNoteRequest, StreamNotesRequest, StreamNotesUpdate, TransportNote,
    miden_private_transport_client::MidenPrivateTransportClient,
};
use tonic::{Request, Streaming};
use tonic_health::pb::{HealthCheckRequest, health_client::HealthClient};
#[cfg(feature = "tonic")]
use {
    std::time::Duration,
    tonic::transport::{Channel, ClientTlsConfig},
    tower::timeout::Timeout,
};

use crate::{
    Error, NoteStream, Result,
    types::{NoteHeader, NoteInfo, NoteTag},
};

#[cfg(feature = "tonic")]
type Service = Timeout<Channel>;
#[cfg(feature = "web-tonic")]
type Service = tonic_web_wasm_client::Client;

/// gRPC client
#[derive(Clone)]
pub struct GrpcClient {
    client: MidenPrivateTransportClient<Service>,
    health_client: HealthClient<Service>,
}

impl GrpcClient {
    /// gRPC client constructor
    #[cfg(feature = "tonic")]
    pub async fn connect(endpoint: String, timeout_ms: u64) -> Result<Self> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let channel = Channel::from_shared(endpoint.clone())
            .map_err(|e| Error::Internal(format!("Invalid endpoint URI: {e}")))?
            .tls_config(tls)?
            .connect()
            .await?;
        let timeout = Duration::from_millis(timeout_ms);
        let timeout_channel = Timeout::new(channel, timeout);
        let health_client = HealthClient::new(timeout_channel.clone());
        let client = MidenPrivateTransportClient::new(timeout_channel);

        Ok(Self { client, health_client })
    }

    /// gRPC client (WASM) constructor
    #[cfg(feature = "web-tonic")]
    pub async fn connect(endpoint: String, _timeout_ms: u64) -> Result<Self> {
        let client = tonic_web_wasm_client::Client::new(endpoint);
        let health_client = HealthClient::new(client.clone());
        let client = MidenPrivateTransportClient::new(client.clone());

        Ok(Self { client, health_client })
    }

    /// Send a note
    ///
    /// Pushes a note to the transport layer.
    /// While the note header goes in plaintext, the provided note details can be encrypted.
    async fn send_note_internal(&mut self, header: NoteHeader, details: Vec<u8>) -> Result<()> {
        let request = SendNoteRequest {
            note: Some(TransportNote { header: header.to_bytes(), details }),
        };

        let response = self
            .client
            .clone()
            .send_note(Request::new(request))
            .await
            .map_err(|e| Error::Internal(format!("Send note failed: {e:?}")))?;

        let _response = response.into_inner();

        Ok(())
    }

    /// Fetch notes
    ///
    /// Downloads notes for a given tag.
    /// Only notes with cursor greater than the provided cursor are returned.
    pub async fn fetch_notes(&mut self, tag: NoteTag, cursor: u64) -> Result<Vec<NoteInfo>> {
        let request = FetchNotesRequest { tag: tag.as_u32(), cursor };

        let response = self
            .client
            .clone()
            .fetch_notes(Request::new(request))
            .await
            .map_err(|e| Error::Internal(format!("Fetch notes failed: {e:?}")))?;

        let response = response.into_inner();

        // Convert protobuf notes to internal format
        let mut notes = Vec::new();

        for pg_note in response.notes {
            let note = pg_note
                .note
                .ok_or_else(|| Error::Internal("Fetched note has no data".to_string()))?;
            let header = NoteHeader::read_from_bytes(&note.header)
                .map_err(|e| Error::Internal(format!("Invalid note header: {e:?}")))?;

            notes.push(NoteInfo {
                header,
                details: note.details,
                cursor: pg_note.cursor,
            });
        }

        Ok(notes)
    }

    /// Stream notes
    ///
    /// Subscribes to a given tag.
    /// New notes are received periodically.
    pub async fn stream_notes(&mut self, tag: NoteTag, cursor: u64) -> Result<NoteStreamAdapter> {
        let request = StreamNotesRequest { tag: tag.as_u32(), cursor };

        let response = self
            .client
            .stream_notes(request)
            .await
            .map_err(|e| Error::Internal(format!("Stream notes failed: {e:?}")))?;
        Ok(NoteStreamAdapter::new(response.into_inner()))
    }

    /// gRPC-standardized server health-check
    pub async fn health_check(&mut self) -> Result<()> {
        let request = tonic::Request::new(HealthCheckRequest {
            service: String::new(), // empty string -> whole server
        });

        let response = self.health_client.check(request).await?.into_inner();

        let serving = matches!(
            response.status(),
            tonic_health::pb::health_check_response::ServingStatus::Serving
        );

        serving
            .then_some(())
            .ok_or_else(|| tonic::Status::unavailable("Service is not serving").into())
    }
}

#[cfg_attr(not(feature = "web-tonic"), async_trait::async_trait)]
#[cfg_attr(feature = "web-tonic", async_trait::async_trait(?Send))]
impl super::TransportClient for GrpcClient {
    async fn send_note(&mut self, header: NoteHeader, details: Vec<u8>) -> Result<()> {
        self.send_note_internal(header, details).await
    }

    async fn fetch_notes(
        &mut self,
        tag: NoteTag,
        cursor: u64,
    ) -> Result<Vec<crate::types::NoteInfo>> {
        self.fetch_notes(tag, cursor).await
    }

    async fn stream_notes(&mut self, tag: NoteTag, cursor: u64) -> Result<Box<dyn NoteStream>> {
        let stream = self.stream_notes(tag, cursor).await?;
        Ok(Box::new(stream))
    }
}

/// Convert from `tonic::Streaming<StreamNotesUpdate>` to [`NoteStream`]
pub struct NoteStreamAdapter {
    inner: Streaming<StreamNotesUpdate>,
}

impl NoteStreamAdapter {
    /// Create a new [`NoteStreamAdapter`]
    pub fn new(stream: Streaming<StreamNotesUpdate>) -> Self {
        Self { inner: stream }
    }
}

impl Stream for NoteStreamAdapter {
    type Item = Result<Vec<NoteInfo>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(update))) => {
                // Convert StreamNotesUpdate to Vec<NoteInfo>
                let mut notes = Vec::new();
                for pg_note in update.notes {
                    if let Some(note) = pg_note.note {
                        let header = NoteHeader::read_from_bytes(&note.header)
                            .map_err(|e| Error::Internal(format!("Invalid note header: {e:?}")))?;

                        notes.push(NoteInfo {
                            header,
                            details: note.details,
                            cursor: pg_note.cursor,
                        });
                    }
                }
                Poll::Ready(Some(Ok(notes)))
            },
            Poll::Ready(Some(Err(status))) => Poll::Ready(Some(Err(status.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl NoteStream for NoteStreamAdapter {}
