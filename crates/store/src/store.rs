//! Flow storage.
//!
//! Flows go through a bounded queue to one writer thread, which appends their
//! bodies to the log and inserts their rows in batches. The proxy never waits on
//! disk: when the queue is full the flow is counted as dropped instead.
//!
//! The queue is bounded twice. A count alone is not enough, since 100,000 flows
//! with 64 KiB bodies came to 6 GB when the disk fell behind. Past the byte
//! budget a flow keeps its row and loses its bodies, and says so.
//!
//! The writer also keeps the session inside its limits. A disk that fills up is
//! the same to it as any other reason a body cannot be kept: the flow says so
//! and the writer carries on, because traffic must not notice.

use crate::{
    decode::{self, Encoding},
    session::{self, BodyLog, LogPlace, Shared},
    settings::StorageSettings,
    spool::Spool,
};
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const BATCH_MAX: usize = 5_000;
/// Bodies written to the log at a time. A failure loses at most this much.
const LOG_CHUNK: usize = 4 * 1024 * 1024;
/// How many rows one step of trimming deletes, so a flood is never held up long.
const TRIM_ROWS: i64 = 5_000;
const LOOK_EVERY: Duration = Duration::from_millis(500);
const WARN_EVERY: Duration = Duration::from_secs(5);

/// One finished exchange, as stored.
#[derive(Debug, Default)]
pub struct FlowRecord {
    pub started_us: i64,
    pub client: String,
    pub kind: &'static str,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    /// The version the client spoke to the proxy.
    pub client_version: String,
    /// The version the proxy spoke to the server.
    pub server_version: Option<String>,
    pub status: Option<u16>,
    pub content_type: Option<String>,
    pub req_content_type: Option<String>,
    pub req_encoding: Option<String>,
    pub resp_encoding: Option<String>,
    /// `name: value` lines. They go in the log, not the row: the rows stay small,
    /// which is what keeps the list fast.
    pub req_headers: String,
    pub resp_headers: String,
    pub head: Option<LogPlace>,
    pub ttfb_us: Option<i64>,
    pub duration_us: Option<i64>,
    pub request: BodyRecord,
    pub response: BodyRecord,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct BodyRecord {
    pub bytes: u64,
    /// The body as it went over the wire, or its first `log_max` bytes, or nothing
    /// if all of it is in a file.
    pub captured: Vec<u8>,
    /// Only part of it was kept.
    pub truncated: bool,
    /// Whether the body reached its end, rather than being dropped early.
    pub complete: bool,
    /// The file holding the whole body, when it did not fit in `captured`.
    pub spool_file: Option<String>,
    /// The file stops short: the disk could not keep up.
    pub spool_lost: bool,
    /// The bytes in `captured` were let go: too much was waiting for the disk, or
    /// the disk had no room.
    pub shed: bool,
    /// Where the writer put `captured`, and whether it compressed it.
    pub place: Option<LogPlace>,
    pub ours: bool,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub submitted: AtomicU64,
    pub dropped: AtomicU64,
    pub written: AtomicU64,
    /// Flows that lost their bodies to the byte budget.
    pub shed: AtomicU64,
    /// The most bytes of bodies ever waiting.
    pub peak_queued: AtomicU64,
    /// Of `shed`, those let go because the disk was nearly full.
    pub shed_low_disk: AtomicU64,
    /// Rows the database would not take, and writes the log would not take.
    pub rows_lost: AtomicU64,
    pub log_errors: AtomicU64,
    /// Flows dropped to stay inside the session's limit.
    pub trimmed: AtomicU64,
    /// Bytes as they arrived, and as they went to the log.
    pub bytes_in: AtomicU64,
    pub bytes_logged: AtomicU64,
    /// The most the session has taken on disk.
    pub peak_session: AtomicU64,
}

impl Stats {
    /// Until the writer has dealt with everything submitted by now.
    pub fn wait_until_written(&self) {
        let submitted = self.submitted.load(Ordering::Relaxed);
        while self.written.load(Ordering::Relaxed)
            + self.rows_lost.load(Ordering::Relaxed)
            + self.dropped.load(Ordering::Relaxed)
            < submitted
        {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

enum Message {
    Flow(Box<FlowRecord>),
    Close,
}

fn body_bytes(record: &FlowRecord) -> u64 {
    (record.request.captured.len() + record.response.captured.len()) as u64
}

fn shed_body(body: &mut BodyRecord) {
    if !body.captured.is_empty() {
        body.captured = Vec::new();
        body.shed = true;
    }
}

#[derive(Clone)]
pub struct Recorder {
    tx: SyncSender<Message>,
    stats: Arc<Stats>,
    queued: Arc<AtomicU64>,
    budget: u64,
    shared: Arc<Shared>,
}

impl Recorder {
    /// Opens the session's database and starts the writer thread.
    pub fn spawn(
        session: &Path,
        settings: &StorageSettings,
        shared: Arc<Shared>,
        spool: Option<Spool>,
        queue: usize,
    ) -> Result<(Self, WriterThread)> {
        let conn = open(&session::flows_db(session))?;
        let (tx, rx) = mpsc::sync_channel(queue);
        let stats = Arc::clone(&shared.stats);
        let budget = settings.queue_budget();

        let queued = Arc::new(AtomicU64::new(0));
        let writer = Writer::new(
            conn,
            session,
            settings,
            Arc::clone(&shared),
            spool,
            Arc::clone(&stats),
            Arc::clone(&queued),
        )?;
        let handle = std::thread::Builder::new()
            .name("flow-writer".into())
            .spawn(move || writer.run(rx))?;

        let writer = WriterThread {
            handle,
            tx: tx.clone(),
        };
        Ok((
            Self {
                tx,
                stats,
                queued,
                budget,
                shared,
            },
            writer,
        ))
    }

    pub fn submit(&self, mut record: FlowRecord) {
        self.stats.submitted.fetch_add(1, Ordering::Relaxed);
        let mut size = body_bytes(&record);
        let queued = self.queued.fetch_add(size, Ordering::Relaxed) + size;
        let low_disk = self.shared.low_disk.load(Ordering::Relaxed);
        if size > 0 && (queued > self.budget || low_disk) {
            self.queued.fetch_sub(size, Ordering::Relaxed);
            size = 0;
            shed_body(&mut record.request);
            shed_body(&mut record.response);
            self.stats.shed.fetch_add(1, Ordering::Relaxed);
            if low_disk {
                self.stats.shed_low_disk.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            self.stats.peak_queued.fetch_max(queued, Ordering::Relaxed);
        }
        match self.tx.try_send(Message::Flow(Box::new(record))) {
            Ok(()) => {}
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                self.queued.fetch_sub(size, Ordering::Relaxed);
                self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Like [`submit`](Self::submit), but waits for room instead of dropping.
    /// For bulk loads, never for the proxy.
    pub fn submit_wait(&self, record: FlowRecord) {
        self.stats.submitted.fetch_add(1, Ordering::Relaxed);
        self.queued
            .fetch_add(body_bytes(&record), Ordering::Relaxed);
        if self.tx.send(Message::Flow(Box::new(record))).is_err() {
            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn stats(&self) -> Arc<Stats> {
        Arc::clone(&self.stats)
    }
}

pub struct WriterThread {
    handle: JoinHandle<Result<()>>,
    tx: SyncSender<Message>,
}

impl WriterThread {
    /// Writes every flow queued so far and stops the thread. Flows submitted
    /// afterwards, by connections still open, are counted as dropped.
    ///
    /// An explicit close is needed because hudsucker runs CONNECT tunnels in
    /// detached tasks that keep a `Recorder` alive for as long as the client
    /// keeps the tunnel open.
    pub fn close(self) -> Result<()> {
        // Blocks while the queue is full; the writer is draining it.
        let _ = self.tx.send(Message::Close);
        self.handle
            .join()
            .map_err(|_| anyhow::anyhow!("flow writer panicked"))?
    }
}

fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
    // Without a limit the WAL stays the largest it has ever been.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA journal_size_limit = 67108864;
         PRAGMA user_version = 1;
         CREATE TABLE IF NOT EXISTS flows (
             id INTEGER PRIMARY KEY,
             started_us INTEGER NOT NULL,
             client TEXT NOT NULL,
             kind TEXT NOT NULL,
             method TEXT NOT NULL,
             scheme TEXT NOT NULL,
             host TEXT NOT NULL,
             path TEXT NOT NULL,
             client_version TEXT NOT NULL,
             server_version TEXT,
             status INTEGER,
             content_type TEXT,
             req_content_type TEXT,
             ttfb_us INTEGER,
             duration_us INTEGER,
             error TEXT,
             req_bytes INTEGER NOT NULL,
             req_truncated INTEGER NOT NULL,
             req_complete INTEGER NOT NULL,
             req_encoding TEXT,
             req_segment INTEGER,
             req_offset INTEGER,
             req_len INTEGER,
             req_zstd INTEGER NOT NULL,
             req_spool TEXT,
             req_spool_lost INTEGER NOT NULL,
             req_shed INTEGER NOT NULL,
             resp_bytes INTEGER NOT NULL,
             resp_truncated INTEGER NOT NULL,
             resp_complete INTEGER NOT NULL,
             resp_encoding TEXT,
             resp_segment INTEGER,
             resp_offset INTEGER,
             resp_len INTEGER,
             resp_zstd INTEGER NOT NULL,
             resp_spool TEXT,
             resp_spool_lost INTEGER NOT NULL,
             resp_shed INTEGER NOT NULL,
             head_segment INTEGER,
             head_offset INTEGER,
             head_len INTEGER
         );
         CREATE INDEX IF NOT EXISTS flows_host ON flows (host);",
    )?;
    Ok(conn)
}

/// One segment's share of the session: its log file, and the files of the flows
/// that belong to it.
struct Segment {
    number: u32,
    bytes: u64,
    last_id: i64,
}

struct Writer {
    conn: Connection,
    session: PathBuf,
    settings: StorageSettings,
    shared: Arc<Shared>,
    spool: Option<Spool>,
    stats: Arc<Stats>,
    queued: Arc<AtomicU64>,
    log: BodyLog,
    compressor: Option<zstd::bulk::Compressor<'static>>,
    next_id: i64,
    /// A segment's share grows to this, then the next one starts. The session's
    /// limit is met by dropping whole ones, so it sets how much goes at a time.
    segment_target: u64,
    /// The oldest first; the last is the one being written.
    segments: VecDeque<Segment>,
    /// Chosen to go, with their rows still being deleted.
    trimming: VecDeque<Segment>,
    /// Rows up to this are deleted.
    deleted_to: i64,
    last_look: Instant,
    /// What the databases took at the last look.
    others: u64,
    last_warn: Option<Instant>,
}

impl Writer {
    fn new(
        conn: Connection,
        session: &Path,
        settings: &StorageSettings,
        shared: Arc<Shared>,
        spool: Option<Spool>,
        stats: Arc<Stats>,
        queued: Arc<AtomicU64>,
    ) -> Result<Self> {
        let compressor = settings
            .compress_stored
            .then(|| zstd::bulk::Compressor::new(settings.compress_level))
            .transpose()?;
        let log = BodyLog::new(session);
        let segments = VecDeque::from([Segment {
            number: log.segment(),
            bytes: 0,
            last_id: 0,
        }]);
        shared.first_live.store(1, Ordering::Relaxed);
        Ok(Self {
            conn,
            session: session.to_owned(),
            segment_target: (settings.session_max() / 16).clamp(4 << 20, 256 << 20),
            settings: settings.clone(),
            shared,
            spool,
            stats,
            queued,
            log,
            compressor,
            next_id: 1,
            segments,
            trimming: VecDeque::new(),
            deleted_to: 0,
            last_look: Instant::now(),
            others: 0,
            last_warn: None,
        })
    }

    fn run(mut self, rx: Receiver<Message>) -> Result<()> {
        let mut batch = Vec::with_capacity(BATCH_MAX);
        let mut closing = false;
        self.look_around();
        while !closing {
            match rx.recv_timeout(LOOK_EVERY) {
                Ok(Message::Flow(record)) => batch.push(*record),
                Ok(Message::Close) | Err(RecvTimeoutError::Disconnected) => closing = true,
                Err(RecvTimeoutError::Timeout) => {}
            }
            while !closing && !batch.is_empty() && batch.len() < BATCH_MAX {
                match rx.try_recv() {
                    Ok(Message::Flow(record)) => batch.push(*record),
                    Ok(Message::Close) | Err(TryRecvError::Disconnected) => closing = true,
                    Err(TryRecvError::Empty) => break,
                }
            }
            if !batch.is_empty() {
                self.write_batch(&mut batch);
                batch.clear();
            }
            if self.last_look.elapsed() >= LOOK_EVERY {
                self.look_around();
            }
            self.keep_to_limit();
            self.trim_step();
        }
        self.look_around();
        self.keep_to_limit();
        while !self.trimming.is_empty() {
            self.trim_step();
        }
        Ok(())
    }

    fn write_batch(&mut self, batch: &mut [FlowRecord]) {
        let queued: u64 = batch.iter().map(body_bytes).sum();
        let first_id = self.next_id;
        self.keep_bodies(batch);
        if let Err(e) = insert(&mut self.conn, first_id, batch) {
            // The rows are gone, and their bodies with them when their segment
            // goes. A full disk may have room again in a moment.
            self.warn(format_args!("flows not written: {e:#}"));
            self.stats
                .rows_lost
                .fetch_add(batch.len() as u64, Ordering::Relaxed);
        } else {
            self.stats
                .written
                .fetch_add(batch.len() as u64, Ordering::Relaxed);
            // Only here: lost rows give their ids back, so that ids have no gaps and
            // the list can find a row from its place.
            self.next_id += batch.len() as i64;
        }
        self.queued.fetch_sub(queued, Ordering::Relaxed);
        self.segments.back_mut().expect("never empty").last_id = self.next_id - 1;
    }

    /// Puts each flow's bodies in the log, a flow's two always in one segment.
    fn keep_bodies(&mut self, batch: &mut [FlowRecord]) {
        let mut unwritten = 0;
        for i in 0..batch.len() {
            let record = &mut batch[i];
            let request_type = record.req_content_type.clone();
            self.keep_body(
                &mut record.request,
                record.req_encoding.as_deref(),
                request_type.as_deref(),
            );
            let response_type = record.content_type.clone();
            self.keep_body(
                &mut record.response,
                record.resp_encoding.as_deref(),
                response_type.as_deref(),
            );
            self.keep_headers(record);
            let in_files = file_bytes(record, self.settings.body_max());
            let segment = self.segments.back_mut().expect("never empty");
            segment.bytes += in_files;
            let full = segment.bytes + self.log.pending() as u64 >= self.segment_target;
            if self.log.pending() >= LOG_CHUNK || full {
                self.flush_log(&mut batch[unwritten..=i]);
                unwritten = i + 1;
            }
            if full {
                self.next_segment(self.next_id + i as i64);
            }
        }
        self.flush_log(&mut batch[unwritten..]);
    }

    /// A disk that is full stays full: once now and then is enough, the counts say
    /// how much.
    fn warn(&mut self, message: std::fmt::Arguments) {
        if self.last_warn.is_none_or(|at| at.elapsed() >= WARN_EVERY) {
            self.last_warn = Some(Instant::now());
            tracing::warn!("{message}");
        }
    }

    fn check_free_space(&self) {
        if let Some(free) = session::free_space(&self.session) {
            self.shared
                .low_disk
                .store(free < self.settings.disk_reserve(), Ordering::Relaxed);
        }
    }

    fn keep_headers(&mut self, record: &mut FlowRecord) {
        if self.shared.low_disk.load(Ordering::Relaxed) {
            return;
        }
        let both = session::join_headers(&record.req_headers, &record.resp_headers);
        record.head = Some(self.log.append(both.as_bytes()));
    }

    fn keep_body(
        &mut self,
        body: &mut BodyRecord,
        encoding: Option<&str>,
        content_type: Option<&str>,
    ) {
        if body.captured.is_empty() {
            return;
        }
        // Flows queued before the disk ran low are caught here. Rows take so little
        // that the reserve lasts them a long time.
        if self.shared.low_disk.load(Ordering::Relaxed) {
            shed_body(body);
            self.stats.shed.fetch_add(1, Ordering::Relaxed);
            self.stats.shed_low_disk.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.stats
            .bytes_in
            .fetch_add(body.captured.len() as u64, Ordering::Relaxed);
        // Compressing is what limits this thread: about 500 MB/s against 1.6 GB/s
        // coming in. With bodies piling up, keeping them matters more than their
        // size, so they go as they are until the queue has gone down.
        let behind = self.queued.load(Ordering::Relaxed) > self.settings.queue_budget() / 4;
        let packed = match &mut self.compressor {
            Some(compressor)
                if !behind
                    && Encoding::from_header(encoding) == Encoding::Identity
                    && decode::is_text(content_type) =>
            {
                compressor
                    .compress(&body.captured)
                    .ok()
                    .filter(|packed| packed.len() < body.captured.len())
            }
            _ => None,
        };
        body.ours = packed.is_some();
        let data = packed.as_deref().unwrap_or(&body.captured);
        self.stats
            .bytes_logged
            .fetch_add(data.len() as u64, Ordering::Relaxed);
        body.place = Some(self.log.append(data));
        body.captured = Vec::new();
    }

    /// After this, a flow in `batch` either has its bodies on disk or says it
    /// has not.
    fn flush_log(&mut self, batch: &mut [FlowRecord]) {
        let pending = self.log.pending() as u64;
        if let Err(e) = self.log.flush() {
            self.warn(format_args!("bodies not written: {e}"));
            self.stats.log_errors.fetch_add(1, Ordering::Relaxed);
            for record in batch {
                record.head = None;
                for body in [&mut record.request, &mut record.response] {
                    if body.place.take().is_some() {
                        body.shed = true;
                        body.ours = false;
                    }
                }
            }
        } else {
            self.segments.back_mut().expect("never empty").bytes += pending;
            // A flood fills a small disk between two looks; this runs every 4 MB.
            self.check_free_space();
        }
    }

    fn next_segment(&mut self, last_id: i64) {
        self.segments.back_mut().expect("never empty").last_id = last_id;
        self.log.next_segment();
        self.segments.push_back(Segment {
            number: self.log.segment(),
            bytes: 0,
            last_id,
        });
    }

    fn database_bytes(&self) -> u64 {
        let db = session::flows_db(&self.session);
        let wal = db.with_extension("db-wal");
        [db, wal]
            .iter()
            .map(|path| std::fs::metadata(path).map_or(0, |m| m.len()))
            .sum()
    }

    /// How much room there is, and how large the databases have become.
    fn look_around(&mut self) {
        self.last_look = Instant::now();
        self.check_free_space();
        self.others = self.database_bytes() + self.shared.index_bytes.load(Ordering::Relaxed);
    }

    /// Whether the session has to give some up. After every batch, since a flood
    /// can bring 800 MB between two looks at the disk.
    fn keep_to_limit(&mut self) {
        let bodies = |segments: &VecDeque<Segment>| segments.iter().map(|s| s.bytes).sum::<u64>();
        let mut size = self.others + bodies(&self.segments);
        self.stats.peak_session.fetch_max(size, Ordering::Relaxed);
        while size > self.settings.session_max() {
            if self.segments.len() == 1 {
                let current = &self.segments[0];
                if current.bytes == 0 {
                    break;
                }
                self.next_segment(self.next_id - 1);
            }
            let oldest = self.segments.pop_front().expect("two at least");
            size -= oldest.bytes;
            self.trimming.push_back(oldest);
        }
    }

    fn trim_step(&mut self) {
        let Some(target) = self.trimming.front() else {
            return;
        };
        let (number, last_id) = (target.number, target.last_id);
        let to = (self.deleted_to + TRIM_ROWS).min(last_id);
        let mut gone = false;
        match delete_rows(&mut self.conn, self.spool.as_ref(), self.deleted_to, to) {
            Ok(deleted) => {
                self.stats.trimmed.fetch_add(deleted, Ordering::Relaxed);
                self.deleted_to = to;
                self.shared.first_live.store(to + 1, Ordering::Relaxed);
                gone = to >= last_id;
            }
            Err(e) => {
                // A full disk cannot even write down a delete. Room comes first,
                // and these rows go on the next try, by which time their bodies
                // read as gone.
                self.warn(format_args!("rows not trimmed: {e:#}"));
                let _ = session::remove_segment(&self.session, number);
            }
        }
        if gone {
            let _ = session::remove_segment(&self.session, number);
            self.trimming.pop_front();
        }
    }
}

fn file_bytes(record: &FlowRecord, body_max: u64) -> u64 {
    [&record.request, &record.response]
        .iter()
        .filter(|body| body.spool_file.is_some())
        .map(|body| body.bytes.min(body_max))
        .sum()
}

/// Deletes rows after `from`, up to `to`, and their files. The files go either
/// way: when this fails, it is most likely for want of the room they hold.
fn delete_rows(conn: &mut Connection, spool: Option<&Spool>, from: i64, to: i64) -> Result<u64> {
    let mut names = Vec::new();
    {
        let mut stmt = conn.prepare_cached(
            "SELECT req_spool, resp_spool FROM flows
             WHERE id > ?1 AND id <= ?2 AND (req_spool IS NOT NULL OR resp_spool IS NOT NULL)",
        )?;
        let mut rows = stmt.query(params![from, to])?;
        while let Some(row) = rows.next()? {
            names.extend(row.get::<_, Option<String>>(0)?);
            names.extend(row.get::<_, Option<String>>(1)?);
        }
    }
    let result = conn
        .prepare_cached("DELETE FROM flows WHERE id > ?1 AND id <= ?2")
        .and_then(|mut stmt| stmt.execute(params![from, to]));
    if let Some(spool) = spool {
        for name in names {
            spool.remove(name);
        }
    }
    Ok(result? as u64)
}

fn insert(conn: &mut Connection, first_id: i64, batch: &[FlowRecord]) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO flows (
                 id, started_us, client, kind, method, scheme, host, path,
                 client_version, server_version, status, content_type, req_content_type,
                 ttfb_us, duration_us, error,
                 req_bytes, req_truncated, req_complete, req_encoding,
                 req_segment, req_offset, req_len, req_zstd, req_spool, req_spool_lost, req_shed,
                 resp_bytes, resp_truncated, resp_complete, resp_encoding,
                 resp_segment, resp_offset, resp_len, resp_zstd, resp_spool, resp_spool_lost, resp_shed,
                 head_segment, head_offset, head_len
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                       ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27,
                       ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41)",
        )?;
        for (i, r) in batch.iter().enumerate() {
            let (req, resp) = (r.request.place, r.response.place);
            stmt.execute(params![
                first_id + i as i64,
                r.started_us,
                r.client,
                r.kind,
                r.method,
                r.scheme,
                r.host,
                r.path,
                r.client_version,
                r.server_version,
                r.status,
                r.content_type,
                r.req_content_type,
                r.ttfb_us,
                r.duration_us,
                r.error,
                r.request.bytes as i64,
                r.request.truncated,
                r.request.complete,
                r.req_encoding,
                req.map(|p| p.segment),
                req.map(|p| p.offset as i64),
                req.map(|p| p.len),
                r.request.ours,
                r.request.spool_file,
                r.request.spool_lost,
                r.request.shed,
                r.response.bytes as i64,
                r.response.truncated,
                r.response.complete,
                r.resp_encoding,
                resp.map(|p| p.segment),
                resp.map(|p| p.offset as i64),
                resp.map(|p| p.len),
                r.response.ours,
                r.response.spool_file,
                r.response.spool_lost,
                r.response.shed,
                r.head.map(|p| p.segment),
                r.head.map(|p| p.offset as i64),
                r.head.map(|p| p.len),
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}
