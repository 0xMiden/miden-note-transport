use core::task::{Poll, Waker};
use std::{collections::BTreeMap, sync::Arc};

use chrono::{DateTime, Utc};
use miden_private_transport_proto::miden_private_transport::{
    StreamNotesUpdate, TransportNoteTimestamped,
};
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};

use crate::{
    database::Database,
    types::{NoteTag, proto_timestamp_to_datetime},
};

/// Streaming handler
pub struct NoteStreamer {
    manager: NoteStreamerManager,
    rx: mpsc::UnboundedReceiver<StreamerMessage>,
}

/// Streaming manager
///
/// Periodically queries new notes by note tag stored in the database and feeds them to relevant
/// subscribers.
struct NoteStreamerManager {
    /// Tracked tags
    tags: BTreeMap<NoteTag, TagData>,
    /// Sub wakers
    wakers: BTreeMap<u64, Waker>,
    /// Database
    database: Arc<Database>,
}

/// Internal control message exchanged with the [`NoteStreamer`]
pub(crate) enum StreamerMessage {
    /// New sub
    Sub(Subface),
    /// Update waker for sub
    Waker((u64, Waker)),
}

/// Tag data tracking
pub struct TagData {
    lts: DateTime<Utc>,
    subs: BTreeMap<u64, mpsc::UnboundedSender<Vec<TransportNoteTimestamped>>>,
}

/// Subscription
pub struct Sub {
    id: u64,
    rx: mpsc::UnboundedReceiver<Vec<TransportNoteTimestamped>>,
    streamer_tx: mpsc::UnboundedSender<StreamerMessage>,
}

/// Subscription interface
pub struct Subface {
    id: u64,
    tag: NoteTag,
    tx: mpsc::UnboundedSender<Vec<TransportNoteTimestamped>>,
}

impl NoteStreamerManager {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            tags: BTreeMap::new(),
            wakers: BTreeMap::new(),
            database,
        }
    }

    pub(super) async fn query_updates(
        &self,
    ) -> crate::Result<Vec<(NoteTag, Vec<TransportNoteTimestamped>)>> {
        // Update period
        sleep(Duration::from_millis(500)).await;

        let mut updates = vec![];
        for (tag, tag_data) in &self.tags {
            let snotes = self.database.fetch_notes(*tag, tag_data.lts).await?;

            // Convert to protobuf format
            let pnotes: Result<Vec<_>, _> =
                snotes.into_iter().map(TransportNoteTimestamped::try_from).collect();
            let pnotes = pnotes.map_err(|e| {
                crate::Error::Internal(format!(
                    "Failed converting into proto TransportNoteTimestamped: {e}"
                ))
            })?;

            if !pnotes.is_empty() {
                updates.push((*tag, pnotes));
            }
        }

        Ok(updates)
    }

    pub(super) fn forward_updates(
        &mut self,
        tag_notes: Vec<(NoteTag, Vec<TransportNoteTimestamped>)>,
    ) {
        // Forward updates to subs
        for (tag, notes) in tag_notes {
            if let Some(tag_data) = self.tags.get(&tag) {
                // Wake-up subs with `tag`
                for (sub_id, sub_tx) in &tag_data.subs {
                    if let Some(waker) = self.wakers.remove(sub_id) {
                        sub_tx.send(notes.clone()).unwrap();
                        waker.wake();
                    }
                }
            }
        }
    }

    pub(super) fn update_timestamps(
        &mut self,
        tag_notes: &[(NoteTag, Vec<TransportNoteTimestamped>)],
    ) {
        // Update query timestamps, to the timestamp of the most recent note
        for (tag, notes) in tag_notes {
            if let Some(tag_data) = self.tags.get_mut(tag) {
                let lts_opt = notes
                    .iter()
                    .map(|note| note.timestamp.unwrap_or_default())
                    .max_by_key(|ts| proto_timestamp_to_datetime(*ts).ok())
                    .and_then(|pts| proto_timestamp_to_datetime(pts).ok());
                if let Some(lts) = lts_opt {
                    tag_data.lts = lts;
                }
            }
        }
    }

    pub fn update_waker(&mut self, sub_id: u64, waker: Waker) {
        self.wakers.insert(sub_id, waker);
    }

    pub fn add_sub(&mut self, sub: Subface) {
        let entry = self.tags.entry(sub.tag).or_insert_with(TagData::new);
        entry.subs.insert(sub.id, sub.tx);
    }
}

impl NoteStreamer {
    pub(crate) fn new(
        database: Arc<Database>,
        rx: mpsc::UnboundedReceiver<StreamerMessage>,
    ) -> Self {
        Self {
            manager: NoteStreamerManager::new(database),
            rx,
        }
    }

    /// Streamer main loop
    pub(crate) async fn stream(self) {
        let mut manager = self.manager;
        let mut rx = self.rx;
        loop {
            if let Err(e) = Self::step(&mut manager, &mut rx).await {
                tracing::error!("Streamer error: {e}");
            }
        }
    }

    /// Streamer loop step
    async fn step(
        manager: &mut NoteStreamerManager,
        rx: &mut mpsc::UnboundedReceiver<StreamerMessage>,
    ) -> crate::Result<()> {
        tokio::select! {
            // Periodically query DB for new notes
            res = manager.query_updates() => {
                let tag_notes = res?;
                manager.update_timestamps(&tag_notes);
                manager.forward_updates(tag_notes);
            }
            // Handle streamer control messages
            Some(msg) = rx.recv() => {
                match msg {
                    StreamerMessage::Sub(sub) => manager.add_sub(sub),
                    StreamerMessage::Waker((id, waker)) => manager.update_waker(id, waker),
                }
            }
        }
        Ok(())
    }
}

impl Sub {
    pub(crate) fn new(
        id: u64,
        rx: mpsc::UnboundedReceiver<Vec<TransportNoteTimestamped>>,
        streamer_tx: mpsc::UnboundedSender<StreamerMessage>,
    ) -> Self {
        Self { id, rx, streamer_tx }
    }
}

impl Subface {
    pub fn new(
        id: u64,
        tag: NoteTag,
        tx: mpsc::UnboundedSender<Vec<TransportNoteTimestamped>>,
    ) -> Self {
        Self { id, tag, tx }
    }
}

impl TagData {
    pub fn new() -> Self {
        Self { lts: Utc::now(), subs: BTreeMap::new() }
    }
}

impl tonic::codegen::tokio_stream::Stream for Sub {
    type Item = std::result::Result<StreamNotesUpdate, tonic::Status>;

    // Required method
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Send update notes to client
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(notes)) => {
                let updates = StreamNotesUpdate { notes };
                return Poll::Ready(Some(Ok(updates)));
            },
            Poll::Ready(None) => return Poll::Ready(None),
            _ => (),
        }

        // Update streamer' stored waker
        if let Err(e) = self.streamer_tx.send(StreamerMessage::Waker((self.id, cx.waker().clone())))
        {
            tracing::error!("Streaming waker tx failure: {e}");
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}
