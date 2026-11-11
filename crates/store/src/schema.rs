//! `.proto` files, and descriptor sets made of them, that name the fields of
//! Protobuf and gRPC bodies. The sources chosen are kept in `protos.toml`,
//! and read again when a file in them changes.

use prost_reflect::{DescriptorPool, MessageDescriptor, MethodDescriptor};
use protox::file::{
    ChainFileResolver, DescriptorSetFileResolver, GoogleFileResolver, IncludeFileResolver,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

/// Past this many files, the rest of a folder is not read.
const FILES_MAX: usize = 20_000;

/// How long a check that nothing changed holds.
const LOOK_EVERY: Duration = Duration::from_secs(2);

/// Extensions of a descriptor set, as `protoc -o` writes one.
const SET_EXTENSIONS: [&str; 4] = ["pb", "desc", "binpb", "protoset"];

/// What was made of the sources.
#[derive(Debug, Default)]
pub struct Schema {
    pool: DescriptorPool,
    pub sources: Vec<Source>,
}

/// One source: what was read from it, and what could not be.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    pub path: PathBuf,
    pub files: usize,
    pub messages: usize,
    pub services: usize,
    pub problems: Vec<String>,
}

impl Schema {
    /// Compiles the sources: folders of `.proto` files, `.proto` files, and
    /// descriptor sets. A file that cannot be read is told of, and the rest
    /// are still read.
    pub fn load(sources: &[PathBuf]) -> Self {
        let mut resolver = ChainFileResolver::new();
        // What each source opens: paths of `.proto` files, or the names in a set.
        let mut opened: Vec<(usize, Vec<PathBuf>)> = Vec::new();
        let mut reports: Vec<Source> = sources
            .iter()
            .map(|path| Source {
                path: path.clone(),
                ..Source::default()
            })
            .collect();
        for (i, path) in sources.iter().enumerate() {
            let report = &mut reports[i];
            if path.is_dir() {
                resolver.add(IncludeFileResolver::new(path.clone()));
                let mut files = Vec::new();
                proto_files(path, &mut files);
                if files.len() >= FILES_MAX {
                    report
                        .problems
                        .push(format!("only the first {FILES_MAX} .proto files are read"));
                }
                opened.push((i, files));
            } else if is_set(path) {
                match std::fs::read(path)
                    .map_err(|e| e.to_string())
                    .and_then(|data| {
                        DescriptorSetFileResolver::decode(&data[..]).map_err(|e| e.to_string())
                    }) {
                    Ok(set) => {
                        let names = set_names(path);
                        resolver.add(set);
                        opened.push((i, names));
                    }
                    Err(e) => report.problems.push(e),
                }
            } else if path.is_file() {
                let parent = path.parent().unwrap_or(Path::new(".")).to_owned();
                resolver.add(IncludeFileResolver::new(parent));
                opened.push((i, vec![path.clone()]));
            } else {
                report.problems.push("not found".to_owned());
            }
        }
        resolver.add(GoogleFileResolver::new());
        let mut compiler = protox::Compiler::with_file_resolver(resolver);
        compiler.include_imports(true);
        let mut names: Vec<(usize, String)> = Vec::new();
        for (i, files) in opened {
            for file in files {
                let before: Vec<String> = compiler.files().map(|f| f.name().to_owned()).collect();
                match compiler.open_file(&file) {
                    Ok(_) => {
                        // The file just opened is the one not there before, or
                        // one opened already as an import.
                        let name = compiler
                            .files()
                            .find(|f| {
                                f.path() == Some(file.as_path())
                                    || f.name() == file.to_string_lossy()
                            })
                            .or_else(|| {
                                compiler
                                    .files()
                                    .find(|f| !before.iter().any(|b| b == f.name()))
                            })
                            .map(|f| f.name().to_owned());
                        if let Some(name) = name {
                            names.push((i, name));
                        }
                    }
                    Err(e) => reports[i].problems.push(describe(&e, &file)),
                }
            }
        }
        let pool = compiler.descriptor_pool();
        for (i, name) in names {
            if let Some(file) = pool.get_file_by_name(&name) {
                let report = &mut reports[i];
                report.files += 1;
                report.messages += count_messages(file.messages());
                report.services += file.services().len();
            }
        }
        Self {
            pool,
            sources: reports,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pool.services().len() == 0 && self.pool.all_messages().len() == 0
    }

    /// The call a gRPC path names: `/package.Service/Method`, perhaps under
    /// a prefix a gateway adds.
    pub fn method(&self, path: &str) -> Option<MethodDescriptor> {
        let path = path.split(['?', '#']).next().unwrap_or(path);
        let mut parts = path.rsplit('/');
        let method = parts.next()?;
        let service = parts.next()?;
        self.pool
            .get_service_by_name(service)?
            .methods()
            .find(|m| m.name() == method)
    }

    /// Every message's full name, but for maps' entries and Google's own
    /// types, in order.
    pub fn message_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .pool
            .all_messages()
            .filter(|m| !m.is_map_entry() && m.package_name() != "google.protobuf")
            .map(|m| m.full_name().to_owned())
            .collect();
        names.sort();
        names
    }

    /// A message by its full name, with or without a leading dot.
    pub fn message(&self, name: &str) -> Option<MessageDescriptor> {
        self.pool.get_message_by_name(name.trim_start_matches('.'))
    }

    /// The message a body holds, as far as its path and type tell: a gRPC
    /// call's request or answer, or what `messageType=` or `proto=` names.
    pub fn message_for(
        &self,
        path: &str,
        content_type: Option<&str>,
        request: bool,
        format: crate::binjson::Format,
    ) -> Option<MessageDescriptor> {
        use crate::binjson::Format;
        match format {
            Format::Grpc | Format::GrpcWeb | Format::GrpcWebText => {
                let method = self.method(path)?;
                Some(if request {
                    method.input()
                } else {
                    method.output()
                })
            }
            Format::Protobuf => {
                let named = content_type?.split(';').skip(1).find_map(|param| {
                    let (key, value) = param.split_once('=')?;
                    matches!(
                        key.trim().to_ascii_lowercase().as_str(),
                        "messagetype" | "proto" | "type"
                    )
                    .then(|| value.trim().trim_matches('"').to_owned())
                })?;
                self.message(&named)
            }
            _ => None,
        }
    }
}

/// An error with the file and the line it is on, as `shop.proto:3:9: …`.
fn describe(error: &protox::Error, opened: &Path) -> String {
    use miette::Diagnostic;
    fn place(diagnostic: &dyn Diagnostic, code: Option<&dyn miette::SourceCode>) -> Option<String> {
        let code = diagnostic.source_code().or(code);
        if let (Some(code), Some(label)) = (code, diagnostic.labels().and_then(|mut l| l.next())) {
            let span = code.read_span(label.inner(), 0, 0).ok()?;
            return Some(format!("{}:{}", span.line() + 1, span.column() + 1));
        }
        diagnostic
            .related()?
            .find_map(|related| place(related, code))
    }
    let file = error.file().map(str::to_owned).unwrap_or_else(|| {
        opened
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    let mut text = error.to_string();
    let detail = error
        .related()
        .and_then(|mut related| related.next().map(|r| r.to_string()));
    if let Some(detail) = detail.filter(|d| *d != text) {
        text = detail;
    }
    match place(error, None) {
        Some(at) => format!("{file}:{at}: {text}"),
        None => format!("{file}: {text}"),
    }
}

fn without_query(path: &str) -> &str {
    path.split(['?', '#']).next().unwrap_or(path)
}

fn count_messages(messages: impl Iterator<Item = MessageDescriptor>) -> usize {
    messages
        .filter(|m| !m.is_map_entry())
        .map(|m| 1 + count_messages(m.child_messages()))
        .sum()
}

fn is_set(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SET_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// The files a descriptor set holds, by name.
fn set_names(path: &Path) -> Vec<PathBuf> {
    use prost::Message;
    std::fs::read(path)
        .ok()
        .and_then(|data| prost_types::FileDescriptorSet::decode(&data[..]).ok())
        .map(|set| {
            set.file
                .into_iter()
                .map(|f| PathBuf::from(f.name()))
                .collect()
        })
        .unwrap_or_default()
}

/// `.proto` files under a folder, past hidden folders and `node_modules`.
fn proto_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if out.len() >= FILES_MAX {
            return;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if !name.starts_with('.') && name != "node_modules" && name != "target" {
                proto_files(&path, out);
            }
        } else if name.ends_with(".proto") {
            out.push(path);
        }
    }
}

/// What the sources are now, so that a change is seen: each file's path,
/// size and time.
fn stamp(sources: &[PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for source in sources {
        let mut files = Vec::new();
        if source.is_dir() {
            proto_files(source, &mut files);
        } else {
            files.push(source.clone());
        }
        for file in files {
            file.hash(&mut hasher);
            if let Ok(meta) = std::fs::metadata(&file) {
                meta.len().hash(&mut hasher);
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

#[derive(Default, Deserialize, Serialize)]
struct Kept {
    #[serde(default)]
    sources: Vec<PathBuf>,
    #[serde(default)]
    chosen: Vec<Chosen>,
}

/// A message chosen by hand for the bodies one side of a path sends.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Chosen {
    pub host: String,
    /// Without its query.
    pub path: String,
    pub request: bool,
    pub message: String,
}

struct State {
    sources: Vec<PathBuf>,
    chosen: Vec<Chosen>,
    schema: Arc<Schema>,
    stamp: Option<u64>,
    looked: Option<Instant>,
}

/// The sources chosen, and the schema made of them, made again when they
/// change. Shared by the inspector and the AI tools.
pub struct Library {
    file: PathBuf,
    state: Mutex<State>,
}

impl Library {
    /// The sources kept in `protos.toml` in the data folder. Nothing is
    /// compiled until a schema is asked for.
    pub fn open(data_dir: &Path) -> Arc<Self> {
        let file = data_dir.join("protos.toml");
        let kept: Kept = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();
        Arc::new(Self {
            file,
            state: Mutex::new(State {
                sources: kept.sources,
                chosen: kept.chosen,
                schema: Arc::new(Schema::default()),
                stamp: None,
                looked: None,
            }),
        })
    }

    pub fn sources(&self) -> Vec<PathBuf> {
        self.lock().sources.clone()
    }

    /// The schema, compiled again if a source changed since it was last
    /// looked at. Slow the first time and after a change: not on the UI
    /// thread.
    pub fn schema(&self) -> Arc<Schema> {
        let mut state = self.lock();
        if state.sources.is_empty() {
            return Arc::clone(&state.schema);
        }
        if state.looked.is_some_and(|at| at.elapsed() < LOOK_EVERY) {
            return Arc::clone(&state.schema);
        }
        state.looked = Some(Instant::now());
        let now = stamp(&state.sources);
        if state.stamp != Some(now) {
            state.schema = Arc::new(Schema::load(&state.sources));
            state.stamp = Some(now);
        }
        Arc::clone(&state.schema)
    }

    /// Reads the files again the next time the schema is asked for, even if
    /// nothing seems to have changed.
    pub fn forget(&self) {
        let mut state = self.lock();
        state.stamp = None;
        state.looked = None;
    }

    /// The schema as last made, without looking at the files.
    pub fn last_schema(&self) -> Arc<Schema> {
        Arc::clone(&self.lock().schema)
    }

    /// Adds sources, keeps the list, and makes the schema again.
    pub fn add(&self, paths: impl IntoIterator<Item = PathBuf>) -> std::io::Result<Arc<Schema>> {
        let mut state = self.lock();
        for path in paths {
            if !state.sources.contains(&path) {
                state.sources.push(path);
            }
        }
        self.changed(&mut state)
    }

    pub fn remove(&self, path: &Path) -> std::io::Result<Arc<Schema>> {
        let mut state = self.lock();
        state.sources.retain(|p| p != path);
        self.changed(&mut state)
    }

    /// The message chosen for one side of a path, if one was.
    pub fn chosen(&self, host: &str, path: &str, request: bool) -> Option<String> {
        let path = without_query(path);
        self.lock()
            .chosen
            .iter()
            .find(|c| c.host == host && c.path == path && c.request == request)
            .map(|c| c.message.clone())
    }

    /// Chooses the message one side of a path sends, or with `None` lets it
    /// be found again as it would be. Kept with the sources.
    pub fn choose(
        &self,
        host: &str,
        path: &str,
        request: bool,
        message: Option<&str>,
    ) -> std::io::Result<()> {
        let path = without_query(path);
        let mut state = self.lock();
        state
            .chosen
            .retain(|c| !(c.host == host && c.path == path && c.request == request));
        if let Some(message) = message {
            state.chosen.push(Chosen {
                host: host.to_owned(),
                path: path.to_owned(),
                request,
                message: message.to_owned(),
            });
        }
        self.save(&state)
    }

    fn save(&self, state: &State) -> std::io::Result<()> {
        let kept = Kept {
            sources: state.sources.clone(),
            chosen: state.chosen.clone(),
        };
        let text = toml::to_string(&kept).map_err(std::io::Error::other)?;
        let part = self.file.with_extension("toml.part");
        std::fs::write(&part, text)?;
        std::fs::rename(&part, &self.file)
    }

    fn changed(&self, state: &mut State) -> std::io::Result<Arc<Schema>> {
        self.save(state)?;
        state.schema = Arc::new(Schema::load(&state.sources));
        state.stamp = Some(stamp(&state.sources));
        state.looked = Some(Instant::now());
        Ok(Arc::clone(&state.schema))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const SHOP: &str = r#"
syntax = "proto3";
package shop;
import "google/protobuf/timestamp.proto";
import "common/money.proto";

enum State {
  STATE_UNSPECIFIED = 0;
  PAID = 1;
  SENT = 2;
}

message Item {
  string sku = 1;
  uint32 qty = 2;
}

message Order {
  uint64 id = 1;
  string customer = 2;
  repeated Item items = 3;
  common.Money total = 4;
  State state = 5;
  google.protobuf.Timestamp paid_at = 6;
  map<string, string> notes = 7;
}

message GetOrder { uint64 id = 1; }

service Orders {
  rpc Get(GetOrder) returns (Order);
}
"#;

    pub(crate) const MONEY: &str = r#"
syntax = "proto3";
package common;
message Money { string currency = 1; int64 units = 2; }
"#;

    pub(crate) fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracewing-schema-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("common")).unwrap();
        std::fs::write(dir.join("shop.proto"), SHOP).unwrap();
        std::fs::write(dir.join("common/money.proto"), MONEY).unwrap();
        dir
    }

    #[test]
    fn a_folder_is_read_with_its_imports() {
        let dir = temp("folder");
        let schema = Schema::load(std::slice::from_ref(&dir));
        let source = &schema.sources[0];
        assert_eq!(source.problems, Vec::<String>::new());
        assert_eq!((source.files, source.messages, source.services), (2, 4, 1));
        let method = schema.method("/shop.Orders/Get").unwrap();
        assert_eq!(method.output().full_name(), "shop.Order");
        assert!(
            schema.method("/api/v1/shop.Orders/Get").is_some(),
            "under a prefix"
        );
        assert!(schema.method("/shop.Orders/Nope").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_broken_file_is_told_of_and_the_rest_read() {
        let dir = temp("broken");
        std::fs::write(dir.join("broken.proto"), "syntax = \"proto3\";\nmessage {").unwrap();
        let schema = Schema::load(std::slice::from_ref(&dir));
        let source = &schema.sources[0];
        assert_eq!(source.problems.len(), 1, "{:?}", source.problems);
        assert!(
            source.problems[0].starts_with("broken.proto:2:9: "),
            "{:?}",
            source.problems
        );
        assert!(schema.message("shop.Order").is_some());
        let missing = Schema::load(&[dir.join("gone")]);
        assert_eq!(missing.sources[0].problems, ["not found"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_descriptor_set_and_a_single_file_are_read() {
        let dir = temp("set");
        let mut compiler = protox::Compiler::new([&dir]).unwrap();
        compiler.include_imports(true);
        compiler.open_file("shop.proto").unwrap();
        let set = dir.join("shop.pb");
        std::fs::write(&set, compiler.encode_file_descriptor_set()).unwrap();
        let schema = Schema::load(std::slice::from_ref(&set));
        assert_eq!(schema.sources[0].problems, Vec::<String>::new());
        assert!(schema.method("/shop.Orders/Get").is_some());

        let schema = Schema::load(&[dir.join("common/money.proto")]);
        assert_eq!(schema.sources[0].files, 1);
        assert!(schema.message(".common.Money").is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_type_is_found_by_path_or_named() {
        use crate::binjson::Format;
        let dir = temp("found");
        let schema = Schema::load(std::slice::from_ref(&dir));
        let name = |m: Option<MessageDescriptor>| m.map(|m| m.full_name().to_owned());
        assert_eq!(
            name(schema.message_for("/shop.Orders/Get", None, true, Format::Grpc)).as_deref(),
            Some("shop.GetOrder")
        );
        assert_eq!(
            name(schema.message_for("/shop.Orders/Get", None, false, Format::GrpcWeb)).as_deref(),
            Some("shop.Order")
        );
        let typed = "application/x-protobuf; messageType=\"shop.Item\"";
        assert_eq!(
            name(schema.message_for("/x", Some(typed), false, Format::Protobuf)).as_deref(),
            Some("shop.Item")
        );
        assert_eq!(
            schema.message_for(
                "/x",
                Some("application/x-protobuf"),
                false,
                Format::Protobuf
            ),
            None
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_library_keeps_its_sources_and_sees_changes() {
        let dir = temp("library");
        let data = dir.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let library = Library::open(&data);
        assert!(library.schema().is_empty());
        let schema = library.add([dir.join("common")]).unwrap();
        assert!(schema.message("common.Money").is_some());
        assert_eq!(Library::open(&data).sources(), [dir.join("common")]);

        std::fs::write(
            dir.join("common/more.proto"),
            "syntax = \"proto3\"; package common; message Tag { string name = 1; }",
        )
        .unwrap();
        library.lock().looked = None;
        assert!(
            library.schema().message("common.Tag").is_some(),
            "read again"
        );

        library
            .choose("api.shop.vn", "/orders/42?x=1", false, Some("common.Money"))
            .unwrap();
        let again = Library::open(&data);
        assert_eq!(
            again.chosen("api.shop.vn", "/orders/42", false).as_deref(),
            Some("common.Money")
        );
        assert_eq!(again.chosen("api.shop.vn", "/orders/42", true), None);
        assert_eq!(again.sources(), [dir.join("common")], "kept beside");
        library
            .choose("api.shop.vn", "/orders/42", false, None)
            .unwrap();
        assert_eq!(
            Library::open(&data).chosen("api.shop.vn", "/orders/42", false),
            None
        );
        assert_eq!(
            library.last_schema().message_names(),
            ["common.Money", "common.Tag"]
        );

        library.remove(&dir.join("common")).unwrap();
        assert!(library.schema().is_empty());
        assert!(Library::open(&data).sources().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
