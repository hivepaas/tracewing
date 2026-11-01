//! The search index, kept apart from the flows.
//!
//! It lives in databases of its own, filled by one thread that follows the flows
//! table. SQLite allows one writer per database, so an index in the same file
//! would hold up the flow writer whenever it fell behind, and a body index does
//! fall behind: about 22,000 rows/s, where traffic can reach 88,000/s. This way
//! search lags under a flood and catches up after it, and traffic never knows.
//!
//! The index comes in pieces, each holding a run of flows, so that the oldest can
//! be deleted whole when its flows are gone. Searching ten of them took as long
//! as searching one. Bodies are unpacked here, and only up to `index_kib_per_body`,
//! which bounds both what a zip bomb can cost and how large the index gets: with
//! no limit it came out larger than the compressed bodies it describes.

use crate::{
    decode,
    session::{self, BodyReader, Shared, StoredBody},
    settings::StorageSettings,
};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags, params};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

const BATCH: i64 = 2_000;
const IDLE: Duration = Duration::from_millis(50);
const RETRY: Duration = Duration::from_secs(1);

/// `đ` is its own letter to Unicode, not a `d` with a mark, so SQLite's
/// `remove_diacritics` leaves it. Without this, "dat hang" never finds
/// "đặt hàng". Goes on what is indexed and on what is searched for.
pub fn normalize(text: &str) -> String {
    text.replace(['đ', 'Đ'], "d")
}

#[derive(Default)]
pub struct IndexerStats {
    pub indexed: AtomicU64,
    pub last_id: AtomicU64,
    /// Batches the index would not take, most likely for want of disk.
    pub errors: AtomicU64,
}

pub struct Indexer {
    handle: JoinHandle<Result<()>>,
    stop: Arc<AtomicBool>,
    leave: Arc<AtomicBool>,
    stats: Arc<IndexerStats>,
}

impl Indexer {
    pub fn spawn(session: &Path, settings: &StorageSettings, shared: Arc<Shared>) -> Result<Self> {
        let pieces = Pieces::start(
            &session::search_dir(session),
            (settings.session_max() / 16).clamp(16 << 20, 1 << 30),
            settings,
        )?;
        let stop = Arc::new(AtomicBool::new(false));
        let leave = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(IndexerStats::default());
        let handle = {
            let session = session.to_owned();
            let settings = settings.clone();
            let (stop, leave, stats) = (Arc::clone(&stop), Arc::clone(&leave), Arc::clone(&stats));
            std::thread::Builder::new()
                .name("search-index".into())
                .spawn(move || {
                    follow(&session, &settings, pieces, &shared, &stop, &leave, &stats)
                })?
        };
        Ok(Self {
            handle,
            stop,
            leave,
            stats,
        })
    }

    pub fn stats(&self) -> Arc<IndexerStats> {
        Arc::clone(&self.stats)
    }

    /// Stops after the batch in hand, leaving the rest. A window closing should not
    /// have to wait: on a slow disk the index can be minutes behind.
    pub fn leave(self) -> Result<()> {
        self.leave.store(true, Ordering::Relaxed);
        self.finish()
    }

    /// Indexes what is left and stops.
    pub fn finish(self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .join()
            .map_err(|_| anyhow::anyhow!("search indexer panicked"))?
    }
}

fn open_piece(path: &Path, settings: &StorageSettings) -> Result<Connection> {
    let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA journal_size_limit = 67108864;",
    )?;
    if settings.index_urls {
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS urls USING fts5(
                 url, tokenize = 'trigram remove_diacritics 1', content = ''
             );",
        )?;
    }
    if settings.index_bodies {
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS bodies USING fts5(
                 body, tokenize = 'unicode61 remove_diacritics 2', content = ''
             );",
        )?;
    }
    Ok(conn)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn piece_bytes(path: &Path) -> u64 {
    ["", "-wal"]
        .iter()
        .map(|suffix| std::fs::metadata(with_suffix(path, suffix)).map_or(0, |m| m.len()))
        .sum()
}

struct Piece {
    path: PathBuf,
    last_id: i64,
}

/// The pieces so far, the oldest first. The last is being written.
struct Pieces {
    dir: PathBuf,
    target: u64,
    settings: StorageSettings,
    next: u32,
    list: VecDeque<Piece>,
    current: Connection,
}

impl Pieces {
    fn start(dir: &Path, target: u64, settings: &StorageSettings) -> Result<Self> {
        let path = dir.join(format!("{:08}.db", 1));
        let current = open_piece(&path, settings)?;
        Ok(Self {
            dir: dir.to_owned(),
            target,
            settings: settings.clone(),
            next: 2,
            list: VecDeque::from([Piece { path, last_id: 0 }]),
            current,
        })
    }

    /// Starts a new piece if the current one is big enough, and drops those whose
    /// flows are all gone. Returns the size of what is left.
    fn tidy(&mut self, last_id: i64, first_live: i64) -> Result<u64> {
        let newest = self.list.back_mut().expect("never empty");
        newest.last_id = last_id;
        if piece_bytes(&newest.path) >= self.target {
            let path = self.dir.join(format!("{:08}.db", self.next));
            self.current = open_piece(&path, &self.settings)?;
            self.next += 1;
            self.list.push_back(Piece { path, last_id });
        }
        while self.list.len() > 1 && self.list[0].last_id < first_live {
            let oldest = self.list.pop_front().expect("two at least");
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(with_suffix(&oldest.path, suffix));
            }
        }
        Ok(self.list.iter().map(|piece| piece_bytes(&piece.path)).sum())
    }
}

/// Text of one body for the index, if it has any.
fn body_text(
    reader: &mut BodyReader,
    body: &StoredBody,
    content_type: Option<&str>,
    max: usize,
) -> Option<String> {
    if !body.exists() || !decode::is_text(content_type) {
        return None;
    }
    let text = reader.unpacked(body, max)?;
    Some(decode::lossy_text(&text))
}

struct Indexing<'a> {
    reader: BodyReader,
    settings: &'a StorageSettings,
    query: String,
}

impl Indexing<'_> {
    /// One batch after `last`. Returns how many and the last id; on an error the
    /// transaction is undone, so nothing has moved.
    fn batch(
        &mut self,
        read: &Connection,
        write: &mut Connection,
        last: i64,
    ) -> Result<(u64, i64)> {
        let (index_urls, index_bodies) = (self.settings.index_urls, self.settings.index_bodies);
        let max = self.settings.index_per_body();
        let tx = write.transaction()?;
        let mut newest = last;
        let mut seen = 0;
        {
            let mut select = read.prepare_cached(&self.query)?;
            let mut rows = select.query(params![last, BATCH])?;
            while let Some(row) = rows.next()? {
                newest = row.get(0)?;
                seen += 1;
                if index_urls {
                    let url: String = row.get(1)?;
                    tx.prepare_cached("INSERT INTO urls (rowid, url) VALUES (?1, ?2)")?
                        .execute(params![newest, normalize(&url)])?;
                }
                if !index_bodies {
                    continue;
                }
                let request: Option<String> = row.get(2)?;
                let response: Option<String> = row.get(3)?;
                let req = StoredBody::from_row(row, 4)?;
                let resp = StoredBody::from_row(row, 10)?;
                let request = body_text(&mut self.reader, &req, request.as_deref(), max);
                let response = body_text(&mut self.reader, &resp, response.as_deref(), max);
                let text = match (request, response) {
                    (Some(request), Some(response)) => format!("{request}\n{response}"),
                    (Some(text), None) | (None, Some(text)) => text,
                    (None, None) => continue,
                };
                tx.prepare_cached("INSERT INTO bodies (rowid, body) VALUES (?1, ?2)")?
                    .execute(params![newest, normalize(&text)])?;
            }
        }
        tx.commit()?;
        Ok((seen, newest))
    }
}

fn follow(
    session: &Path,
    settings: &StorageSettings,
    mut pieces: Pieces,
    shared: &Shared,
    stop: &AtomicBool,
    leave: &AtomicBool,
    stats: &IndexerStats,
) -> Result<()> {
    // The writer makes the file before the proxy starts; reading never blocks it.
    let read =
        Connection::open_with_flags(session::flows_db(session), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut indexing = Indexing {
        reader: BodyReader::new(session),
        settings,
        query: format!(
            "SELECT id, host || path, req_content_type, content_type, {}, {} FROM flows
             WHERE id > ?1 ORDER BY id LIMIT ?2",
            StoredBody::columns("req"),
            StoredBody::columns("resp"),
        ),
    };
    let mut last = 0i64;
    loop {
        // Asked before the batch, so that a stop still sees everything written.
        let stopping = stop.load(Ordering::Relaxed);
        if leave.load(Ordering::Relaxed) {
            return Ok(());
        }
        // What the index writes needs the room that is running out. It can catch
        // up when there is some; on the way out it is left behind.
        if shared.low_disk.load(Ordering::Relaxed) {
            if stopping {
                return Ok(());
            }
            std::thread::sleep(RETRY);
            continue;
        }
        let seen = match indexing.batch(&read, &mut pieces.current, last) {
            Ok((seen, newest)) => {
                last = newest;
                seen
            }
            Err(e) => {
                tracing::warn!("search index: {e:#}");
                stats.errors.fetch_add(1, Ordering::Relaxed);
                if stopping {
                    return Ok(());
                }
                std::thread::sleep(RETRY);
                continue;
            }
        };
        stats.indexed.fetch_add(seen, Ordering::Relaxed);
        stats.last_id.store(last as u64, Ordering::Relaxed);
        match pieces.tidy(last, shared.first_live.load(Ordering::Relaxed)) {
            Ok(bytes) => shared.index_bytes.store(bytes, Ordering::Relaxed),
            Err(e) => {
                tracing::warn!("search index: {e:#}");
                stats.errors.fetch_add(1, Ordering::Relaxed);
            }
        }
        if seen < BATCH as u64 {
            if stopping {
                return Ok(());
            }
            std::thread::sleep(IDLE);
        }
    }
}

/// The flows a search finds, newest first. `table` is `urls` or `bodies`.
pub fn find(session: &Path, table: &str, text: &str, first_live: i64) -> Result<Vec<i64>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(session::search_dir(session))?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "db"))
        .collect();
    paths.sort_unstable();
    let query = format!("\"{}\"", normalize(text).replace('"', "\"\""));
    let mut found = Vec::new();
    for path in paths.iter().rev() {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT rowid FROM {table} WHERE {table} MATCH ?1 AND rowid >= ?2 ORDER BY rowid DESC"
        ))?;
        let ids = stmt.query_map(params![query, first_live], |r| r.get(0))?;
        for id in ids {
            found.push(id?);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("search-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        session::create(&dir).unwrap();
        dir
    }

    fn insert(path: &Path, id: i64, text: &str) {
        open_piece(path, &StorageSettings::default())
            .unwrap()
            .execute(
                "INSERT INTO bodies (rowid, body) VALUES (?1, ?2)",
                params![id, normalize(text)],
            )
            .unwrap();
    }

    #[test]
    fn unaccented_finds_vietnamese() {
        let dir = temp("words");
        insert(
            &session::search_dir(&dir).join("00000001.db"),
            1,
            "Đơn đặt hàng ở Hà Nội",
        );
        for query in ["dat hang", "đặt hàng", "ĐẶT HÀNG", "ha noi", "don"] {
            assert_eq!(find(&dir, "bodies", query, 1).unwrap(), [1], "{query}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_search_covers_every_piece_and_only_live_flows() {
        let dir = temp("pieces");
        let search = session::search_dir(&dir);
        insert(&search.join("00000001.db"), 1, "thanh toán");
        insert(&search.join("00000001.db"), 2, "thanh toán");
        insert(&search.join("00000002.db"), 3, "thanh toán");
        assert_eq!(find(&dir, "bodies", "thanh toan", 1).unwrap(), [3, 2, 1]);
        assert_eq!(find(&dir, "bodies", "thanh toan", 2).unwrap(), [3, 2]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pieces_roll_over_and_the_old_ones_go() {
        let dir = temp("roll");
        let search = session::search_dir(&dir);
        let mut pieces = Pieces::start(&search, 1, &StorageSettings::default()).unwrap();
        // Any file is past a target of one byte.
        pieces.tidy(10, 1).unwrap();
        assert_eq!(pieces.list.len(), 2);
        pieces.tidy(20, 1).unwrap();
        assert_eq!(pieces.list.len(), 3);
        assert!(search.join("00000001.db").exists());
        pieces.tidy(30, 15).unwrap();
        assert!(!search.join("00000001.db").exists(), "flows to 10 are gone");
        assert!(search.join("00000002.db").exists(), "it has flows to 20");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
