//! How bodies should be kept, answered by measuring.
//!
//! - `encode [FILE…]`: what servers' compression does to text, how fast it comes
//!   back, and what an index costs next to the bodies it describes.
//! - `threshold --dir D`: a body in SQLite, in a file of its own, or in an
//!   append-only log, per size.
//! - `scale --dir D --layout L --gb N`: the same mix of bodies filling a session,
//!   with reads timed on the way and the oldest tenth dropped at the end.
//! - `indextrim --dir D --rows N`: taking the oldest tenth out of a search index,
//!   row by row in one database, or by deleting a whole file among ten.

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, params};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tracewing_dev::seed::{Rng, rich_text};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Encode {
        /// Real text to measure as well as the generated kind.
        files: Vec<PathBuf>,
        /// Bodies for the index part.
        #[arg(long, default_value_t = 100_000)]
        flows: u64,
        #[arg(long)]
        dir: PathBuf,
    },
    Threshold {
        #[arg(long)]
        dir: PathBuf,
        /// Bytes per size, per way of keeping them.
        #[arg(long, default_value_t = 1 << 30)]
        bytes: u64,
    },
    Scale {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        layout: String,
        #[arg(long, default_value_t = 50)]
        gb: u64,
    },
    Indextrim {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long, default_value_t = 1_000_000)]
        rows: u64,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Encode { files, flows, dir } => encode(&files, flows, &dir),
        Command::Threshold { dir, bytes } => threshold(&dir, bytes),
        Command::Scale { dir, layout, gb } => scale(&dir, &layout, gb),
        Command::Indextrim { dir, rows } => indextrim(&dir, rows),
    }
}

// ---------------------------------------------------------------- encode

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
    out.write_all(data).expect("a Vec takes any bytes");
    out.finish().expect("a Vec takes any bytes")
}

fn ungzip(data: &[u8], out: &mut Vec<u8>) {
    flate2::read::GzDecoder::new(data)
        .read_to_end(out)
        .expect("valid gzip");
}

fn brotli_at(data: &[u8], quality: u32) -> Vec<u8> {
    let mut out = Vec::new();
    brotli::CompressorReader::new(data, 4096, quality, 22)
        .read_to_end(&mut out)
        .expect("a Vec takes any bytes");
    out
}

fn unbrotli(data: &[u8], out: &mut Vec<u8>) {
    brotli::Decompressor::new(data, 4096)
        .read_to_end(out)
        .expect("valid brotli");
}

fn unzstd(data: &[u8], out: &mut Vec<u8>) {
    zstd::stream::read::Decoder::new(data)
        .expect("a decoder")
        .read_to_end(out)
        .expect("valid zstd");
}

/// Compress and decompress one corpus in pieces of `piece` bytes.
fn codec_table(name: &str, corpus: &[u8], piece: usize) {
    let pieces: Vec<&[u8]> = corpus.chunks(piece).collect();
    type Enc<'a> = (
        &'a str,
        Box<dyn Fn(&[u8]) -> Vec<u8>>,
        fn(&[u8], &mut Vec<u8>),
    );
    let codecs: Vec<Enc> = vec![
        ("gzip 6", Box::new(gzip), ungzip),
        ("brotli 5", Box::new(|d| brotli_at(d, 5)), unbrotli),
        (
            "zstd 3",
            Box::new(|d| zstd::encode_all(d, 3).expect("zstd")),
            unzstd,
        ),
        (
            "zstd 1",
            Box::new(|d| zstd::encode_all(d, 1).expect("zstd")),
            unzstd,
        ),
    ];
    for (codec, encode, decode) in &codecs {
        let started = Instant::now();
        let packed: Vec<Vec<u8>> = pieces.iter().map(|p| encode(p)).collect();
        let enc_time = started.elapsed();
        let packed_bytes: usize = packed.iter().map(Vec::len).sum();
        let started = Instant::now();
        let mut out = Vec::with_capacity(piece * 2);
        for p in &packed {
            out.clear();
            decode(p, &mut out);
        }
        let dec_time = started.elapsed();
        println!(
            "  {name:<22} {:>8} {codec:<9} {:>5.1}:1  encode {:>6.0} MB/s  decode {:>6.0} MB/s",
            size_text(piece as u64),
            corpus.len() as f64 / packed_bytes as f64,
            corpus.len() as f64 / 1e6 / enc_time.as_secs_f64(),
            corpus.len() as f64 / 1e6 / dec_time.as_secs_f64(),
        );
    }
}

/// 60% small, 30% medium, 9% large, 1% very large: about 26 KB on average.
fn mixed_size(rng: &mut Rng) -> usize {
    let roll = rng.below(100);
    let (low, high) = match roll {
        0..=59 => (1 << 10, 4 << 10),
        60..=89 => (4 << 10, 32 << 10),
        90..=98 => (32 << 10, 256 << 10),
        _ => (256 << 10, 1 << 20),
    };
    low + rng.below((high - low) as u64) as usize
}

fn encode(files: &[PathBuf], flows: u64, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let generated = rich_text(&mut rng, 64 << 20).into_bytes();
    println!("== codecs");
    for piece in [2 << 10, 20 << 10, 200 << 10] {
        codec_table("generated JSON", &generated, piece);
    }
    for file in files {
        let data = fs::read(file).with_context(|| format!("read {}", file.display()))?;
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        println!("  -- {name}: {}", size_text(data.len() as u64));
        for piece in [2 << 10, 20 << 10, 200 << 10] {
            if data.len() >= piece {
                codec_table(&name, &data, piece);
            }
        }
    }

    println!("\n== {flows} flows of mixed size, what each thing costs on disk");
    let mut raw = 0u64;
    let mut gz = 0u64;
    let mut zs = 0u64;
    let path = |name: &str| dir.join(format!("{name}.db"));
    let caps: [(&str, usize); 4] = [
        ("4 KiB", 4 << 10),
        ("16 KiB", 16 << 10),
        ("64 KiB", 64 << 10),
        ("no cap", usize::MAX),
    ];
    let mut conns = Vec::new();
    for (name, _) in caps {
        let p = path(&format!("idx-{}", name.replace(' ', "")));
        let _ = fs::remove_file(&p);
        let conn = Connection::open(&p)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;
             CREATE VIRTUAL TABLE search USING fts5(
                 text, tokenize = 'unicode61 remove_diacritics 2', content = ''
             );",
        )?;
        conns.push((conn, p, Duration::ZERO, 0u64));
    }
    for tx in &conns {
        tx.0.execute_batch("BEGIN")?;
    }
    for flow in 1..=flows as i64 {
        let size = mixed_size(&mut rng);
        let text = rich_text(&mut rng, size);
        raw += text.len() as u64;
        // Every fourth body is packed, for the sizes: gzip is slow to run a
        // hundred thousand times and the ratio settles quickly.
        if flow % 4 == 0 {
            gz += gzip(text.as_bytes()).len() as u64;
            zs += zstd::encode_all(text.as_bytes(), 1)?.len() as u64;
        }
        for ((conn, _, took, indexed), (_, cap)) in conns.iter_mut().zip(caps) {
            let end = floor_char(&text, cap);
            let started = Instant::now();
            conn.prepare_cached("INSERT INTO search (rowid, text) VALUES (?1, ?2)")?
                .execute(params![flow, &text[..end]])?;
            *took += started.elapsed();
            *indexed += end as u64;
        }
        if flow % 2000 == 0 {
            for (conn, _, took, _) in &mut conns {
                let started = Instant::now();
                conn.execute_batch("COMMIT; BEGIN")?;
                *took += started.elapsed();
            }
        }
    }
    gz *= 4;
    zs *= 4;
    println!(
        "bodies as text {}, gzip 6 about {} ({:.1}:1), zstd 1 about {} ({:.1}:1)",
        size_text(raw),
        size_text(gz),
        raw as f64 / gz as f64,
        size_text(zs),
        raw as f64 / zs as f64,
    );
    for ((conn, p, took, indexed), (name, _)) in conns.into_iter().zip(caps) {
        conn.execute_batch("COMMIT; PRAGMA wal_checkpoint(TRUNCATE);")?;
        drop(conn);
        let size = fs::metadata(&p)?.len();
        println!(
            "  index, {name:<7} per body: {:>9}  {:.2}× gzip  {:.2}× text  {:>7.0} flows/s  {:>5.0} MB/s  text indexed {}",
            size_text(size),
            size as f64 / gz as f64,
            size as f64 / raw as f64,
            flows as f64 / took.as_secs_f64(),
            indexed as f64 / 1e6 / took.as_secs_f64(),
            size_text(indexed),
        );
        fs::remove_file(&p)?;
    }
    Ok(())
}

fn floor_char(text: &str, cap: usize) -> usize {
    if text.len() <= cap {
        return text.len();
    }
    let mut end = cap;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    end
}

// ---------------------------------------------------------------- threshold

/// Takes a file out of the page cache, as long after it was written.
#[cfg(target_os = "linux")]
fn forget(path: &Path) {
    use std::os::fd::AsRawFd;
    if let Ok(file) = File::open(path) {
        let _ = file.sync_all();
        unsafe {
            libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn forget(_path: &Path) {}

fn forget_dir(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            forget(&entry.path());
        }
    }
}

fn dir_bytes(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

fn percentiles(mut samples: Vec<Duration>) -> (f64, f64) {
    samples.sort_unstable();
    let at = |p: f64| samples[((samples.len() - 1) as f64 * p) as usize].as_secs_f64() * 1000.;
    (at(0.5), at(0.99))
}

fn threshold(dir: &Path, bytes: u64) -> Result<()> {
    println!(
        "{:>8} {:<8} {:>8} {:>10} {:>9} {:>17} {:>17}",
        "body", "kept in", "bodies", "write MB/s", "disk", "cold p50/p99 ms", "warm p50/p99 ms"
    );
    for size in [
        1usize << 10,
        4 << 10,
        16 << 10,
        64 << 10,
        256 << 10,
        1 << 20,
        4 << 20,
    ] {
        let count = (bytes / size as u64).clamp(200, 40_000) as usize;
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        let body = rich_text(&mut rng, size).into_bytes();
        for kind in ["sqlite", "files", "log"] {
            let work = dir.join(format!("t-{kind}"));
            let _ = fs::remove_dir_all(&work);
            fs::create_dir_all(&work)?;
            let started = Instant::now();
            let mut store = Kept::create(kind, &work)?;
            for i in 0..count {
                store.put(i as i64, &body)?;
            }
            store.finish()?;
            let took = started.elapsed();
            let disk = dir_bytes(&work);

            forget_dir(&work);
            let mut reader = Kept::open(&work, &store)?;
            let mut cold = Vec::new();
            let mut pick = Rng(0x9e37_79b9_7f4a_7c15);
            let mut buf = Vec::with_capacity(body.len());
            for _ in 0..500 {
                let id = pick.below(count as u64) as i64;
                let started = Instant::now();
                reader.get(id, &mut buf)?;
                cold.push(started.elapsed());
                assert_eq!(buf.len(), body.len());
            }
            let mut warm = Vec::new();
            for i in 0..500 {
                let id = (i % count.min(100)) as i64;
                reader.get(id, &mut buf)?;
                let started = Instant::now();
                reader.get(id, &mut buf)?;
                warm.push(started.elapsed());
            }
            let (c50, c99) = percentiles(cold);
            let (w50, w99) = percentiles(warm);
            println!(
                "{:>8} {kind:<8} {count:>8} {:>10.0} {:>9} {:>8.3}/{:<8.3} {:>8.3}/{:<8.3}",
                size_text(size as u64),
                (count * size) as f64 / 1e6 / took.as_secs_f64(),
                size_text(disk),
                c50,
                c99,
                w50,
                w99,
            );
            drop(reader);
            drop(store);
            fs::remove_dir_all(&work)?;
        }
    }
    Ok(())
}

/// One way of keeping bodies.
enum Kept {
    Sqlite(Connection, usize),
    Files(PathBuf),
    Log {
        dir: PathBuf,
        file: File,
        at: u64,
        segment: u32,
        spans: Vec<(u32, u64, u32)>,
        open: Option<(u32, File)>,
    },
}

const SEGMENT: u64 = 256 << 20;

impl Kept {
    fn create(kind: &str, dir: &Path) -> Result<Self> {
        Ok(match kind {
            "sqlite" => {
                let conn = Connection::open(dir.join("bodies.db"))?;
                conn.execute_batch(
                    "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;
                     CREATE TABLE bodies (id INTEGER PRIMARY KEY, data BLOB NOT NULL);
                     BEGIN;",
                )?;
                Self::Sqlite(conn, 0)
            }
            "files" => Self::Files(dir.to_owned()),
            "log" => Self::Log {
                dir: dir.to_owned(),
                file: File::create(dir.join("0.log"))?,
                at: 0,
                segment: 0,
                spans: Vec::new(),
                open: None,
            },
            other => bail!("unknown store {other}"),
        })
    }

    fn put(&mut self, id: i64, body: &[u8]) -> Result<()> {
        match self {
            Self::Sqlite(conn, pending) => {
                conn.prepare_cached("INSERT INTO bodies (id, data) VALUES (?1, ?2)")?
                    .execute(params![id, body])?;
                *pending += body.len();
                if *pending > 64 << 20 {
                    conn.execute_batch("COMMIT; BEGIN")?;
                    *pending = 0;
                }
            }
            Self::Files(dir) => {
                File::create(dir.join(format!("{id}.body")))?.write_all(body)?;
            }
            Self::Log {
                dir,
                file,
                at,
                segment,
                spans,
                ..
            } => {
                if *at + body.len() as u64 > SEGMENT && *at > 0 {
                    *segment += 1;
                    *file = File::create(dir.join(format!("{segment}.log")))?;
                    *at = 0;
                }
                file.write_all(body)?;
                spans.push((*segment, *at, body.len() as u32));
                *at += body.len() as u64;
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        if let Self::Sqlite(conn, _) = self {
            conn.execute_batch("COMMIT; PRAGMA wal_checkpoint(TRUNCATE);")?;
        }
        Ok(())
    }

    fn open(dir: &Path, written: &Self) -> Result<Self> {
        Ok(match written {
            Self::Sqlite(..) => Self::Sqlite(Connection::open(dir.join("bodies.db"))?, 0),
            Self::Files(_) => Self::Files(dir.to_owned()),
            Self::Log { spans, .. } => Self::Log {
                dir: dir.to_owned(),
                file: File::open(dir.join("0.log"))?,
                at: 0,
                segment: 0,
                spans: spans.clone(),
                open: None,
            },
        })
    }

    fn get(&mut self, id: i64, buf: &mut Vec<u8>) -> Result<()> {
        buf.clear();
        match self {
            Self::Sqlite(conn, _) => {
                let data: Vec<u8> = conn
                    .prepare_cached("SELECT data FROM bodies WHERE id = ?1")?
                    .query_row([id], |r| r.get(0))?;
                *buf = data;
            }
            Self::Files(dir) => {
                File::open(dir.join(format!("{id}.body")))?.read_to_end(buf)?;
            }
            Self::Log {
                dir, spans, open, ..
            } => {
                let (segment, at, len) = spans[id as usize];
                if open.as_ref().is_none_or(|(s, _)| *s != segment) {
                    *open = Some((segment, File::open(dir.join(format!("{segment}.log")))?));
                }
                let (_, file) = open.as_mut().context("no segment open")?;
                file.seek(SeekFrom::Start(at))?;
                buf.resize(len as usize, 0);
                file.read_exact(buf)?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- scale

const META_COLUMNS: &str =
    "started_us INTEGER NOT NULL, client TEXT NOT NULL, method TEXT NOT NULL,
    host TEXT NOT NULL, path TEXT NOT NULL, status INTEGER, content_type TEXT,
    duration_us INTEGER, resp_bytes INTEGER NOT NULL";

/// Where a session's bodies go.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layout {
    /// Bodies in the row, between the columns, as the prototype has them.
    Inline,
    /// One flows database, one bodies database.
    Split,
    /// One flows database, and bodies in append-only segments that are deleted
    /// whole when the oldest flows go.
    Log,
}

struct Session {
    layout: Layout,
    dir: PathBuf,
    flows: Connection,
    bodies: Option<Connection>,
    file: Option<File>,
    segment: i64,
    at: u64,
    pending: usize,
    /// Segments open for reading.
    reading: Option<(i64, File)>,
}

impl Session {
    fn create(dir: &Path, layout: Layout) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let flows = Connection::open(dir.join("flows.db"))?;
        let extra = match layout {
            Layout::Inline => "req_body BLOB, resp_body BLOB, error TEXT",
            Layout::Split => "error TEXT",
            Layout::Log => "error TEXT, segment INTEGER, offset INTEGER, len INTEGER",
        };
        flows.execute_batch(&format!(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;
             CREATE TABLE flows (id INTEGER PRIMARY KEY, {META_COLUMNS}, {extra});
             CREATE INDEX flows_host ON flows (host);
             CREATE INDEX flows_status ON flows (status);
             BEGIN;"
        ))?;
        let bodies = if layout == Layout::Split {
            let conn = Connection::open(dir.join("bodies.db"))?;
            conn.execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;
                 CREATE TABLE bodies (id INTEGER PRIMARY KEY, data BLOB NOT NULL);
                 BEGIN;",
            )?;
            Some(conn)
        } else {
            None
        };
        let file = if layout == Layout::Log {
            Some(File::create(dir.join("0.log"))?)
        } else {
            None
        };
        Ok(Self {
            layout,
            dir: dir.to_owned(),
            flows,
            bodies,
            file,
            segment: 0,
            at: 0,
            pending: 0,
            reading: None,
        })
    }

    fn insert(&mut self, id: i64, host: &str, path: &str, status: i64, body: &[u8]) -> Result<()> {
        let started = 1_793_000_000_000_000_i64 + id * 3_000;
        let len = body.len() as i64;
        match self.layout {
            Layout::Inline => {
                self.flows.prepare_cached(
                    "INSERT INTO flows (id, started_us, client, method, host, path, status, content_type, duration_us, resp_bytes, resp_body)
                     VALUES (?1, ?2, '127.0.0.1:52344', 'GET', ?3, ?4, ?5, 'application/json', 1000, ?6, ?7)",
                )?
                .execute(params![id, started, host, path, status, len, body])?;
            }
            Layout::Split => {
                self.flows.prepare_cached(
                    "INSERT INTO flows (id, started_us, client, method, host, path, status, content_type, duration_us, resp_bytes)
                     VALUES (?1, ?2, '127.0.0.1:52344', 'GET', ?3, ?4, ?5, 'application/json', 1000, ?6)",
                )?
                .execute(params![id, started, host, path, status, len])?;
                self.bodies
                    .as_ref()
                    .context("no bodies database")?
                    .prepare_cached("INSERT INTO bodies (id, data) VALUES (?1, ?2)")?
                    .execute(params![id, body])?;
            }
            Layout::Log => {
                if self.at + body.len() as u64 > SEGMENT && self.at > 0 {
                    self.segment += 1;
                    self.file = Some(File::create(
                        self.dir.join(format!("{}.log", self.segment)),
                    )?);
                    self.at = 0;
                }
                self.file.as_mut().context("no segment")?.write_all(body)?;
                self.flows.prepare_cached(
                    "INSERT INTO flows (id, started_us, client, method, host, path, status, content_type, duration_us, resp_bytes, segment, offset, len)
                     VALUES (?1, ?2, '127.0.0.1:52344', 'GET', ?3, ?4, ?5, 'application/json', 1000, ?6, ?7, ?8, ?6)",
                )?
                .execute(params![id, started, host, path, status, len, self.segment, self.at as i64])?;
                self.at += body.len() as u64;
            }
        }
        self.pending += body.len();
        if self.pending > 64 << 20 {
            self.commit()?;
        }
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        self.flows.execute_batch("COMMIT; BEGIN")?;
        if let Some(bodies) = &self.bodies {
            bodies.execute_batch("COMMIT; BEGIN")?;
        }
        self.pending = 0;
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        self.flows
            .execute_batch("COMMIT; PRAGMA wal_checkpoint(TRUNCATE);")?;
        if let Some(bodies) = &self.bodies {
            bodies.execute_batch("COMMIT; PRAGMA wal_checkpoint(TRUNCATE);")?;
        }
        if let Some(file) = &self.file {
            file.sync_data()?;
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        self.flows.execute_batch("BEGIN")?;
        if let Some(bodies) = &self.bodies {
            bodies.execute_batch("BEGIN")?;
        }
        Ok(())
    }

    fn read_body(&mut self, id: i64, buf: &mut Vec<u8>) -> Result<()> {
        buf.clear();
        match self.layout {
            Layout::Inline => {
                *buf = self
                    .flows
                    .prepare_cached("SELECT resp_body FROM flows WHERE id = ?1")?
                    .query_row([id], |r| r.get(0))?;
            }
            Layout::Split => {
                *buf = self
                    .bodies
                    .as_ref()
                    .context("no bodies database")?
                    .prepare_cached("SELECT data FROM bodies WHERE id = ?1")?
                    .query_row([id], |r| r.get(0))?;
            }
            Layout::Log => {
                let (segment, offset, len): (i64, i64, i64) = self
                    .flows
                    .prepare_cached("SELECT segment, offset, len FROM flows WHERE id = ?1")?
                    .query_row([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
                if self.reading.as_ref().is_none_or(|(s, _)| *s != segment) {
                    self.reading = Some((
                        segment,
                        File::open(self.dir.join(format!("{segment}.log")))?,
                    ));
                }
                let (_, file) = self.reading.as_mut().context("no segment open")?;
                file.seek(SeekFrom::Start(offset as u64))?;
                buf.resize(len as usize, 0);
                file.read_exact(buf)?;
            }
        }
        Ok(())
    }

    fn disk(&self) -> u64 {
        dir_bytes(&self.dir)
    }

    /// Drops every flow up to `last` and their bodies, committing in steps as an
    /// app that stays responsive would.
    fn drop_oldest(&mut self, last: i64) -> Result<Duration> {
        let started = Instant::now();
        const STEP: i64 = 10_000;
        let mut from: i64 = 0;
        while from < last {
            let to = (from + STEP).min(last);
            self.flows.execute(
                "DELETE FROM flows WHERE id > ?1 AND id <= ?2",
                params![from, to],
            )?;
            if let Some(bodies) = &self.bodies {
                bodies.execute(
                    "DELETE FROM bodies WHERE id > ?1 AND id <= ?2",
                    params![from, to],
                )?;
            }
            self.commit()?;
            from = to;
        }
        if self.layout == Layout::Log {
            // A segment goes when no flow left points into it.
            let oldest: i64 = self
                .flows
                .query_row("SELECT min(segment) FROM flows", [], |r| r.get(0))?;
            self.reading = None;
            for segment in 0..oldest {
                let _ = fs::remove_file(self.dir.join(format!("{segment}.log")));
            }
        }
        Ok(started.elapsed())
    }
}

fn scale(dir: &Path, layout: &str, gb: u64) -> Result<()> {
    let layout = match layout {
        "inline" => Layout::Inline,
        "split" => Layout::Split,
        "log" => Layout::Log,
        other => bail!("unknown layout {other}"),
    };
    let work = dir.join(format!("scale-{layout:?}"));
    let _ = fs::remove_dir_all(&work);
    let mut session = Session::create(&work, layout)?;

    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    // A pool of bodies, since generating 50 GB of words would measure the
    // generator. Each is stamped with its flow, so no two are the same.
    let pool: Vec<Vec<u8>> = (0..4096)
        .map(|_| {
            let size = 1024 + (mixed_size(&mut rng) % (63 << 10));
            rich_text(&mut rng, size).into_bytes()
        })
        .collect();
    println!(
        "== {layout:?}, target {gb} GB, mean body {}",
        size_text(pool.iter().map(|b| b.len() as u64).sum::<u64>() / pool.len() as u64)
    );
    println!(
        "{:>7} {:>9} {:>9} {:>11} {:>17} {:>17} {:>17} {:>17}",
        "GB",
        "flows",
        "MB/s",
        "flows/s",
        "page cold p50/p99",
        "page warm p50/p99",
        "host p50/p99",
        "body cold p50/p99"
    );

    let step = 10u64 << 30;
    let target = gb << 30;
    let mut written = 0u64;
    let mut next_report = step.min(target);
    let mut id = 0i64;
    let mut stamped = Vec::new();
    let mut interval = Instant::now();
    let mut interval_bytes = 0u64;
    let mut interval_flows = 0u64;
    let mut paused_for = Duration::ZERO;
    while written < target {
        id += 1;
        let body = &pool[rng.below(pool.len() as u64) as usize];
        stamped.clear();
        stamped.extend_from_slice(body);
        let stamp = id.to_le_bytes();
        stamped[..8].copy_from_slice(&stamp);
        let host = format!("api{}.example.com", rng.below(300));
        let path = format!("/v1/items/{}?page={}", rng.below(9_000_000), rng.below(40));
        let status = if rng.below(100) == 0 { 500 } else { 200 };
        session.insert(id, &host, &path, status, &stamped)?;
        written += stamped.len() as u64;
        interval_bytes += stamped.len() as u64;
        interval_flows += 1;

        if written >= next_report {
            let took = interval.elapsed();
            let at = Instant::now();
            session.pause()?;
            let flush = at.elapsed();
            let rate = interval_bytes as f64 / 1e6 / (took + flush).as_secs_f64();
            let flows_rate = interval_flows as f64 / (took + flush).as_secs_f64();
            let row = read_probe(&mut session, id, &mut rng)?;
            println!(
                "{:>7.1} {id:>9} {rate:>9.0} {flows_rate:>11.0} {row}",
                session.disk() as f64 / (1u64 << 30) as f64,
            );
            session.resume()?;
            next_report += step;
            interval = Instant::now();
            interval_bytes = 0;
            interval_flows = 0;
            paused_for += at.elapsed();
        }
    }
    session.pause()?;
    let before = session.disk();
    session.resume()?;
    let last = id / 10;
    let took = session.drop_oldest(last)?;
    session.pause()?;
    println!(
        "dropped the oldest tenth ({last} flows): {:.2} s, disk {} → {}",
        took.as_secs_f64(),
        size_text(before),
        size_text(session.disk()),
    );
    let row = read_probe(&mut session, id, &mut rng)?;
    println!("reads after that: {row}");
    session.resume()?;

    // Writing on must use the freed room, or the limit means nothing.
    let started = Instant::now();
    let mut more = 0u64;
    while more < target / 10 {
        id += 1;
        let body = &pool[rng.below(pool.len() as u64) as usize];
        session.insert(id, "api1.example.com", "/more", 200, body)?;
        more += body.len() as u64;
    }
    session.pause()?;
    println!(
        "wrote a tenth more at {:.0} MB/s, disk now {}",
        more as f64 / 1e6 / started.elapsed().as_secs_f64(),
        size_text(session.disk()),
    );

    if layout != Layout::Log {
        let target_db = if layout == Layout::Inline {
            "flows.db"
        } else {
            "bodies.db"
        };
        let vacuum = Connection::open(work.join(target_db))?;
        let started = Instant::now();
        vacuum.execute_batch("VACUUM")?;
        println!(
            "VACUUM of {target_db}: {:.1} s, disk now {}",
            started.elapsed().as_secs_f64(),
            size_text(session.disk()),
        );
    }
    drop(session);
    fs::remove_dir_all(&work)?;
    Ok(())
}

fn read_probe(session: &mut Session, last: i64, rng: &mut Rng) -> Result<String> {
    let oldest: i64 =
        session
            .flows
            .query_row("SELECT coalesce(min(id), 1) FROM flows", [], |r| r.get(0))?;
    let span = (last - oldest).max(1) as u64;

    forget_dir(&session.dir);
    let page = |session: &Session, from: i64| -> Result<u64> {
        let mut stmt = session.flows.prepare_cached(
            "SELECT id, started_us, method, host, path, status, content_type, duration_us, resp_bytes, error
             FROM flows WHERE id >= ?1 ORDER BY id LIMIT 200",
        )?;
        let mut rows = stmt.query([from])?;
        let mut seen = 0;
        while rows.next()?.is_some() {
            seen += 1;
        }
        Ok(seen)
    };

    let mut cold = Vec::new();
    for _ in 0..100 {
        let from = oldest + rng.below(span) as i64;
        let started = Instant::now();
        page(session, from)?;
        cold.push(started.elapsed());
    }
    let mut warm = Vec::new();
    for _ in 0..200 {
        let from = oldest + rng.below(span.min(100_000)) as i64;
        page(session, from)?;
        let started = Instant::now();
        page(session, from)?;
        warm.push(started.elapsed());
    }
    let mut host = Vec::new();
    for i in 0..100 {
        let started = Instant::now();
        let mut stmt = session
            .flows
            .prepare_cached("SELECT id FROM flows WHERE host = ?1 ORDER BY id DESC LIMIT 200")?;
        let mut rows = stmt.query([format!("api{}.example.com", i % 300)])?;
        while rows.next()?.is_some() {}
        host.push(started.elapsed());
    }
    forget_dir(&session.dir);
    let mut bodies = Vec::new();
    let mut buf = Vec::new();
    for _ in 0..200 {
        let id = oldest + rng.below(span) as i64;
        let started = Instant::now();
        session.read_body(id, &mut buf)?;
        bodies.push(started.elapsed());
    }
    let cell = |samples: Vec<Duration>| {
        let (p50, p99) = percentiles(samples);
        format!("{p50:>8.2}/{p99:<8.2}")
    };
    Ok(format!(
        "{} {} {} {}",
        cell(cold),
        cell(warm),
        cell(host),
        cell(bodies)
    ))
}

fn size_text(bytes: u64) -> String {
    const MB: f64 = 1024. * 1024.;
    let b = bytes as f64;
    if b >= MB * 1024. {
        format!("{:.1} GB", b / MB / 1024.)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= 1024. {
        format!("{:.0} KB", b / 1024.)
    } else {
        format!("{bytes} B")
    }
}

// ---------------------------------------------------------------- indextrim

const TRIM_QUERIES: [(&str, &str); 3] = [
    ("common", "\"thanh toan\""),
    ("rare", "\"ma giam gia\""),
    ("none", "zzznosuch"),
];

fn index_db(path: &Path, deletable: bool) -> Result<Connection> {
    let conn = Connection::open(path)?;
    let option = if deletable {
        ", contentless_delete = 1"
    } else {
        ""
    };
    conn.execute_batch(&format!(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE VIRTUAL TABLE IF NOT EXISTS bodies USING fts5(
             body, tokenize = 'unicode61 remove_diacritics 2', content = ''{option}
         );"
    ))?;
    Ok(conn)
}

fn fill_index(conn: &mut Connection, rng: &mut Rng, ids: std::ops::Range<u64>) -> Result<()> {
    let ids: Vec<u64> = ids.collect();
    for chunk in ids.chunks(2_000) {
        let tx = conn.transaction()?;
        {
            let mut insert =
                tx.prepare_cached("INSERT INTO bodies (rowid, body) VALUES (?1, ?2)")?;
            for id in chunk {
                let target = 500 + rng.below(3000) as usize;
                let text = tracewing_store::search::normalize(&rich_text(rng, target));
                insert.execute(params![*id as i64, text])?;
            }
        }
        tx.commit()?;
    }
    Ok(())
}

fn files_mb(paths: &[PathBuf]) -> f64 {
    let mut bytes = 0;
    for path in paths {
        for suffix in ["", "-wal"] {
            let mut name = path.clone().into_os_string();
            name.push(suffix);
            bytes += fs::metadata(&name).map_or(0, |m| m.len());
        }
    }
    bytes as f64 / 1e6
}

/// Milliseconds to find every match: what a filter costs on the first keystroke.
fn trim_query_ms(conns: &[Connection], first_live: u64) -> Result<Vec<(f64, u64)>> {
    let mut out = Vec::new();
    for (_, query) in TRIM_QUERIES {
        let mut best = f64::MAX;
        let mut found = 0;
        for _ in 0..5 {
            let started = Instant::now();
            found = 0u64;
            for conn in conns {
                let mut stmt = conn.prepare_cached(
                    "SELECT rowid FROM bodies WHERE bodies MATCH ?1 AND rowid >= ?2 ORDER BY rowid DESC",
                )?;
                let mut rows = stmt.query(params![query, first_live as i64])?;
                while rows.next()?.is_some() {
                    found += 1;
                }
            }
            best = best.min(started.elapsed().as_secs_f64() * 1000.0);
        }
        out.push((best, found));
    }
    Ok(out)
}

fn print_queries(label: &str, results: &[(f64, u64)]) {
    print!("{label:<24}");
    for ((name, _), (ms, found)) in TRIM_QUERIES.iter().zip(results) {
        print!("  {name} {ms:.1} ms ({found})");
    }
    println!();
}

fn indextrim(dir: &Path, rows: u64) -> Result<()> {
    let tenth = rows / 10;
    let work = dir.join("indextrim");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work)?;

    // One database, rows taken out one at a time.
    println!("== one database, contentless_delete");
    let path = work.join("one.db");
    let paths = [path.clone()];
    let mut conn = index_db(&path, true)?;
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let started = Instant::now();
    fill_index(&mut conn, &mut rng, 1..rows + 1)?;
    println!(
        "built {rows} rows in {:.1} s, {:.0} MB",
        started.elapsed().as_secs_f64(),
        files_mb(&paths)
    );
    print_queries("before", &trim_query_ms(std::slice::from_ref(&conn), 1)?);
    let mut first = 1;
    for round in 0..3 {
        let started = Instant::now();
        let ids: Vec<u64> = (first..first + tenth).collect();
        let mut slowest = Duration::ZERO;
        for chunk in ids.chunks(5_000) {
            let chunk_started = Instant::now();
            let tx = conn.transaction()?;
            {
                let mut delete = tx.prepare_cached("DELETE FROM bodies WHERE rowid = ?1")?;
                for id in chunk {
                    delete.execute([*id as i64])?;
                }
            }
            tx.commit()?;
            slowest = slowest.max(chunk_started.elapsed());
        }
        let took = started.elapsed();
        first += tenth;
        let after_delete = files_mb(&paths);
        fill_index(
            &mut conn,
            &mut rng,
            rows + round * tenth + 1..rows + (round + 1) * tenth + 1,
        )?;
        println!(
            "round {round}: deleted {tenth} in {:.2} s (slowest 5,000: {:.0} ms), {:.0} MB; refilled, {:.0} MB",
            took.as_secs_f64(),
            slowest.as_secs_f64() * 1000.0,
            after_delete,
            files_mb(&paths)
        );
    }
    print_queries(
        "after 3 rounds",
        &trim_query_ms(std::slice::from_ref(&conn), 1)?,
    );
    drop(conn);

    // Ten databases, the oldest deleted whole.
    println!("== ten databases, oldest deleted whole");
    let shard = |n: u64| work.join(format!("shard-{n:04}.db"));
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let started = Instant::now();
    let mut conns = Vec::new();
    for n in 0..10 {
        let mut conn = index_db(&shard(n), false)?;
        fill_index(&mut conn, &mut rng, n * tenth + 1..(n + 1) * tenth + 1)?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        conns.push(conn);
    }
    let paths: Vec<PathBuf> = (0..10).map(shard).collect();
    println!(
        "built {rows} rows in {:.1} s, {:.0} MB",
        started.elapsed().as_secs_f64(),
        files_mb(&paths)
    );
    print_queries("before", &trim_query_ms(&conns, 1)?);
    for round in 0..3 {
        let started = Instant::now();
        drop(conns.remove(0));
        for suffix in ["", "-wal", "-shm"] {
            let mut name = shard(round).into_os_string();
            name.push(suffix);
            let _ = fs::remove_file(name);
        }
        let took = started.elapsed();
        let live: Vec<PathBuf> = (round + 1..11 + round).map(shard).collect();
        let after_delete = files_mb(&live);
        let n = 10 + round;
        let mut conn = index_db(&shard(n), false)?;
        fill_index(&mut conn, &mut rng, n * tenth + 1..(n + 1) * tenth + 1)?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        conns.push(conn);
        println!(
            "round {round}: deleted the file in {:.2} ms, {:.0} MB; refilled, {:.0} MB",
            took.as_secs_f64() * 1000.0,
            after_delete,
            files_mb(&live)
        );
    }
    print_queries("after 3 rounds", &trim_query_ms(&conns, 1)?);
    drop(conns);
    fs::remove_dir_all(&work)?;
    Ok(())
}
