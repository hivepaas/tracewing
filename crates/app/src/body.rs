//! A response body on disk, split into display lines by a background thread.
//!
//! The body is mapped, not read: it stays in the page cache, where the OS can
//! drop it, and never in the heap. Minified JSON is the hard case, since one
//! line of the file may be many millions of display lines, so [`Lines`] breaks
//! it where a formatter would without building any tree.
//!
//! The heap holds one mark per [`MARK_EVERY`] lines, enough to start [`Lines`]
//! again near any line: a line needs no more than its start and its depth. One
//! `u64` per line came to 447 MB for a 500 MB API answer and 2 GB for 500 MB of
//! single digits.
//!
//! The marks are written by one thread and read by the window's thread without
//! a lock, so a scan can never make a frame wait.

use anyhow::{Context, Result};
use memchr::{memchr, memchr2};
use memmap2::Mmap;
use std::{
    fs::File,
    ops::Range,
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// A mark every this many lines, which also bounds how many lines a read skips.
pub const MARK_EVERY: u64 = 256;
const CHUNK_BITS: u64 = 12;
const CHUNK_LEN: u64 = 1 << CHUNK_BITS;
/// With `CHUNK_LEN` and `MARK_EVERY`, room for 4 billion lines; 32 KiB up front.
const CHUNKS: usize = 4 * 1024;
const DEPTH_BITS: u64 = 16;
/// A file without line breaks is cut into lines of this many bytes.
pub const TEXT_LINE_MAX: usize = 4096;
/// How much of a line is ever turned into text for the window.
pub const SHOWN_BYTES_MAX: usize = 512;
/// How often the scanner says how far it has got.
const PROGRESS_EVERY: usize = 1 << 20;
/// How much of a body decides whether it is text.
const SNIFF_BYTES: usize = 8 * 1024;
const HEX_LINE: usize = 16;

/// A `u64` per [`MARK_EVERY`] lines, `offset << 16 | depth`, and how many lines
/// there are: one writer, any number of readers.
pub struct LineIndex {
    chunks: Box<[OnceLock<Box<[AtomicU64]>>]>,
    len: AtomicU64,
}

impl LineIndex {
    fn new() -> Self {
        Self {
            chunks: (0..CHUNKS).map(|_| OnceLock::new()).collect(),
            len: AtomicU64::new(0),
        }
    }

    pub fn len(&self) -> u64 {
        self.len.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// False when the marks are full, which takes over 4 billion lines.
    fn push(&self, offset: u64, depth: u32) -> bool {
        let line = self.len.load(Ordering::Relaxed);
        if line.is_multiple_of(MARK_EVERY) {
            let mark = line / MARK_EVERY;
            let Some(slot) = self.chunks.get((mark >> CHUNK_BITS) as usize) else {
                return false;
            };
            let chunk = slot.get_or_init(|| (0..CHUNK_LEN).map(|_| AtomicU64::new(0)).collect());
            let entry = offset << DEPTH_BITS | u64::from(depth.min(u16::MAX as u32));
            chunk[(mark & (CHUNK_LEN - 1)) as usize].store(entry, Ordering::Relaxed);
        }
        // Readers only look below `len`, so this publishes the mark above.
        self.len.store(line + 1, Ordering::Release);
        true
    }

    /// The nearest mark at or before `line`.
    fn mark_before(&self, line: u64) -> (u64, u32) {
        let mark = line / MARK_EVERY;
        let chunk = self.chunks[(mark >> CHUNK_BITS) as usize]
            .get()
            .expect("asked for a line that was not written");
        let entry = chunk[(mark & (CHUNK_LEN - 1)) as usize].load(Ordering::Relaxed);
        (
            entry >> DEPTH_BITS,
            (entry & ((1 << DEPTH_BITS) - 1)) as u32,
        )
    }

    /// The heap this index holds.
    pub fn heap_bytes(&self) -> u64 {
        let marks = self.len().div_ceil(MARK_EVERY);
        let chunks = marks.div_ceil(CHUNK_LEN);
        chunks * CHUNK_LEN * 8 + (CHUNKS * size_of::<OnceLock<Box<[AtomicU64]>>>()) as u64
    }
}

/// Finds display lines, from the start of a body or from any line's start.
pub struct Lines<'a> {
    data: &'a [u8],
    kind: Kind,
    at: usize,
    depth: u32,
    line_start: usize,
    /// The line to hand out next, if it is already known.
    pending: Option<(usize, u32)>,
}

impl<'a> Lines<'a> {
    fn new(data: &'a [u8], kind: Kind) -> Self {
        let at = if kind == Kind::Json {
            next_token(data, 0)
        } else {
            0
        };
        Self::resume(data, kind, at, 0)
    }

    fn resume(data: &'a [u8], kind: Kind, at: usize, depth: u32) -> Self {
        // A JSON line that starts with a closer was counted when it was found.
        let closer = kind == Kind::Json && matches!(data.get(at), Some(b'}' | b']'));
        Self {
            data,
            kind,
            at: if closer { at + 1 } else { at },
            depth,
            line_start: at,
            pending: (at < data.len()).then_some((at, depth)),
        }
    }

    /// Where it has got to, for the progress bar.
    fn position(&self) -> usize {
        self.at
    }

    fn begin_line(&mut self, at: usize) {
        self.line_start = at;
        self.pending = Some((at, self.depth));
    }

    /// Breaks after `{`, `[` and `,`, and before `}` and `]`. An empty `{}` or
    /// `[]` stays on its line. It checks nothing else, so that a body that is
    /// broken, cut short or not quite JSON still shows as far as it goes.
    fn advance_json(&mut self) {
        let data = self.data;
        while self.pending.is_none() && self.at < data.len() {
            let i = self.at;
            match data[i] {
                b'"' => self.at = skip_string(data, i),
                open @ (b'{' | b'[') => {
                    let close = if open == b'{' { b'}' } else { b']' };
                    let after = next_token(data, i + 1);
                    if data.get(after) == Some(&close) {
                        self.at = after + 1;
                    } else {
                        self.depth += 1;
                        self.at = after;
                        // A closer of the wrong kind begins its own line below.
                        if after < data.len() && !matches!(data[after], b'}' | b']') {
                            self.begin_line(after);
                        }
                    }
                }
                b'}' | b']' => {
                    self.depth = self.depth.saturating_sub(1);
                    self.at = i + 1;
                    self.begin_line(i);
                }
                b',' => {
                    let after = next_token(data, i + 1);
                    self.at = after;
                    // A closer after a stray comma begins its own line anyway.
                    if after < data.len() && !matches!(data[after], b'}' | b']') {
                        self.begin_line(after);
                    }
                }
                _ => self.at = i + 1,
            }
        }
    }

    fn advance_text(&mut self) {
        let data = self.data;
        let i = self.at;
        let window = &data[i..data.len().min(i + TEXT_LINE_MAX)];
        self.at += match memchr(b'\n', window) {
            Some(found) => found + 1,
            None if i + window.len() == data.len() => window.len(),
            None => cut_at_char(window, TEXT_LINE_MAX).len().max(1),
        };
        if self.at < data.len() {
            self.begin_line(self.at);
        }
    }
}

impl Iterator for Lines<'_> {
    /// Where a line starts, and how deep.
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.pending.take()?;
        match self.kind {
            Kind::Json => self.advance_json(),
            Kind::Text => self.advance_text(),
            Kind::Binary => {}
        }
        Some(line)
    }
}

fn next_token(data: &[u8], mut at: usize) -> usize {
    while at < data.len() && data[at].is_ascii_whitespace() {
        at += 1;
    }
    at
}

/// From the opening quote to just past the closing one. Long strings are where
/// a byte-at-a-time loop would hurt.
fn skip_string(data: &[u8], open: usize) -> usize {
    let mut i = open + 1;
    while i < data.len() {
        match memchr2(b'"', b'\\', &data[i..]) {
            Some(found) if data[i + found] == b'"' => return i + found + 1,
            Some(found) => i += found + 2,
            None => return data.len(),
        }
    }
    data.len()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Json,
    Text,
    /// Shown as a hex dump, whose lines need no index.
    Binary,
}

/// What the scanner has done so far.
#[derive(Default)]
struct Progress {
    scanned: AtomicU64,
    done: AtomicBool,
    took_us: AtomicU64,
    stop: AtomicBool,
}

pub struct Body {
    map: Mmap,
    kind: Kind,
    index: LineIndex,
    progress: Progress,
}

/// One line, ready to draw.
pub struct Line {
    pub text: String,
    pub depth: u32,
    /// Bytes of this line left out of `text`.
    pub cut: u64,
}

impl Body {
    /// Maps the file and starts the scan. Returns before the scan has done
    /// anything, so it takes the same time for any size of file.
    pub fn open(path: &Path) -> Result<Arc<Self>> {
        let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        // SAFETY: bodies are spooled once and never change while they are shown.
        let map = unsafe { Mmap::map(&file) }.with_context(|| format!("map {}", path.display()))?;
        let kind = sniff(&map);
        let body = Arc::new(Self {
            map,
            kind,
            index: LineIndex::new(),
            progress: Progress::default(),
        });
        let scanner = Arc::clone(&body);
        std::thread::Builder::new()
            .name("body-scan".into())
            .spawn(move || scanner.scan())
            .context("start the scan")?;
        Ok(body)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn size(&self) -> u64 {
        self.map.len() as u64
    }

    pub fn scanned(&self) -> u64 {
        self.progress.scanned.load(Ordering::Relaxed)
    }

    pub fn is_scanned(&self) -> bool {
        self.progress.done.load(Ordering::Acquire)
    }

    pub fn scan_time(&self) -> Duration {
        Duration::from_micros(self.progress.took_us.load(Ordering::Relaxed))
    }

    pub fn index_bytes(&self) -> u64 {
        self.index.heap_bytes()
    }

    /// Stops the scan, for a window that closes before it ends.
    pub fn stop(&self) {
        self.progress.stop.store(true, Ordering::Relaxed);
    }

    /// Lines ready to show. The line being scanned has no end yet, so it waits.
    pub fn lines(&self) -> u64 {
        if self.kind == Kind::Binary {
            return self.size().div_ceil(HEX_LINE as u64);
        }
        let done = self.is_scanned();
        let len = self.index.len();
        if done { len } else { len.saturating_sub(1) }
    }

    /// The lines in `range`, which must be ready (below [`Self::lines`]).
    pub fn read(&self, range: Range<u64>) -> Vec<Line> {
        if self.kind == Kind::Binary {
            return range.map(|line| self.hex_line(line)).collect();
        }
        if range.is_empty() {
            return Vec::new();
        }
        let (offset, depth) = self.index.mark_before(range.start);
        let skip = range.start % MARK_EVERY;
        let wanted = (range.end - range.start) as usize;
        let mut lines = Lines::resume(&self.map, self.kind, offset as usize, depth)
            .skip(skip as usize)
            .peekable();
        let mut out = Vec::with_capacity(wanted);
        for _ in 0..wanted {
            let Some((start, depth)) = lines.next() else {
                break;
            };
            let end = lines.peek().map_or(self.map.len(), |(next, _)| *next);
            out.push(self.shown(start, end, depth));
        }
        out
    }

    fn shown(&self, start: usize, end: usize, depth: u32) -> Line {
        let whole = &self.map[start..end];
        let shown = cut_at_char(whole, SHOWN_BYTES_MAX);
        // JSON lines end in the whitespace before the next token. Files with
        // their own line breaks may also have them between tokens of a line.
        let text = String::from_utf8_lossy(shown).replace(['\n', '\r', '\t'], " ");
        Line {
            text: text.trim_end().to_owned(),
            depth,
            cut: (whole.len() - shown.len()) as u64,
        }
    }

    fn hex_line(&self, line: u64) -> Line {
        use std::fmt::Write;
        let start = line as usize * HEX_LINE;
        let bytes = &self.map[start..self.map.len().min(start + HEX_LINE)];
        let mut text = String::with_capacity(4 * HEX_LINE + 2);
        for at in 0..HEX_LINE {
            match bytes.get(at) {
                Some(b) => write!(text, "{b:02x} ").expect("a String takes any text"),
                None => text.push_str("   "),
            }
        }
        text.push(' ');
        text.extend(bytes.iter().map(|b| {
            if b.is_ascii_graphic() {
                *b as char
            } else {
                '.'
            }
        }));
        Line {
            text,
            depth: 0,
            cut: 0,
        }
    }

    fn scan(&self) {
        let started = Instant::now();
        if self.kind != Kind::Binary {
            self.scan_lines();
        }
        self.progress.scanned.store(self.size(), Ordering::Relaxed);
        self.progress
            .took_us
            .store(started.elapsed().as_micros() as u64, Ordering::Relaxed);
        self.progress.done.store(true, Ordering::Release);
    }

    fn scan_lines(&self) {
        let mut lines = Lines::new(&self.map, self.kind);
        let mut next_report = PROGRESS_EVERY;
        while let Some((start, depth)) = lines.next() {
            if !self.index.push(start as u64, depth) {
                return;
            }
            if lines.position() >= next_report {
                self.progress
                    .scanned
                    .store(lines.position() as u64, Ordering::Relaxed);
                if self.progress.stop.load(Ordering::Relaxed) {
                    return;
                }
                next_report = lines.position() + PROGRESS_EVERY;
            }
        }
    }
}

fn sniff(data: &[u8]) -> Kind {
    let head = &data[..data.len().min(SNIFF_BYTES)];
    let is_text = !head.contains(&0)
        && match std::str::from_utf8(head) {
            Ok(_) => true,
            // A character cut by the end of the sample is still text.
            Err(e) => e.error_len().is_none(),
        };
    if !is_text {
        return Kind::Binary;
    }
    match head.iter().find(|b| !b.is_ascii_whitespace()) {
        Some(b'{' | b'[') => Kind::Json,
        _ => Kind::Text,
    }
}

/// At most `max` bytes, ending on a character boundary. A byte that starts no
/// character (binary data) does not hold it back by more than three bytes.
fn cut_at_char(bytes: &[u8], max: usize) -> &[u8] {
    if bytes.len() <= max {
        return bytes;
    }
    let mut end = max;
    while end > max.saturating_sub(3) && bytes[end] & 0xc0 == 0x80 {
        end -= 1;
    }
    &bytes[..end]
}

/// What to color a run of a shown line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Key,
    Str,
    Number,
    Literal,
    Punct,
}

/// The colored runs of a JSON line. It sees only what is shown, so a cut string
/// runs to the end. Works on text, not bytes, so ranges land on characters.
pub fn tokens(text: &str) -> Vec<(std::ops::Range<usize>, Token)> {
    let bytes = text.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i = (i + 1).min(bytes.len());
                let mut next = i;
                while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                    next += 1;
                }
                let kind = if bytes.get(next) == Some(&b':') {
                    Token::Key
                } else {
                    Token::Str
                };
                runs.push((start..i, kind));
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                i += 1;
                runs.push((start..i, Token::Punct));
            }
            b if b.is_ascii_whitespace() => i += 1,
            _ => {
                while i < bytes.len()
                    && !matches!(bytes[i], b',' | b'}' | b']' | b':' | b'"')
                    && !bytes[i].is_ascii_whitespace()
                {
                    i += 1;
                }
                let kind = if matches!(bytes[start], b'-' | b'0'..=b'9') {
                    Token::Number
                } else {
                    Token::Literal
                };
                runs.push((start..i, kind));
            }
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn lines_of(contents: &[u8]) -> Vec<(u32, String)> {
        let path = std::env::temp_dir().join(format!(
            "body-{}-{:x}",
            std::process::id(),
            contents.as_ptr() as usize
        ));
        File::create(&path).unwrap().write_all(contents).unwrap();
        let body = Body::open(&path).unwrap();
        while !body.is_scanned() {
            std::thread::yield_now();
        }
        let out = body
            .read(0..body.lines())
            .into_iter()
            .map(|line| (line.depth, line.text))
            .collect();
        std::fs::remove_file(&path).unwrap();
        out
    }

    fn plain(lines: &[(u32, &str)]) -> Vec<(u32, String)> {
        lines.iter().map(|(d, t)| (*d, (*t).to_owned())).collect()
    }

    #[test]
    fn minified_json_lays_out_like_a_formatter() {
        assert_eq!(
            lines_of(br#"{"a":1,"b":[true,{"c":"x,y{"}],"d":{},"e":[]}"#),
            plain(&[
                (0, "{"),
                (1, r#""a":1,"#),
                (1, r#""b":["#),
                (2, "true,"),
                (2, "{"),
                (3, r#""c":"x,y{""#),
                (2, "}"),
                (1, "],"),
                (1, r#""d":{},"#),
                (1, r#""e":[]"#),
                (0, "}"),
            ])
        );
    }

    #[test]
    fn formatted_json_keeps_the_same_lines() {
        let formatted = b"{\n  \"a\": 1,\n  \"b\": [\n    2\n  ]\n}\n";
        assert_eq!(
            lines_of(formatted),
            plain(&[
                (0, "{"),
                (1, r#""a": 1,"#),
                (1, r#""b": ["#),
                (2, "2"),
                (1, "]"),
                (0, "}"),
            ])
        );
    }

    #[test]
    fn escapes_do_not_end_a_string() {
        assert_eq!(
            lines_of(br#"["a\",b\\",1]"#),
            plain(&[(0, "["), (1, r#""a\",b\\","#), (1, "1"), (0, "]")])
        );
    }

    #[test]
    fn broken_json_shows_as_far_as_it_goes() {
        assert_eq!(
            lines_of(br#"{"a":[1,"unterminated"#),
            plain(&[
                (0, "{"),
                (1, r#""a":["#),
                (2, "1,"),
                (2, r#""unterminated"#),
            ])
        );
        assert_eq!(lines_of(b"[\"x\\"), plain(&[(0, "["), (1, "\"x\\")]));
    }

    #[test]
    fn closers_past_the_top_do_no_harm() {
        assert_eq!(lines_of(b"[]]]")[0], (0, "[]".to_owned()));
    }

    #[test]
    fn text_splits_on_line_breaks_and_on_length() {
        assert_eq!(lines_of(b"one\ntwo\n"), plain(&[(0, "one"), (0, "two")]));
        let long = vec![b'x'; TEXT_LINE_MAX * 2 + 10];
        assert_eq!(lines_of(&long).len(), 3);
    }

    #[test]
    fn long_lines_are_cut_on_a_character() {
        let mut body = b"\"".to_vec();
        body.extend("ệ".repeat(SHOWN_BYTES_MAX).as_bytes());
        body.push(b'"');
        let mut wrapped = b"[".to_vec();
        wrapped.extend(&body);
        wrapped.push(b']');
        let text = &lines_of(&wrapped)[1].1;
        assert!(text.len() <= SHOWN_BYTES_MAX);
        assert!(text.ends_with('ệ'));
    }

    /// Reading from a mark must find what the scan found, or a line would
    /// change as it scrolls into view.
    #[test]
    fn any_line_reads_the_same_from_its_mark() {
        const PIECES: &[&str] = &[
            "{",
            "}",
            "[",
            "]",
            ",",
            ":",
            "\"a\"",
            "\"b\\\"c\"",
            "1",
            "null",
            " ",
            "\n",
            "{}",
            "[]",
            "\"ệ\"",
            ",]",
            ",}",
        ];
        let mut rng = 0x9e37_79b9_7f4a_7c15u64;
        for run in 0..40 {
            let mut json = String::from("[");
            for _ in 0..4000 {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                json.push_str(PIECES[(rng % PIECES.len() as u64) as usize]);
            }
            let path = std::env::temp_dir().join(format!("marks-{}-{run}", std::process::id()));
            std::fs::write(&path, &json).unwrap();
            let body = Body::open(&path).unwrap();
            while !body.is_scanned() {
                std::thread::yield_now();
            }
            let starts: Vec<_> = Lines::new(json.as_bytes(), Kind::Json).collect();
            assert_eq!(body.lines(), starts.len() as u64);
            let all = body.read(0..body.lines());
            assert!(
                starts.len() > MARK_EVERY as usize,
                "too few lines to test marks"
            );
            for line in (0..body.lines()).step_by(7) {
                let one = body.read(line..line + 1);
                assert_eq!(
                    one[0].text, all[line as usize].text,
                    "run {run} line {line}"
                );
                assert_eq!(
                    one[0].depth, starts[line as usize].1,
                    "run {run} line {line}"
                );
            }
            std::fs::remove_file(&path).unwrap();
        }
    }

    #[test]
    fn index_stays_small() {
        let path = std::env::temp_dir().join(format!("small-{}", std::process::id()));
        let digits = "0,".repeat(1 << 20);
        std::fs::write(&path, format!("[{digits}0]")).unwrap();
        let body = Body::open(&path).unwrap();
        while !body.is_scanned() {
            std::thread::yield_now();
        }
        assert!(body.lines() > 1 << 20);
        assert!(body.index_bytes() < 1 << 18, "{} bytes", body.index_bytes());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn binary_is_a_hex_dump() {
        let lines = lines_of(&[0, b'A', 0xff]);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0].1.split_whitespace().collect::<Vec<_>>(),
            ["00", "41", "ff", ".A."]
        );
        assert_eq!(lines_of(&[0; 17]).len(), 2);
    }

    #[test]
    fn a_character_cut_by_the_sample_is_still_text() {
        let mut text = "a".repeat(SNIFF_BYTES - 1).into_bytes();
        text.extend("ệ".as_bytes());
        assert_eq!(sniff(&text), Kind::Text);
        assert_eq!(sniff(&[0xff, 0xfe, b'x']), Kind::Binary);
    }

    #[test]
    fn tokens_color_keys_apart_from_strings() {
        let kinds = |text: &str| tokens(text).into_iter().map(|(_, k)| k).collect::<Vec<_>>();
        assert_eq!(
            kinds(r#""a" : "b","#),
            [Token::Key, Token::Punct, Token::Str, Token::Punct]
        );
        assert_eq!(kinds("-1.5e3,"), [Token::Number, Token::Punct]);
        assert_eq!(kinds("null"), [Token::Literal]);
        assert_eq!(kinds(r#""cut off"#), [Token::Str]);
    }

    #[test]
    fn a_cut_escape_stays_inside_the_text() {
        assert_eq!(tokens(r#""a\"#)[0].0, 0..3);
    }
}
