//! A session on disk: one directory, and everything in it can be deleted by
//! deleting the directory.
//!
//! - `flows.db`: one row per flow, and where its bodies are. No bodies, which is
//!   what keeps a page of the list as fast at 50 GB as at 10.
//! - `bodies/N.log`: small bodies, one after another, as the wire sent them.
//!   Never rewritten; the oldest goes whole when the session is over its limit.
//!   That is what gives disk back at once, where a database file stays its size
//!   until a VACUUM that took 34 minutes and twice the space at 50 GB.
//! - `files/N.body`: large bodies, one file each, written as they arrive.
//! - `search/N.db`: the search index, in the same kind of pieces, and for the same
//!   reason: deleting one took 6 ms, deleting the same rows from one database up
//!   to 32 s, and left it larger and slower.

use anyhow::{Result, bail};
use rusqlite::{Connection, OpenFlags, Row, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, AtomicU64},
    },
};

use crate::{
    decode::{self, Encoding},
    store::Stats,
};

pub fn flows_db(session: &Path) -> PathBuf {
    session.join("flows.db")
}

pub fn bodies_dir(session: &Path) -> PathBuf {
    session.join("bodies")
}

pub fn files_dir(session: &Path) -> PathBuf {
    session.join("files")
}

pub fn search_dir(session: &Path) -> PathBuf {
    session.join("search")
}

/// A new session gets a directory of its own. Going on with an old one would
/// mean finding out what it holds, and nothing needs that yet.
pub fn create(session: &Path) -> Result<()> {
    if flows_db(session).exists() {
        bail!("{} already holds a session", session.display());
    }
    fs::create_dir_all(bodies_dir(session))?;
    fs::create_dir_all(files_dir(session))?;
    fs::create_dir_all(search_dir(session))?;
    Ok(())
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Meta {
    name: Option<String>,
}

fn meta_path(session: &Path) -> PathBuf {
    session.join("session.toml")
}

/// A session's name, if it was given one. A named session is kept until the user
/// deletes it.
pub fn name_of(session: &Path) -> Option<String> {
    let text = fs::read_to_string(meta_path(session)).ok()?;
    toml::from_str::<Meta>(&text).ok()?.name
}

/// Names a session, or with nothing but spaces takes its name away.
pub fn set_name(session: &Path, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return match fs::remove_file(meta_path(session)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        };
    }
    let meta = Meta {
        name: Some(name.to_owned()),
    };
    fs::write(meta_path(session), toml::to_string(&meta)?)?;
    Ok(())
}

/// What the sessions screen says about one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionInfo {
    pub dir: PathBuf,
    pub name: Option<String>,
    pub flows: u64,
    pub bytes: u64,
    /// When its first and last flow started, in microseconds since 1970.
    pub first_us: Option<i64>,
    pub last_us: Option<i64>,
    /// False if this version cannot read it: an older one wrote it, or it is damaged.
    pub readable: bool,
}

/// Rows of the format this version writes.
const VERSION: i64 = 2;

fn dir_bytes(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => dir_bytes(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

/// How many flows, and when the first and the last started.
type Rows = (u64, Option<i64>, Option<i64>);

fn read_rows(session: &Path) -> rusqlite::Result<Option<Rows>> {
    let flows = Connection::open_with_flags(flows_db(session), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version: i64 = flows.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version != VERSION {
        return Ok(None);
    }
    // Ids have no gaps, and the first and last are found by the primary key.
    let (first, last): (Option<i64>, Option<i64>) = flows.query_row(
        "SELECT (SELECT min(id) FROM flows), (SELECT max(id) FROM flows)",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let at = |id: Option<i64>| -> rusqlite::Result<Option<i64>> {
        id.map(|id| {
            flows.query_row(
                "SELECT started_us FROM flows WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
        })
        .transpose()
    };
    let count = first.zip(last).map_or(0, |(a, b)| (b - a + 1) as u64);
    Ok(Some((count, at(first)?, at(last)?)))
}

pub fn info(session: &Path) -> SessionInfo {
    let rows = read_rows(session).ok().flatten();
    SessionInfo {
        dir: session.to_owned(),
        name: name_of(session),
        flows: rows.map_or(0, |r| r.0),
        bytes: dir_bytes(session),
        first_us: rows.and_then(|r| r.1),
        last_us: rows.and_then(|r| r.2),
        readable: rows.is_some(),
    }
}

fn session_dirs(sessions: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(sessions) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| flows_db(path).exists())
        .collect();
    dirs.sort_unstable();
    dirs
}

/// Every session in a folder, the newest first.
pub fn list(sessions: &Path) -> Vec<SessionInfo> {
    let mut all: Vec<SessionInfo> = session_dirs(sessions).iter().map(|d| info(d)).collect();
    all.reverse();
    all
}

/// Deletes a session. Only what is one: this is a whole folder going.
pub fn delete(session: &Path) -> Result<()> {
    if !flows_db(session).exists() {
        bail!("{} is not a session", session.display());
    }
    fs::remove_dir_all(session)?;
    Ok(())
}

/// Makes room as a session starts. A named session stays, one with no flows does
/// not, and of the rest the newest `kept` do.
pub fn forget_old(sessions: &Path, kept: u64) {
    let mut unnamed = Vec::new();
    for dir in session_dirs(sessions) {
        let about = info(&dir);
        if about.name.is_some() {
            continue;
        }
        if about.readable && about.flows == 0 {
            let _ = fs::remove_dir_all(&dir);
        } else {
            unnamed.push(dir);
        }
    }
    let extra = unnamed.len().saturating_sub(kept as usize);
    for old in &unnamed[..extra] {
        let _ = fs::remove_dir_all(old);
    }
}

/// What the threads of a session need to know about each other.
#[derive(Debug, Default)]
pub struct Shared {
    /// Less than the reserve is left on the disk: bodies stop being kept.
    pub low_disk: AtomicBool,
    /// Flows below this id are gone.
    pub first_live: AtomicI64,
    /// The size of the search index, which counts towards the session's.
    pub index_bytes: AtomicU64,
    pub stats: Arc<Stats>,
}

/// Room left on the disk holding `path`, for this user.
#[cfg(unix)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: the name is a valid C string and the buffer is the size asked for;
    // the buffer is read only after the call says it filled it.
    let stats = unsafe {
        if libc::statvfs(name.as_ptr(), stats.as_mut_ptr()) != 0 {
            return None;
        }
        stats.assume_init()
    };
    Some(stats.f_bavail as u64 * stats.f_frsize as u64)
}

#[cfg(windows)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let name: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut free = 0u64;
    // SAFETY: the name ends in a zero and the other two results may be null.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            name.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

/// Where a body starts in the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogPlace {
    pub segment: u32,
    pub offset: u64,
    pub len: u32,
}

fn segment_path(dir: &Path, segment: u32) -> PathBuf {
    dir.join(format!("{segment:08}.log"))
}

pub fn remove_segment(session: &Path, segment: u32) -> io::Result<()> {
    fs::remove_file(segment_path(&bodies_dir(session), segment))
}

/// The writing end of the log. Bodies wait in memory for [`flush`](Self::flush);
/// a body counts as kept only once that has returned.
pub struct BodyLog {
    dir: PathBuf,
    segment: u32,
    file: Option<File>,
    written: u64,
    pending: Vec<u8>,
}

impl BodyLog {
    pub fn new(session: &Path) -> Self {
        Self {
            dir: bodies_dir(session),
            segment: 1,
            file: None,
            written: 0,
            pending: Vec::new(),
        }
    }

    pub fn segment(&self) -> u32 {
        self.segment
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    pub fn append(&mut self, data: &[u8]) -> LogPlace {
        let place = LogPlace {
            segment: self.segment,
            offset: self.written + self.pending.len() as u64,
            len: data.len() as u32,
        };
        self.pending.extend_from_slice(data);
        place
    }

    /// On an error everything since the last flush is gone, and the file is as it
    /// was then, so its place can be used again.
    pub fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let result = self.write_pending();
        match result {
            Ok(()) => self.written += self.pending.len() as u64,
            Err(_) => {
                // Gives back what a partial write took; a failure here only
                // leaves it there to be overwritten.
                if let Some(file) = &self.file {
                    let _ = file.set_len(self.written);
                }
            }
        }
        self.pending.clear();
        result
    }

    fn write_pending(&mut self) -> io::Result<()> {
        if self.file.is_none() {
            self.file = Some(File::create(segment_path(&self.dir, self.segment))?);
        }
        let file = self.file.as_mut().expect("just made");
        file.seek(SeekFrom::Start(self.written))?;
        file.write_all(&self.pending)
    }

    /// Starts the next segment. Only between flushes.
    pub fn next_segment(&mut self) {
        debug_assert!(self.pending.is_empty());
        self.segment += 1;
        self.file = None;
        self.written = 0;
    }
}

/// How one body of one flow was kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredBody {
    pub place: Option<LogPlace>,
    pub file: Option<String>,
    /// We compressed it, it came plain.
    pub ours: bool,
    /// The `Content-Encoding` it came with.
    pub encoding: Option<String>,
}

impl StoredBody {
    /// The columns [`from_row`](Self::from_row) reads, for `side` as `req` or `resp`.
    pub fn columns(side: &str) -> String {
        format!(
            "{side}_segment, {side}_offset, {side}_len, {side}_spool, {side}_zstd, {side}_encoding"
        )
    }

    pub fn from_row(row: &Row, first: usize) -> rusqlite::Result<Self> {
        let segment: Option<i64> = row.get(first)?;
        let place = match segment {
            Some(segment) => Some(LogPlace {
                segment: segment as u32,
                offset: row.get::<_, i64>(first + 1)? as u64,
                len: row.get::<_, i64>(first + 2)? as u32,
            }),
            None => None,
        };
        Ok(Self {
            place,
            file: row.get(first + 3)?,
            ours: row.get(first + 4)?,
            encoding: row.get(first + 5)?,
        })
    }

    pub fn exists(&self) -> bool {
        self.place.is_some() || self.file.is_some()
    }

    fn packing(&self) -> Encoding {
        if self.ours {
            Encoding::Zstd
        } else {
            Encoding::from_header(self.encoding.as_deref())
        }
    }
}

/// The reading end. Keeps the last segment open, since reads tend to stay in one.
/// Anything missing or short means the body is gone: its file was dropped with
/// the oldest flows, or the disk filled up.
pub struct BodyReader {
    session: PathBuf,
    open: Option<(u32, File)>,
}

impl BodyReader {
    pub fn new(session: &Path) -> Self {
        Self {
            session: session.to_owned(),
            open: None,
        }
    }

    pub fn read_place(&mut self, place: LogPlace) -> io::Result<Vec<u8>> {
        if self.open.as_ref().is_none_or(|(n, _)| *n != place.segment) {
            let path = segment_path(&bodies_dir(&self.session), place.segment);
            self.open = Some((place.segment, File::open(path)?));
        }
        let (_, file) = self.open.as_mut().expect("just opened");
        file.seek(SeekFrom::Start(place.offset))?;
        let mut data = vec![0; place.len as usize];
        file.read_exact(&mut data)?;
        Ok(data)
    }

    /// The body as kept, packed.
    pub fn source(&mut self, body: &StoredBody) -> io::Result<Box<dyn Read + Send>> {
        if let Some(place) = body.place {
            return Ok(Box::new(Cursor::new(self.read_place(place)?)));
        }
        let Some(name) = &body.file else {
            return Err(io::ErrorKind::NotFound.into());
        };
        Ok(Box::new(File::open(files_dir(&self.session).join(name))?))
    }

    /// The text or data of the body, up to `max` bytes of it. `None` if it is
    /// gone or cannot be read.
    pub fn unpacked(&mut self, body: &StoredBody, max: usize) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        self.unpack_into(body, max as u64, &mut out)?;
        Some(out)
    }

    pub fn unpack_into(
        &mut self,
        body: &StoredBody,
        max: u64,
        out: &mut impl Write,
    ) -> Option<u64> {
        let source = self.source(body).ok()?;
        decode::unpack_into(source, body.packing(), max, out)
    }
}

/// A flow's headers as one string: the two parts can hold anything but a NUL,
/// which HTTP does not allow in a header.
pub fn join_headers(request: &str, response: &str) -> String {
    format!("{request}\0{response}")
}

pub type Headers = Vec<(String, String)>;

/// `name: value` lines, one per header, as the capture writes them.
pub fn parse_headers(lines: &str) -> Headers {
    lines
        .lines()
        .filter_map(|line| line.split_once(": "))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// A flow's request and response headers, if they were kept.
pub fn stored_headers(
    flows: &Connection,
    reader: &mut BodyReader,
    id: i64,
) -> Result<Option<(Headers, Headers)>> {
    let place: Option<LogPlace> = flows.query_row(
        "SELECT head_segment, head_offset, head_len FROM flows WHERE id = ?1",
        params![id],
        |row| {
            let segment: Option<i64> = row.get(0)?;
            Ok(match segment {
                Some(segment) => Some(LogPlace {
                    segment: segment as u32,
                    offset: row.get::<_, i64>(1)? as u64,
                    len: row.get::<_, i64>(2)? as u32,
                }),
                None => None,
            })
        },
    )?;
    let Some(place) = place else {
        return Ok(None);
    };
    let Ok(both) = reader.read_place(place) else {
        return Ok(None);
    };
    let both = String::from_utf8_lossy(&both);
    let (request, response) = both.split_once('\0').unwrap_or((&both, ""));
    Ok(Some((parse_headers(request), parse_headers(response))))
}

/// One message of a stream, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredMessage {
    pub id: i64,
    pub at_us: i64,
    pub from_client: bool,
    /// `text`, `binary`, `ping`, `pong`, `close`, or `event` for server-sent events.
    pub kind: String,
    /// How long it was on the wire. What is kept stops at `log_max`.
    pub bytes: u64,
    pub place: Option<LogPlace>,
    /// Let go: too much was waiting, or the disk had no room.
    pub shed: bool,
}

/// A flow's messages after this one, the oldest first.
pub fn stored_messages(
    flows: &Connection,
    flow_id: i64,
    after: i64,
    limit: i64,
) -> rusqlite::Result<Vec<StoredMessage>> {
    let mut stmt = flows.prepare_cached(
        "SELECT id, at_us, from_client, kind, bytes, segment, offset, len, shed FROM messages
         WHERE flow_id = ?1 AND id > ?2 ORDER BY id LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![flow_id, after, limit], |r| {
        let segment: Option<i64> = r.get(5)?;
        let place = match segment {
            Some(segment) => Some(LogPlace {
                segment: segment as u32,
                offset: r.get::<_, i64>(6)? as u64,
                len: r.get::<_, i64>(7)? as u32,
            }),
            None => None,
        };
        Ok(StoredMessage {
            id: r.get(0)?,
            at_us: r.get(1)?,
            from_client: r.get(2)?,
            kind: r.get(3)?,
            bytes: r.get::<_, i64>(4)? as u64,
            place,
            shed: r.get(8)?,
        })
    })?;
    rows.collect()
}

/// How one body of a flow was kept, by flow id and `req` or `resp`.
pub fn stored_body(flows: &Connection, id: i64, side: &str) -> Result<StoredBody> {
    Ok(flows.query_row(
        &format!(
            "SELECT {} FROM flows WHERE id = ?1",
            StoredBody::columns(side)
        ),
        params![id],
        |row| StoredBody::from_row(row, 0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("session-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        create(&dir).unwrap();
        dir
    }

    #[test]
    fn bodies_come_back_from_any_segment() {
        let dir = temp("log");
        let mut log = BodyLog::new(&dir);
        let first = log.append(b"first");
        let second = log.append(b"second");
        log.flush().unwrap();
        log.next_segment();
        let third = log.append(b"third");
        log.flush().unwrap();
        assert_eq!((first.segment, third.segment), (1, 2));

        let mut reader = BodyReader::new(&dir);
        for (place, text) in [
            (second, b"second".as_slice()),
            (first, b"first"),
            (third, b"third"),
        ] {
            let body = StoredBody {
                place: Some(place),
                ..StoredBody::default()
            };
            assert_eq!(reader.unpacked(&body, 1024).unwrap(), text);
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_dropped_segment_reads_as_gone() {
        let dir = temp("gone");
        let mut log = BodyLog::new(&dir);
        let place = log.append(b"data");
        log.flush().unwrap();
        remove_segment(&dir, place.segment).unwrap();
        let body = StoredBody {
            place: Some(place),
            ..StoredBody::default()
        };
        assert_eq!(BodyReader::new(&dir).unpacked(&body, 1024), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_session_is_not_written_over() {
        let dir = temp("twice");
        fs::write(flows_db(&dir), b"").unwrap();
        assert!(create(&dir).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A session with this many flows, as far as the screen and the clean-up can tell.
    fn make(sessions: &Path, name: &str, flows: i64) -> PathBuf {
        let dir = sessions.join(name);
        create(&dir).unwrap();
        let db = Connection::open(flows_db(&dir)).unwrap();
        db.execute_batch(
            "PRAGMA user_version = 2; CREATE TABLE flows (id INTEGER PRIMARY KEY, started_us INTEGER);",
        )
        .unwrap();
        for id in 1..=flows {
            db.execute("INSERT INTO flows VALUES (?1, ?2)", params![id, id * 10])
                .unwrap();
        }
        dir
    }

    #[test]
    fn a_session_says_what_it_holds() {
        let sessions = temp("info");
        let dir = make(&sessions, "100", 3);
        let about = info(&dir);
        assert_eq!(about.flows, 3);
        assert_eq!((about.first_us, about.last_us), (Some(10), Some(30)));
        assert!(about.readable);
        assert!(about.bytes > 0);
        assert_eq!(about.name, None);
        fs::remove_dir_all(&sessions).unwrap();
    }

    #[test]
    fn an_older_format_is_not_read_and_not_lost() {
        let sessions = temp("older");
        let dir = make(&sessions, "100", 3);
        Connection::open(flows_db(&dir))
            .unwrap()
            .execute_batch("PRAGMA user_version = 0;")
            .unwrap();
        let about = info(&dir);
        assert!(!about.readable);
        forget_old(&sessions, 5);
        assert!(
            dir.exists(),
            "it has flows, so it counts as one of those kept"
        );
        fs::remove_dir_all(&sessions).unwrap();
    }

    #[test]
    fn names_come_and_go() {
        let sessions = temp("names");
        let dir = make(&sessions, "100", 1);
        assert_eq!(name_of(&dir), None);
        set_name(&dir, "  checkout bug ").unwrap();
        assert_eq!(name_of(&dir).as_deref(), Some("checkout bug"));
        set_name(&dir, "   ").unwrap();
        assert_eq!(name_of(&dir), None);
        set_name(&dir, "").unwrap();
        fs::remove_dir_all(&sessions).unwrap();
    }

    #[test]
    fn the_clean_up_keeps_the_named_and_the_newest() {
        let sessions = temp("forget");
        let named = make(&sessions, "100", 1);
        set_name(&named, "keep me").unwrap();
        let old = make(&sessions, "200", 1);
        let empty = make(&sessions, "300", 0);
        let newer = make(&sessions, "400", 1);
        let newest = make(&sessions, "500", 1);
        // Not a session: left alone, whatever it is.
        fs::create_dir_all(sessions.join("000-notes")).unwrap();
        forget_old(&sessions, 2);
        assert!(named.exists(), "named");
        assert!(!old.exists(), "past the limit");
        assert!(!empty.exists(), "nothing in it");
        assert!(newer.exists() && newest.exists());
        assert!(sessions.join("000-notes").exists());
        let names: Vec<String> = list(&sessions)
            .iter()
            .map(|s| s.dir.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["500", "400", "100"], "the newest first");
        fs::remove_dir_all(&sessions).unwrap();
    }

    #[test]
    fn only_a_session_can_be_deleted() {
        let sessions = temp("delete");
        let other = sessions.join("not-one");
        fs::create_dir_all(&other).unwrap();
        assert!(delete(&other).is_err());
        assert!(other.exists());
        let dir = make(&sessions, "100", 1);
        delete(&dir).unwrap();
        assert!(!dir.exists());
        fs::remove_dir_all(&sessions).unwrap();
    }

    #[test]
    fn the_disk_has_some_room() {
        assert!(free_space(&std::env::temp_dir()).unwrap() > 0);
    }
}
