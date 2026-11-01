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
use rusqlite::{Connection, Row, params};
use std::{
    fs::{self, File},
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicI64, AtomicU64},
};

use crate::decode::{self, Encoding};

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

/// What the threads of a session need to know about each other.
#[derive(Debug, Default)]
pub struct Shared {
    /// Less than the reserve is left on the disk: bodies stop being kept.
    pub low_disk: AtomicBool,
    /// Flows below this id are gone.
    pub first_live: AtomicI64,
    /// The size of the search index, which counts towards the session's.
    pub index_bytes: AtomicU64,
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

    fn read_place(&mut self, place: LogPlace) -> io::Result<Vec<u8>> {
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

    #[test]
    fn the_disk_has_some_room() {
        assert!(free_space(&std::env::temp_dir()).unwrap() > 0);
    }
}
