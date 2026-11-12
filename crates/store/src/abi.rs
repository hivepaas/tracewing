//! The contracts the user chose, by their ABI, so that a call shows the name
//! it has instead of four bytes: the JSON `solc`, Hardhat and Foundry write,
//! read from folders or files named in Settings, as `.proto` files are.
//!
//! The four bytes and the topics are worked out here from each signature's
//! text, the same way [`crate::web3`] works out the ones every chain has.
//! Nothing is fetched: an ABI comes from the user's disk, never from a
//! directory that would be told what their wallet is doing.

use crate::web3::{selector_of, topic_of};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

/// Past this many files a folder is not read further: a node_modules of them
/// is not what was meant.
const FILES_MAX: usize = 20_000;

/// The files are looked at again no more often than this.
const LOOK_EVERY: Duration = Duration::from_secs(2);

/// What was read from the sources: the names, by what a call carries.
#[derive(Debug, Default)]
pub struct Abi {
    functions: HashMap<[u8; 4], String>,
    events: HashMap<[u8; 32], String>,
    errors: HashMap<[u8; 4], String>,
    pub sources: Vec<Source>,
}

/// One source: what was read from it, and what could not be.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    pub path: PathBuf,
    pub files: usize,
    pub functions: usize,
    pub events: usize,
    pub errors: usize,
    pub problems: Vec<String>,
}

impl Abi {
    /// Reads every ABI in `sources`: a folder is walked for `.json`, a file
    /// is read as it is. A file that is not an ABI is not a fault of the
    /// user's: it is counted as a problem of its source and passed over.
    pub fn load(sources: &[PathBuf]) -> Self {
        let mut abi = Self::default();
        for path in sources {
            let mut source = Source {
                path: path.clone(),
                ..Source::default()
            };
            let mut files = Vec::new();
            if path.is_dir() {
                json_files(path, &mut files);
                if files.len() >= FILES_MAX {
                    source
                        .problems
                        .push(format!("only the first {FILES_MAX} .json files are read"));
                }
            } else {
                files.push(path.clone());
            }
            for file in files {
                match std::fs::read_to_string(&file) {
                    Ok(text) => match entries_of(&text) {
                        Some(entries) => {
                            source.files += 1;
                            abi.take(&entries, &mut source);
                        }
                        None => source.problems.push(format!(
                            "{}: not an ABI",
                            file.file_name().unwrap_or_default().to_string_lossy()
                        )),
                    },
                    Err(e) => source.problems.push(format!("{}: {e}", file.display())),
                }
            }
            abi.sources.push(source);
        }
        abi
    }

    /// The signature of the function those four bytes name. The `data` of a
    /// call may be passed whole: only its first four bytes are read.
    pub fn function(&self, data: &[u8]) -> Option<&str> {
        let four: [u8; 4] = data.get(..4)?.try_into().ok()?;
        self.functions.get(&four).map(String::as_str)
    }

    /// The signature of the event that first topic names.
    pub fn event(&self, topic: &[u8]) -> Option<&str> {
        let whole: [u8; 32] = topic.get(..32)?.try_into().ok()?;
        self.events.get(&whole).map(String::as_str)
    }

    /// The signature of the custom error those four bytes name, which is
    /// what a revert carries when it is not `Error(string)`.
    pub fn error(&self, data: &[u8]) -> Option<&str> {
        let four: [u8; 4] = data.get(..4)?.try_into().ok()?;
        self.errors.get(&four).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty() && self.events.is_empty() && self.errors.is_empty()
    }

    /// How many names it holds: functions, events and errors.
    pub fn len(&self) -> usize {
        self.functions.len() + self.events.len() + self.errors.len()
    }

    fn take(&mut self, entries: &[Value], source: &mut Source) {
        for entry in entries {
            let Some(signature) = signature_of(entry) else {
                continue;
            };
            match entry.get("type").and_then(Value::as_str) {
                Some("function") => {
                    let four = selector_of(&signature);
                    let new = self.functions.insert(four, signature).is_none();
                    source.functions += usize::from(new);
                }
                Some("event") => {
                    let topic = topic_of(&signature);
                    let new = self.events.insert(topic, signature).is_none();
                    source.events += usize::from(new);
                }
                Some("error") => {
                    let four = selector_of(&signature);
                    let new = self.errors.insert(four, signature).is_none();
                    source.errors += usize::from(new);
                }
                // A constructor, a fallback and a receive are never called
                // by name, so they have no four bytes to be found by.
                _ => {}
            }
        }
    }
}

/// The entries of an ABI, whether the file is the array itself or one of the
/// artifacts Hardhat and Foundry write, which keep it under `abi`.
fn entries_of(text: &str) -> Option<Vec<Value>> {
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Array(entries) => Some(entries),
        Value::Object(artifact) => match artifact.get("abi")? {
            Value::Array(entries) => Some(entries.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// The signature of one entry, which its four bytes are worked out from:
/// the name, then the types in brackets.
fn signature_of(entry: &Value) -> Option<String> {
    let name = entry.get("name")?.as_str()?;
    if name.is_empty() {
        return None;
    }
    let types: Option<Vec<String>> = match entry.get("inputs") {
        Some(Value::Array(inputs)) => inputs.iter().map(type_of).collect(),
        // An entry with no inputs at all takes none.
        None => Some(Vec::new()),
        Some(_) => None,
    };
    Some(format!("{name}({})", types?.join(",")))
}

/// One input's type. A tuple is written as the types in it, keeping whatever
/// brackets followed it: `(address,bool,bytes)[]`.
fn type_of(input: &Value) -> Option<String> {
    let kind = input.get("type")?.as_str()?;
    let Some(arrays) = kind.strip_prefix("tuple") else {
        return Some(kind.to_owned());
    };
    let parts: Option<Vec<String>> = match input.get("components") {
        Some(Value::Array(components)) => components.iter().map(type_of).collect(),
        _ => return None,
    };
    Some(format!("({}){arrays}", parts?.join(",")))
}

fn json_files(dir: &Path, out: &mut Vec<PathBuf>) {
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
                json_files(&path, out);
            }
        } else if name.ends_with(".json") {
            out.push(path);
        }
    }
}

/// Whether anything in the sources changed, as cheaply as asking the disk
/// for each file's size and when it was last written.
fn stamp(sources: &[PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for source in sources {
        let mut files = Vec::new();
        if source.is_dir() {
            json_files(source, &mut files);
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

#[derive(Debug, Default, Deserialize, Serialize)]
struct Kept {
    #[serde(default)]
    sources: Vec<PathBuf>,
}

struct State {
    sources: Vec<PathBuf>,
    abi: Arc<Abi>,
    stamp: Option<u64>,
    looked: Option<Instant>,
}

/// The sources chosen, and the names read from them, read again when they
/// change. Shared by the inspector and the AI tools, as the schema is.
pub struct Library {
    file: PathBuf,
    state: Mutex<State>,
}

impl Library {
    /// The sources kept in `abis.toml` in the data folder. Nothing is read
    /// until a name is asked for.
    pub fn open(data_dir: &Path) -> Arc<Self> {
        let file = data_dir.join("abis.toml");
        let kept: Kept = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();
        Arc::new(Self {
            file,
            state: Mutex::new(State {
                sources: kept.sources,
                abi: Arc::new(Abi::default()),
                stamp: None,
                looked: None,
            }),
        })
    }

    pub fn sources(&self) -> Vec<PathBuf> {
        self.lock().sources.clone()
    }

    /// The names, read again if a source changed since they were last looked
    /// at. Reads the disk: not on the UI thread.
    pub fn abi(&self) -> Arc<Abi> {
        let mut state = self.lock();
        if state.sources.is_empty() {
            return Arc::clone(&state.abi);
        }
        if state.looked.is_some_and(|at| at.elapsed() < LOOK_EVERY) {
            return Arc::clone(&state.abi);
        }
        state.looked = Some(Instant::now());
        let now = stamp(&state.sources);
        if state.stamp != Some(now) {
            state.abi = Arc::new(Abi::load(&state.sources));
            state.stamp = Some(now);
        }
        Arc::clone(&state.abi)
    }

    /// The names as last read, without looking at the files.
    pub fn last_abi(&self) -> Arc<Abi> {
        Arc::clone(&self.lock().abi)
    }

    /// Reads the files again next time, even if nothing seems to have
    /// changed.
    pub fn forget(&self) {
        let mut state = self.lock();
        state.stamp = None;
        state.looked = None;
    }

    /// Adds sources, keeps the list, and reads them.
    pub fn add(&self, paths: impl IntoIterator<Item = PathBuf>) -> std::io::Result<Arc<Abi>> {
        let mut state = self.lock();
        for path in paths {
            if !state.sources.contains(&path) {
                state.sources.push(path);
            }
        }
        self.changed(&mut state)
    }

    pub fn remove(&self, path: &Path) -> std::io::Result<Arc<Abi>> {
        let mut state = self.lock();
        state.sources.retain(|kept| kept != path);
        self.changed(&mut state)
    }

    fn changed(&self, state: &mut State) -> std::io::Result<Arc<Abi>> {
        self.save(state)?;
        state.abi = Arc::new(Abi::load(&state.sources));
        state.stamp = Some(stamp(&state.sources));
        state.looked = Some(Instant::now());
        Ok(Arc::clone(&state.abi))
    }

    fn save(&self, state: &State) -> std::io::Result<()> {
        let kept = Kept {
            sources: state.sources.clone(),
        };
        let text = toml::to_string_pretty(&kept).unwrap_or_default();
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.file, text)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web3;

    /// What `solc` writes for an ERC-20, cut to what matters here.
    const ERC20: &str = r#"[
      {"type":"function","name":"transfer","stateMutability":"nonpayable",
       "inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],
       "outputs":[{"name":"","type":"bool"}]},
      {"type":"function","name":"totalSupply","inputs":[],"outputs":[{"type":"uint256"}]},
      {"type":"event","name":"Transfer","anonymous":false,
       "inputs":[{"indexed":true,"name":"from","type":"address"},
                 {"indexed":true,"name":"to","type":"address"},
                 {"indexed":false,"name":"value","type":"uint256"}]},
      {"type":"error","name":"InsufficientBalance",
       "inputs":[{"name":"have","type":"uint256"},{"name":"want","type":"uint256"}]},
      {"type":"constructor","inputs":[{"name":"owner","type":"address"}]},
      {"type":"receive","stateMutability":"payable"}
    ]"#;

    /// A tuple in the inputs, which is written as its own bracketed list.
    const MULTICALL: &str = r#"{"contractName":"Multicall3","abi":[
      {"type":"function","name":"aggregate3","stateMutability":"payable",
       "inputs":[{"name":"calls","type":"tuple[]","components":[
          {"name":"target","type":"address"},
          {"name":"allowFailure","type":"bool"},
          {"name":"callData","type":"bytes"}]}],
       "outputs":[]}
    ]}"#;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tracewing-abi-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_signature_is_the_name_and_the_types_in_brackets() {
        let entries = entries_of(ERC20).expect("an ABI");
        let signatures: Vec<String> = entries.iter().filter_map(signature_of).collect();
        assert_eq!(
            signatures,
            [
                "transfer(address,uint256)",
                "totalSupply()",
                "Transfer(address,address,uint256)",
                "InsufficientBalance(uint256,uint256)",
                // A constructor has a name of none, so it is left out above;
                // this one is the receive, which has no name either.
            ],
            "a constructor and a receive have no name to be found by"
        );
        // A tuple keeps the brackets that followed it.
        let multicall = entries_of(MULTICALL).expect("an artifact holds its ABI");
        assert_eq!(
            signature_of(&multicall[0]).as_deref(),
            Some("aggregate3((address,bool,bytes)[])")
        );
    }

    #[test]
    fn the_names_are_found_by_what_a_call_carries() {
        let dir = temp("read");
        std::fs::write(dir.join("erc20.json"), ERC20).unwrap();
        std::fs::write(dir.join("multicall.json"), MULTICALL).unwrap();
        std::fs::write(dir.join("notes.json"), r#"{"what":"not an ABI"}"#).unwrap();
        let abi = Abi::load(std::slice::from_ref(&dir));
        assert!(!abi.is_empty());
        assert_eq!(
            abi.len(),
            5,
            "two functions, an event and an error from the ERC-20, and the tuple"
        );
        // The same four bytes a real call carries, worked out from the text.
        assert_eq!(
            abi.function(&web3::selector_of("transfer(address,uint256)")),
            Some("transfer(address,uint256)")
        );
        assert_eq!(
            abi.event(&web3::topic_of("Transfer(address,address,uint256)")),
            Some("Transfer(address,address,uint256)")
        );
        assert_eq!(
            abi.error(&web3::selector_of("InsufficientBalance(uint256,uint256)")),
            Some("InsufficientBalance(uint256,uint256)")
        );
        assert_eq!(abi.function(b"\x00\x00\x00\x00"), None);
        let source = &abi.sources[0];
        assert_eq!(source.files, 2, "the one that is not an ABI is not read");
        assert_eq!(
            source.problems,
            ["notes.json: not an ABI"],
            "and it is said, rather than passed over in silence"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_sources_are_kept_and_read_again_when_they_change() {
        let dir = temp("library");
        let abis = dir.join("abis");
        std::fs::create_dir_all(&abis).unwrap();
        std::fs::write(abis.join("erc20.json"), ERC20).unwrap();
        let library = Library::open(&dir);
        assert!(library.sources().is_empty());
        assert!(library.abi().is_empty(), "nothing chosen, nothing read");
        let abi = library.add([abis.clone()]).unwrap();
        assert_eq!(abi.len(), 4);
        // Kept, so the next start has it without being told again.
        let again = Library::open(&dir);
        assert_eq!(again.sources(), std::slice::from_ref(&abis));
        assert_eq!(again.abi().len(), 4);
        // A file that changed is read again.
        std::fs::write(abis.join("more.json"), MULTICALL).unwrap();
        again.forget();
        assert_eq!(again.abi().len(), 5);
        assert_eq!(again.remove(&abis).unwrap().len(), 0);
        assert!(Library::open(&dir).sources().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
