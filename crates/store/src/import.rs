//! Sessions from other tools, each read into a session of its own: a HAR, a
//! Fiddler `.saz`, or a Charles JSON session (`.chlsj`). Which one a file is
//! shows in its first bytes, so its name does not matter.

use crate::{
    app::{App, AppAnswer},
    charles,
    decode::{self, Encoding},
    har, saz,
    search::Indexer,
    session::{self, Shared},
    settings::StorageSettings,
    store::{BodyRecord, FlowRecord, Recorder},
};
use anyhow::{Context, Result, bail};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
};
use time::{OffsetDateTime, PrimitiveDateTime, format_description::well_known::Iso8601};

/// What an import made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Imported {
    pub flows: u64,
    /// Entries that could not be read as a flow, such as one with no URL.
    pub skipped: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Har,
    Saz,
    Charles,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Self::Har => "HAR",
            Self::Saz => "Fiddler session",
            Self::Charles => "Charles session",
        }
    }

    /// From the first bytes of a file.
    pub fn sniff(head: &[u8]) -> Result<Self> {
        let head = head.strip_prefix(UTF8_BOM).unwrap_or(head);
        if head.starts_with(b"PK\x03\x04") {
            return Ok(Self::Saz);
        }
        // How Java writes an object, as Charles keeps its own sessions.
        if head.starts_with(&[0xAC, 0xED]) {
            bail!(
                "this is a Charles session in its own format. In Charles, choose File, \
                 Export Session, and save it as a JSON Session File (.chlsj) or a HAR"
            );
        }
        match head.iter().find(|b| !b.is_ascii_whitespace()) {
            Some(b'{') => Ok(Self::Har),
            Some(b'[') => Ok(Self::Charles),
            _ => bail!("this is not a HAR, a Fiddler .saz, or a Charles .chlsj file"),
        }
    }
}

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// Reads the file at `path`, whichever it is, into `session`, a directory that
/// must not hold one yet, and names the session `name`.
pub fn import_file(
    path: &Path,
    session: &Path,
    name: &str,
    settings: &StorageSettings,
) -> Result<(Format, Imported)> {
    let mut file =
        File::open(path).with_context(|| format!("{} could not be opened", path.display()))?;
    let mut head = Vec::with_capacity(64);
    (&mut file).take(64).read_to_end(&mut head)?;
    let format = Format::sniff(&head)?;
    file.rewind()?;
    if head.starts_with(UTF8_BOM) {
        file.seek(std::io::SeekFrom::Start(UTF8_BOM.len() as u64))?;
    }
    let imported = match format {
        Format::Har => har::import(file, session, name, settings),
        Format::Saz => saz::import(file, session, name, settings),
        Format::Charles => charles::import(file, session, name, settings),
    }?;
    Ok((format, imported))
}

/// Writes the flows read, in the order they started, into a new session. A
/// `None` is an entry that could not be read.
pub(crate) fn write(
    records: Vec<Option<FlowRecord>>,
    session: &Path,
    name: &str,
    settings: &StorageSettings,
) -> Result<Imported> {
    let skipped = records.iter().filter(|r| r.is_none()).count() as u64;
    let mut records: Vec<FlowRecord> = records.into_iter().flatten().collect();
    records.sort_by_key(|r| r.started_us);
    let flows = records.len() as u64;
    if flows == 0 {
        if skipped > 0 {
            bail!("none of its {skipped} entries could be read");
        }
        bail!("it has no flows in it");
    }

    session::create(session)?;
    let shared = Arc::new(Shared::default());
    let (recorder, writer) = Recorder::spawn(session, settings, shared, None, 1024)?;
    for record in records {
        recorder.submit_wait(record);
    }
    drop(recorder);
    writer.close()?;
    session::set_name(session, name)?;
    if settings.index_urls || settings.index_bodies {
        Indexer::catch_up(session, settings)?.finish()?;
    }
    Ok(Imported { flows, skipped })
}

/// Numbers connections from 1 in the order they are first seen.
#[derive(Default)]
pub(crate) struct Connections(HashMap<String, (i64, bool)>);

impl Connections {
    /// The connection `key` names, and whether this is the first flow over it,
    /// which is the one to tell what the connection was.
    pub fn over(&mut self, key: Option<String>) -> Option<(i64, bool)> {
        let key = key.filter(|k| !k.is_empty())?;
        let next = self.0.len() as i64 + 1;
        let (id, told) = self.0.entry(key).or_insert((next, false));
        let first = !std::mem::replace(told, true);
        Some((*id, first))
    }
}

/// A body as a file has it, all of it.
pub(crate) fn body(bytes: Vec<u8>) -> BodyRecord {
    BodyRecord {
        bytes: bytes.len() as u64,
        captured: bytes,
        complete: true,
        ..BodyRecord::default()
    }
}

/// The encoding to keep a body under. Tools often save a body unpacked and its
/// `Content-Encoding` as it was, so one that does not unpack is kept as it is.
pub(crate) fn encoding_kept(body: &[u8], header: Option<&str>) -> Option<String> {
    let header = header?.trim();
    match Encoding::from_header(Some(header)) {
        Encoding::Identity => None,
        Encoding::Unknown => Some(header.to_owned()),
        known => {
            (body.is_empty() || decode::unpack(body, known, 1).is_some()).then(|| header.to_owned())
        }
    }
}

/// The program that sent a flow, as the other tool saw it.
pub(crate) fn app(name: &str, pid: u32) -> Option<Arc<AppAnswer>> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let answer = AppAnswer::default();
    answer.give(Some(App {
        pid,
        name: name.to_owned(),
        path: None,
    }));
    Some(Arc::new(answer))
}

/// `1.2.3.4:5` or `[::1]:5`, as the proxy writes an address.
pub(crate) fn address(ip: &str, port: u16) -> Option<String> {
    let ip: IpAddr = ip
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok()?;
    Some(SocketAddr::new(ip, port).to_string())
}

/// A time as other tools write one: with its offset, or without, in UTC. .NET
/// writes seven places after the second, and Java three.
pub(crate) fn unix_micros(date: &str) -> Option<i64> {
    let date = date.trim();
    let at = OffsetDateTime::parse(date, &Iso8601::DEFAULT)
        .or_else(|_| PrimitiveDateTime::parse(date, &Iso8601::DEFAULT).map(|at| at.assume_utc()))
        .ok()?;
    // .NET's way of saying never.
    (at.year() > 1).then(|| (at.unix_timestamp_nanos() / 1000) as i64)
}

/// `HTTP/2.0` as capture writes it, from `h2`, `http/2.0` or `HTTP/2`.
pub(crate) fn version_of(text: &str) -> String {
    match text.trim().to_ascii_lowercase().as_str() {
        "h2" | "http/2" | "http/2.0" => "HTTP/2.0".to_owned(),
        "h3" | "http/3" | "http/3.0" => "HTTP/3.0".to_owned(),
        "http/1.0" => "HTTP/1.0".to_owned(),
        "" | "http/1.1" => "HTTP/1.1".to_owned(),
        _ => text.trim().to_owned(),
    }
}

/// The host as the list shows it, without the scheme's own port.
pub(crate) fn host_of(authority: &str, scheme: &str) -> String {
    let default_port = if scheme == "https" { ":443" } else { ":80" };
    authority
        .strip_suffix(default_port)
        .unwrap_or(authority)
        .to_ascii_lowercase()
}

/// The `name: value` lines a flow keeps its headers as.
pub(crate) fn header_lines<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    pairs
        .into_iter()
        .map(|(name, value)| format!("{name}: {value}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_known_by_its_first_bytes() {
        assert_eq!(Format::sniff(b"PK\x03\x04rest").unwrap(), Format::Saz);
        assert_eq!(Format::sniff(b"  \n{\"log\"").unwrap(), Format::Har);
        assert_eq!(Format::sniff(b"\xEF\xBB\xBF[{").unwrap(), Format::Charles);
        let native = Format::sniff(&[0xAC, 0xED, 0, 5]).unwrap_err();
        assert!(format!("{native}").contains(".chlsj"));
        assert!(Format::sniff(b"GET / HTTP/1.1").is_err());
    }

    #[test]
    fn times_are_read_as_tools_write_them() {
        let net = unix_micros("2026-11-07T17:00:01.1234567+07:00").unwrap();
        let java = unix_micros("2026-11-07T10:00:01.123Z").unwrap();
        assert_eq!(net, 1_794_045_601_123_456);
        assert_eq!(java, 1_794_045_601_123_000);
        assert_eq!(unix_micros("2026-11-07T10:00:01.123").unwrap(), java);
        assert_eq!(unix_micros("0001-01-01T00:00:00"), None);
        assert_eq!(unix_micros("soon"), None);
    }

    #[test]
    fn a_body_said_packed_that_is_not_is_kept_as_it_is() {
        use std::io::Write as _;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(b"hello").unwrap();
        let packed = gz.finish().unwrap();
        assert_eq!(
            encoding_kept(&packed, Some("gzip")).as_deref(),
            Some("gzip")
        );
        assert_eq!(encoding_kept(b"hello", Some("gzip")), None);
        assert_eq!(encoding_kept(b"hello", None), None);
        assert_eq!(encoding_kept(b"x", Some("identity")), None);
        assert_eq!(
            encoding_kept(b"x", Some("snappy")).as_deref(),
            Some("snappy")
        );
    }

    #[test]
    fn addresses_are_written_as_the_proxy_does() {
        assert_eq!(address("10.0.0.5", 443).as_deref(), Some("10.0.0.5:443"));
        assert_eq!(address("::1", 80).as_deref(), Some("[::1]:80"));
        assert_eq!(address("[::1]", 80).as_deref(), Some("[::1]:80"));
        assert_eq!(address("example.com", 80), None);
        assert_eq!(host_of("API.test:443", "https"), "api.test");
        assert_eq!(host_of("api.test:8443", "https"), "api.test:8443");
    }

    #[test]
    fn connections_count_from_one() {
        let mut connections = Connections::default();
        assert_eq!(connections.over(Some("a".into())), Some((1, true)));
        assert_eq!(connections.over(Some("b".into())), Some((2, true)));
        assert_eq!(connections.over(Some("a".into())), Some((1, false)));
        assert_eq!(connections.over(Some(String::new())), None);
        assert_eq!(connections.over(None), None);
    }
}
