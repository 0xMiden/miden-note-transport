

use core::task::{Poll, Waker};

pub struct NoteStreamer {
    manager: NoteStreamerManager,
    rx: mpsc::UnboundedReceiver<StreamerMessage>,
}

pub struct NoteStreamerManager {
    subs: BTreeMap<u64, Waker>,
    tags: BTreeSet<NoteTag>,
    subsf: Vec<Sub>,
    last_query: DateTime<Utc>,
    database: Arc<Database>,
}

#[derive(Clone)]
pub struct Sub {
    id: u64,
    tag: NoteTag,
    batch: Vec<TransportNoteTimestamped>,
    waker: Option<core::task::Waker>,
    streamer: mpsc::UnboundedSender<StreamerMessage>,
}

impl NoteStreamerManager {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            subs: Default::default(),
            tags: Default::default(),
            subsf: Default::default(),
            last_query: Utc::now(),
            database,
        }
    }

    pub async fn query_updates(&self) -> Vec<TransportNoteTimestamped> {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            for tag in &self.tags {
                let notes = self
                    .database
                    .fetch_notes(*tag, self.last_query)
                    .await.unwrap();
                println!("NOTESFETCHED {}", notes.len());

                // Convert to protobuf format
                let mut proto_notes_size = 0;
                let proto_notes: Result<Vec<_>, tonic::Status> = notes
                    .into_iter()
                    .map(|note| {
                        let nanos = note.created_at.timestamp_subsec_nanos();
                        let nanos_i32 = nanos
                            .try_into().unwrap();

                        let pnote = TransportNote {
                            header: note.header.to_bytes(),
                            details: note.details,
                        };

                        let ptimestamp = prost_types::Timestamp {
                            seconds: note.created_at.timestamp(),
                            nanos: nanos_i32,
                        };

                        proto_notes_size += (pnote.header.len() + pnote.details.len()) as u64;
                        Ok(TransportNoteTimestamped {
                            note: Some(pnote),
                            timestamp: Some(ptimestamp),
                        })
                    })
                .collect();
                let proto_notes = proto_notes.unwrap();

                return proto_notes;
            }
            vec![]
    }

    pub fn register(&mut self, sub_id: u64, waker: Waker) {
        self.subs.insert(sub_id, waker);
    }

    pub fn unregister(&mut self, sub_id: u64) {
        self.subs.remove(&sub_id);
    }
}

impl NoteStreamer {
    pub fn new(database: Arc<Database>, rx: mpsc::UnboundedReceiver<StreamerMessage>) -> Self {
        Self {
            manager: NoteStreamerManager::new(database),
            rx,
        }
    }

    pub async fn stream(self) {
        let mut manager = self.manager;
        let mut rx = self.rx;
        loop {
            tokio::select! {
                proto_notes = manager.query_updates() => {
                    for sub in &mut manager.subsf {
                        if let Some(waker) = manager.subs.remove(&sub.id) {
                            sub.push(&proto_notes);
                            waker.wake();
                        }
                    }
                }
                Some(msg) = rx.recv() => {
                    match msg {
                        StreamerMessage::Sub(sub) => {
                            manager.tags.insert(sub.tag);
                            manager.subsf.push(sub);
                        }
                        StreamerMessage::Waker((id, waker)) => {
                            manager.register(id, waker);
                        }
                    }
                }
            }
        }
    }

}

pub enum StreamerMessage {
    Sub(Sub),
    Waker((u64, Waker)),
}

impl Sub {
    pub fn new(id: u64, tag: NoteTag, streamer: mpsc::UnboundedSender<StreamerMessage>) -> Self {
        Self {
            id,
            tag,
            batch: Vec::new(),
            waker: None,
            streamer,
        }
    }

    pub fn push(&mut self, notes: &[TransportNoteTimestamped]) {
        self.batch.extend_from_slice(notes);
        println!("--> PUSHING TO SUB {}", self.batch.len());
    }
}

impl tonic::codegen::tokio_stream::Stream for Sub {
    type Item = std::result::Result<StreamNotesUpdate, tonic::Status>;

    // Required method
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        println!("POLL {}", self.batch.len());
        if !self.batch.is_empty() {
            let updates = StreamNotesUpdate { 
                notes: std::mem::take(&mut self.batch)
            };
            return Poll::Ready(Some(Ok(updates)));
        }

        {
            self.streamer.send(StreamerMessage::Waker((self.id, cx.waker().clone()))).unwrap();
        }

        Poll::Pending
    }
}

