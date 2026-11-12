//! Rules that change traffic on its way: block a URL, answer it from a file, send it
//! to another host, set a header. Each matches by a URL pattern, and a flow says which
//! rules touched it, because a changed response nobody can see the cause of is worse
//! than a bug.

use anyhow::{Context, Result, bail};
use http::{HeaderName, HeaderValue, Uri};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Answers at once, with this status and no body.
    Block {
        status: u16,
    },
    /// Answers from a file, or from a folder by what `*` matched at the pattern's end.
    MapLocal {
        path: PathBuf,
    },
    /// Sends it to this scheme and host instead, keeping the path and query.
    MapRemote {
        to: String,
    },
    SetRequestHeader {
        header: String,
        value: String,
    },
    SetResponseHeader {
        header: String,
        value: String,
    },
    RemoveRequestHeader {
        header: String,
    },
    RemoveResponseHeader {
        header: String,
    },
    /// The server's answer, with this status instead of its own.
    SetStatus {
        status: u16,
    },
    /// Gives the query this parameter, in place of one by the same name or
    /// after the rest.
    SetQuery {
        name: String,
        value: String,
    },
    RemoveQuery {
        name: String,
    },
    /// Every `find` in the request's body becomes `replace`; with `find`
    /// empty, the whole body does.
    ReplaceRequestBody {
        #[serde(default)]
        find: String,
        replace: String,
    },
    ReplaceResponseBody {
        #[serde(default)]
        find: String,
        replace: String,
    },
    /// Holds the request until the window lets it go, edited or not.
    PauseRequest,
    /// The same for the response, before the client gets it.
    PauseResponse,
    /// Holds each message of a WebSocket the app sends, before the server
    /// gets it.
    PauseMessagesToServer,
    /// The same for each the server sends, before the app gets it.
    PauseMessagesToApp,
    /// Runs a JavaScript file's `onRequest` and `onResponse` on it: see
    /// [`crate::script`].
    Script {
        path: PathBuf,
    },
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrafficRule {
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// `api.myapp.dev/v1/*`. `*` stands for anything, a scheme in front is ignored,
    /// and with no path at all it means everything on that host.
    pub url: String,
    /// Empty for every method.
    #[serde(default)]
    pub method: String,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrafficRules {
    pub rules: Vec<TrafficRule>,
}

/// What a request comes to, once every rule has had its say.
#[derive(Debug, Default)]
pub struct Plan {
    /// Answered or sent elsewhere: the first such rule that matches, and only it.
    pub end: Option<End>,
    /// Headers to set, or to take out where there is no value, in the rules' order.
    pub request_headers: Vec<(HeaderName, Option<HeaderValue>)>,
    pub response_headers: Vec<(HeaderName, Option<HeaderValue>)>,
    /// Query parameters to set, or to take out where there is no value.
    pub query: Vec<(String, Option<String>)>,
    /// What to find in a body and what to put in its place, in the rules' order.
    pub request_body: Vec<(String, String)>,
    pub response_body: Vec<(String, String)>,
    /// The status the client gets in place of the server's: the last rule's.
    pub status: Option<u16>,
    /// Which rules touched it, to show on the flow.
    pub names: Vec<String>,
    /// The rule that holds the request, if one does.
    pub pause_request: Option<String>,
    pub pause_response: Option<String>,
    /// The rule that holds a WebSocket's messages each way, if one does.
    pub pause_to_server: Option<String>,
    pub pause_to_app: Option<String>,
    /// The scripts to run on it, in the rules' order, each with its rule's name.
    pub scripts: Vec<(String, PathBuf)>,
}

#[derive(Debug)]
pub enum End {
    Block(u16),
    /// The file to answer with. `None` if there is no such file under the rule's path.
    Local(Option<PathBuf>),
    Remote(Uri),
}

/// `*` in `pattern` for any run of characters. Both are lower case already.
fn glob_fits(pattern: &[u8], text: &[u8]) -> bool {
    let (mut p, mut t) = (0, 0);
    let mut retry = None;
    while t < text.len() {
        if p < pattern.len() && pattern[p] == b'*' {
            p += 1;
            retry = Some((p, t));
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some((after_star, from)) = retry {
            p = after_star;
            t = from + 1;
            retry = Some((after_star, from + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|b| *b == b'*')
}

/// A pattern as it is compared: lower case, no scheme, and a path if it had none.
fn normal_pattern(url: &str) -> String {
    let url = url.trim().to_ascii_lowercase();
    let url = url
        .split_once("://")
        .map_or(url.as_str(), |(_, rest)| rest)
        .to_owned();
    if url.contains('/') {
        url
    } else {
        format!("{url}/*")
    }
}

/// What the `*` at the end of a pattern matched, without the query.
fn rest_of<'a>(pattern: &str, target: &'a str) -> &'a str {
    let Some(literal) = pattern.strip_suffix('*') else {
        return "";
    };
    let rest = target.get(literal.len()..).unwrap_or("");
    rest.split('?').next().unwrap_or("")
}

/// The file a rule's path gives for what the pattern's `*` matched. Nothing outside
/// the folder can be reached, however the URL is written.
pub fn local_file(root: &Path, rest: &str) -> Option<PathBuf> {
    if root.is_file() {
        return Some(root.to_owned());
    }
    let rest = rest.trim_start_matches('/');
    let rest = if rest.is_empty() { "index.html" } else { rest };
    let relative = Path::new(rest);
    if rest.contains('\\')
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let file = root.join(relative).canonicalize().ok()?;
    let root = root.canonicalize().ok()?;
    (file.starts_with(root) && file.is_file()).then_some(file)
}

/// A content type from a file's name, for the kinds worth answering with.
pub fn content_type_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => "application/json",
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn header_pair(name: &str, value: &str) -> Result<(HeaderName, HeaderValue)> {
    Ok((
        HeaderName::from_bytes(name.trim().as_bytes())
            .with_context(|| format!("\"{name}\" cannot be a header's name"))?,
        HeaderValue::from_str(value)
            .with_context(|| format!("\"{value}\" cannot be a header's value"))?,
    ))
}

fn header_name(name: &str) -> Result<HeaderName> {
    HeaderName::from_bytes(name.trim().as_bytes())
        .with_context(|| format!("\"{name}\" cannot be a header's name"))
}

/// Where a request is sent instead. `ws://` and `wss://` are taken as the
/// `http://` and `https://` a WebSocket's upgrade starts as.
fn remote_uri(to: &str) -> Result<Uri> {
    let uri: Uri = to
        .trim()
        .parse()
        .with_context(|| format!("\"{to}\" is not a URL"))?;
    let scheme = match uri.scheme_str() {
        Some("http" | "ws") => "http",
        Some("https" | "wss") => "https",
        _ => bail!("\"{to}\" needs to be like https://host or http://localhost:3000"),
    };
    if uri.authority().is_none() {
        bail!("\"{to}\" needs to be like https://host or http://localhost:3000");
    }
    let mut parts = uri.into_parts();
    parts.scheme = Some(scheme.parse().expect("a scheme"));
    Ok(Uri::from_parts(parts)?)
}

impl TrafficRule {
    /// Refuses what could never work, and says why.
    pub fn check(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("a rule needs a name");
        }
        if self.url.trim().is_empty() {
            bail!("a rule needs a URL to match");
        }
        match &self.action {
            Action::Block { status } => {
                if !(100..=999).contains(status) {
                    bail!("{status} is not a status");
                }
            }
            Action::MapLocal { path } | Action::Script { path } => {
                if !path.is_absolute() {
                    bail!("{} needs to be a full path", path.display());
                }
            }
            Action::MapRemote { to } => {
                remote_uri(to)?;
            }
            Action::SetRequestHeader { header, value }
            | Action::SetResponseHeader { header, value } => {
                header_pair(header, value)?;
            }
            Action::RemoveRequestHeader { header } | Action::RemoveResponseHeader { header } => {
                header_name(header)?;
            }
            Action::SetStatus { status } => {
                if !(100..=999).contains(status) {
                    bail!("{status} is not a status");
                }
            }
            Action::SetQuery { name, .. } | Action::RemoveQuery { name } => {
                if name.trim().is_empty() {
                    bail!("a query parameter needs a name");
                }
            }
            Action::ReplaceRequestBody { .. }
            | Action::ReplaceResponseBody { .. }
            | Action::PauseRequest
            | Action::PauseResponse
            | Action::PauseMessagesToServer
            | Action::PauseMessagesToApp => {}
        }
        Ok(())
    }

    fn fits(&self, method: &str, target: &str, pattern: &str) -> bool {
        self.enabled
            && (self.method.trim().is_empty() || self.method.trim().eq_ignore_ascii_case(method))
            && glob_fits(pattern.as_bytes(), target.as_bytes())
    }
}

/// What a rule set says it is, so that some other TOML is not taken for one.
pub const SET_FORMAT: &str = "tracewing-rules";
const SET_VERSION: u32 = 1;

#[derive(Serialize)]
struct RuleSet<'a> {
    format: &'static str,
    version: u32,
    rules: &'a [TrafficRule],
}

/// Rules as a file to give to someone else, who imports them.
pub fn export_set(rules: &[TrafficRule]) -> Result<String> {
    let set = toml::to_string(&RuleSet {
        format: SET_FORMAT,
        version: SET_VERSION,
        rules,
    })?;
    Ok(format!(
        "# Rules from tracewing. To use them, choose Import in tracewing's Rules.\n\n{set}"
    ))
}

/// What an import did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub added: usize,
    /// The same as one there already, so left out.
    pub already: usize,
    /// Each rule that cannot work here, by its name, and why.
    pub refused: Vec<(String, String)>,
}

impl TrafficRules {
    /// Adds the rules of a set, after those there are. A rule that cannot be
    /// read, or cannot work here, is left out and said so, and the rest come
    /// in. A set's own rules file, without the set's header, will do too.
    pub fn import_set(&mut self, text: &str) -> Result<Imported> {
        #[derive(Deserialize)]
        struct Incoming {
            format: Option<String>,
            version: Option<u32>,
            #[serde(default)]
            rules: Vec<toml::Value>,
        }
        let incoming: Incoming = toml::from_str(text).context("this is not a rules file")?;
        if incoming.format.as_deref().is_some_and(|f| f != SET_FORMAT) {
            bail!("this is not a tracewing rules file");
        }
        if incoming.version.is_some_and(|v| v > SET_VERSION) {
            bail!("these rules come from a newer tracewing");
        }
        if incoming.rules.is_empty() {
            bail!("there are no rules in it");
        }
        let mut imported = Imported::default();
        for (at, value) in incoming.rules.into_iter().enumerate() {
            let name = value
                .get("name")
                .and_then(toml::Value::as_str)
                .map_or_else(|| format!("rule {}", at + 1), str::to_owned);
            let rule = match value.try_into::<TrafficRule>() {
                Ok(rule) => rule,
                Err(e) => {
                    imported.refused.push((name, e.message().trim().to_owned()));
                    continue;
                }
            };
            if let Err(e) = rule.check() {
                imported.refused.push((name, format!("{e:#}")));
            } else if self.rules.contains(&rule) {
                imported.already += 1;
            } else {
                self.rules.push(rule);
                imported.added += 1;
            }
        }
        Ok(imported)
    }

    /// The file's rules; no file means none.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("read {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, toml::to_string(self)?)
            .with_context(|| format!("write {}", path.display()))
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// What the rules make of a request to `host/path?query`. A rule that has gone
    /// bad in its file is passed over, rather than stop traffic.
    pub fn plan(&self, method: &str, target: &str) -> Plan {
        let target = target.to_ascii_lowercase();
        let mut plan = Plan::default();
        for rule in &self.rules {
            let pattern = normal_pattern(&rule.url);
            if !rule.fits(method, &target, &pattern) {
                continue;
            }
            let touched = match &rule.action {
                Action::Block { status } if plan.end.is_none() => {
                    plan.end = Some(End::Block(*status));
                    true
                }
                Action::MapLocal { path } if plan.end.is_none() => {
                    let file = local_file(path, rest_of(&pattern, &target));
                    plan.end = Some(End::Local(file));
                    true
                }
                Action::MapRemote { to } if plan.end.is_none() => match remote_uri(to) {
                    Ok(uri) => {
                        plan.end = Some(End::Remote(uri));
                        true
                    }
                    Err(_) => false,
                },
                Action::SetRequestHeader { header, value } => header_pair(header, value)
                    .map(|(name, value)| plan.request_headers.push((name, Some(value))))
                    .is_ok(),
                Action::SetResponseHeader { header, value } => header_pair(header, value)
                    .map(|(name, value)| plan.response_headers.push((name, Some(value))))
                    .is_ok(),
                Action::RemoveRequestHeader { header } => header_name(header)
                    .map(|name| plan.request_headers.push((name, None)))
                    .is_ok(),
                Action::RemoveResponseHeader { header } => header_name(header)
                    .map(|name| plan.response_headers.push((name, None)))
                    .is_ok(),
                Action::SetStatus { status } => {
                    plan.status = Some(*status);
                    true
                }
                Action::SetQuery { name, value } => {
                    plan.query
                        .push((name.trim().to_owned(), Some(value.clone())));
                    true
                }
                Action::RemoveQuery { name } => {
                    plan.query.push((name.trim().to_owned(), None));
                    true
                }
                Action::ReplaceRequestBody { find, replace } => {
                    plan.request_body.push((find.clone(), replace.clone()));
                    true
                }
                Action::ReplaceResponseBody { find, replace } => {
                    plan.response_body.push((find.clone(), replace.clone()));
                    true
                }
                Action::PauseRequest => {
                    plan.pause_request.get_or_insert_with(|| rule.name.clone());
                    true
                }
                Action::PauseResponse => {
                    plan.pause_response.get_or_insert_with(|| rule.name.clone());
                    true
                }
                Action::PauseMessagesToServer => {
                    plan.pause_to_server
                        .get_or_insert_with(|| rule.name.clone());
                    true
                }
                Action::PauseMessagesToApp => {
                    plan.pause_to_app.get_or_insert_with(|| rule.name.clone());
                    true
                }
                Action::Script { path } => {
                    plan.scripts.push((rule.name.clone(), path.clone()));
                    true
                }
                _ => false,
            };
            if touched {
                plan.names.push(rule.name.clone());
            }
        }
        plan
    }
}

/// A query with parameters set and taken out, in order. A parameter set takes
/// the place of the first by that name, and the others by that name go; one
/// not there comes last. `None` when nothing is left.
pub fn edit_query(query: Option<&str>, edits: &[(String, Option<String>)]) -> Option<String> {
    let mut pairs: Vec<String> = query
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(str::to_owned)
        .collect();
    let name_of = |pair: &str| pair.split('=').next().unwrap_or_default().to_owned();
    for (name, value) in edits {
        let name = query_escape(name);
        let first = pairs.iter().position(|pair| name_of(pair) == name);
        pairs.retain(|pair| name_of(pair) != name);
        if let Some(value) = value {
            let pair = format!("{name}={}", query_escape(value));
            pairs.insert(first.unwrap_or(pairs.len()).min(pairs.len()), pair);
        }
    }
    (!pairs.is_empty()).then(|| pairs.join("&"))
}

/// Text for a query, escaped where a URL needs it.
fn query_escape(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// A body with each `find` made its `replace`, in order. An empty `find` stands
/// for the whole body.
pub fn edit_body(body: Vec<u8>, edits: &[(String, String)]) -> Vec<u8> {
    edits.iter().fold(body, |body, (find, replace)| {
        if find.is_empty() {
            return replace.as_bytes().to_vec();
        }
        let (find, replace) = (find.as_bytes(), replace.as_bytes());
        let mut out = Vec::with_capacity(body.len());
        let mut at = 0;
        while at < body.len() {
            if body[at..].starts_with(find) {
                out.extend_from_slice(replace);
                at += find.len();
            } else {
                out.push(body[at]);
                at += 1;
            }
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_websocket_can_be_sent_elsewhere_by_ws_or_http() {
        assert_eq!(
            remote_uri("ws://localhost:3000").unwrap(),
            "http://localhost:3000/"
        );
        assert_eq!(remote_uri("wss://api.dev").unwrap(), "https://api.dev/");
        assert_eq!(remote_uri(" https://api.dev ").unwrap(), "https://api.dev/");
        assert!(remote_uri("ftp://x").is_err());
        assert!(remote_uri("ws:///nohost").is_err());
    }

    fn named(name: &str, url: &str, action: Action) -> TrafficRule {
        TrafficRule {
            name: name.to_owned(),
            enabled: true,
            url: url.to_owned(),
            method: String::new(),
            action,
        }
    }

    #[test]
    fn a_query_is_set_and_taken_from_in_order() {
        let set = |name: &str, value: &str| (name.to_owned(), Some(value.to_owned()));
        let drop = |name: &str| (name.to_owned(), None);
        assert_eq!(
            edit_query(Some("a=1&env=prod&b=2&env=old"), &[set("env", "dev")]).as_deref(),
            Some("a=1&env=dev&b=2"),
            "in place of the first, the others gone"
        );
        assert_eq!(
            edit_query(Some("a=1"), &[set("q", "a b&c")]).as_deref(),
            Some("a=1&q=a%20b%26c"),
            "added last, escaped"
        );
        assert_eq!(edit_query(Some("debug=1"), &[drop("debug")]), None);
        assert_eq!(
            edit_query(None, &[set("x", "1"), drop("x"), set("y", "2")]).as_deref(),
            Some("y=2")
        );
        assert_eq!(edit_query(Some("a=1&&b"), &[]).as_deref(), Some("a=1&b"));
    }

    #[test]
    fn a_body_is_changed_wherever_it_says_so() {
        let edit = |find: &str, replace: &str| (find.to_owned(), replace.to_owned());
        assert_eq!(
            edit_body(b"aXbXX".to_vec(), &[edit("X", "yy")]),
            b"ayybyyyy"
        );
        assert_eq!(edit_body(b"anything".to_vec(), &[edit("", "{}")]), b"{}");
        assert_eq!(
            edit_body(b"one two".to_vec(), &[edit("one", "two"), edit("two", "2")]),
            b"2 2",
            "in order"
        );
        assert_eq!(edit_body(b"same".to_vec(), &[edit("none", "x")]), b"same");
    }

    #[test]
    fn rewrites_are_planned_in_order_and_checked() {
        let rules = TrafficRules {
            rules: vec![
                named(
                    "keep",
                    "a.dev",
                    Action::SetRequestHeader {
                        header: "x-a".to_owned(),
                        value: "1".to_owned(),
                    },
                ),
                named(
                    "drop",
                    "a.dev",
                    Action::RemoveRequestHeader {
                        header: "x-a".to_owned(),
                    },
                ),
                named("teapot", "a.dev", Action::SetStatus { status: 418 }),
                named("gone", "a.dev", Action::SetStatus { status: 410 }),
                named(
                    "query",
                    "a.dev",
                    Action::RemoveQuery {
                        name: " debug ".to_owned(),
                    },
                ),
                named(
                    "body",
                    "a.dev",
                    Action::ReplaceResponseBody {
                        find: "a".to_owned(),
                        replace: "b".to_owned(),
                    },
                ),
            ],
        };
        let plan = rules.plan("GET", "a.dev/x");
        let headers: Vec<_> = plan
            .request_headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.is_some()))
            .collect();
        assert_eq!(headers, [("x-a", true), ("x-a", false)]);
        assert_eq!(plan.status, Some(410), "the last rule's");
        assert_eq!(plan.query, [("debug".to_owned(), None)]);
        assert_eq!(plan.response_body, [("a".to_owned(), "b".to_owned())]);
        assert_eq!(plan.names.len(), 6);

        assert!(
            named("s", "a.dev", Action::SetStatus { status: 42 })
                .check()
                .is_err()
        );
        let nameless = Action::SetQuery {
            name: " ".to_owned(),
            value: "1".to_owned(),
        };
        assert!(named("q", "a.dev", nameless).check().is_err());
        let bad = Action::RemoveResponseHeader {
            header: "a b".to_owned(),
        };
        assert!(named("h", "a.dev", bad).check().is_err());
    }

    #[test]
    fn a_body_rule_without_find_reads_from_a_file() {
        let rules: TrafficRules = toml::from_str(
            r#"
            [[rules]]
            name = "whole"
            url = "a.dev"
            action = "replace_response_body"
            replace = "{}"
            "#,
        )
        .unwrap();
        assert_eq!(
            rules.rules[0].action,
            Action::ReplaceResponseBody {
                find: String::new(),
                replace: "{}".to_owned()
            }
        );
    }

    #[test]
    fn a_set_goes_out_and_comes_back_in() {
        let mine = vec![
            named(
                "Block ads",
                "ads.example.com",
                Action::Block { status: 403 },
            ),
            named(
                "Staging",
                "api.example.com/*",
                Action::MapRemote {
                    to: "https://staging.example.com".to_owned(),
                },
            ),
        ];
        let text = export_set(&mine).unwrap();
        assert!(text.starts_with("# Rules from tracewing."), "{text}");
        assert!(text.contains("format = \"tracewing-rules\""), "{text}");

        let mut theirs = TrafficRules {
            rules: vec![mine[0].clone()],
        };
        let imported = theirs.import_set(&text).unwrap();
        assert_eq!(
            imported,
            Imported {
                added: 1,
                already: 1,
                refused: vec![],
            }
        );
        assert_eq!(theirs.rules, mine);
    }

    #[test]
    fn a_rule_that_cannot_work_here_is_left_out_and_the_rest_come_in() {
        let text = r#"
format = "tracewing-rules"
version = 1

[[rules]]
name = "Fine"
url = "a.dev"
action = "block"
status = 404

[[rules]]
name = "No such thing"
url = "b.dev"
action = "teleport"

[[rules]]
name = "Relative"
url = "c.dev/*"
action = "map_local"
path = "mocks/c"
"#;
        let mut rules = TrafficRules::default();
        let imported = rules.import_set(text).unwrap();
        assert_eq!(imported.added, 1);
        assert_eq!(rules.rules[0].name, "Fine");
        let names: Vec<&str> = imported.refused.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["No such thing", "Relative"]);
        assert!(
            imported.refused[1].1.contains("full path"),
            "{:?}",
            imported.refused
        );
    }

    #[test]
    fn other_files_are_refused_whole() {
        let mut rules = TrafficRules::default();
        let not_ours = rules
            .import_set("format = \"something-else\"\n")
            .unwrap_err();
        assert!(format!("{not_ours:#}").contains("not a tracewing rules file"));
        let newer = rules
            .import_set("format = \"tracewing-rules\"\nversion = 9\n[[rules]]\nname = \"x\"\n")
            .unwrap_err();
        assert!(format!("{newer:#}").contains("newer"));
        assert!(rules.import_set("rules = []\n").is_err());
        assert!(rules.import_set("not toml at all [").is_err());
        // The app's own rules file, which has no header, will do.
        let own = toml::to_string(&TrafficRules {
            rules: vec![named("Own", "d.dev", Action::Block { status: 410 })],
        })
        .unwrap();
        assert_eq!(rules.import_set(&own).unwrap().added, 1);
    }

    fn rule(url: &str, action: Action) -> TrafficRule {
        TrafficRule {
            name: format!("rule {url}"),
            enabled: true,
            url: url.to_owned(),
            method: String::new(),
            action,
        }
    }

    fn block(url: &str) -> TrafficRule {
        rule(url, Action::Block { status: 403 })
    }

    fn fits(url: &str, target: &str) -> bool {
        let rules = TrafficRules {
            rules: vec![block(url)],
        };
        rules.plan("GET", target).end.is_some()
    }

    #[test]
    fn pauses_are_planned_and_saved() {
        let rules = TrafficRules {
            rules: vec![
                rule("api.dev/v1/*", Action::PauseRequest),
                rule("api.dev/*", Action::PauseResponse),
                rule("api.dev/*", Action::PauseRequest),
            ],
        };
        let plan = rules.plan("POST", "api.dev/v1/items");
        assert_eq!(
            plan.pause_request.as_deref(),
            Some("rule api.dev/v1/*"),
            "the first"
        );
        assert_eq!(plan.pause_response.as_deref(), Some("rule api.dev/*"));
        assert!(plan.end.is_none());
        let plan = rules.plan("GET", "api.dev/other");
        assert_eq!(plan.pause_request.as_deref(), Some("rule api.dev/*"));
        assert!(rules.plan("GET", "else.dev/").pause_response.is_none());

        let text = toml::to_string(&rules).unwrap();
        assert!(text.contains(r#"action = "pause_request""#), "{text}");
        assert_eq!(toml::from_str::<TrafficRules>(&text).unwrap(), rules);
    }

    #[test]
    fn a_sockets_messages_are_paused_each_way() {
        let rules = TrafficRules {
            rules: vec![
                rule("chat.dev/*", Action::PauseMessagesToServer),
                rule("chat.dev/live", Action::PauseMessagesToApp),
            ],
        };
        let plan = rules.plan("GET", "chat.dev/live");
        assert_eq!(plan.pause_to_server.as_deref(), Some("rule chat.dev/*"));
        assert_eq!(plan.pause_to_app.as_deref(), Some("rule chat.dev/live"));
        assert!(plan.pause_request.is_none(), "not its upgrade");
        let plan = rules.plan("GET", "chat.dev/other");
        assert!(plan.pause_to_app.is_none());

        let text = toml::to_string(&rules).unwrap();
        assert!(
            text.contains(r#"action = "pause_messages_to_server""#),
            "{text}"
        );
        assert_eq!(toml::from_str::<TrafficRules>(&text).unwrap(), rules);
    }

    #[test]
    fn patterns() {
        assert!(fits("api.dev", "api.dev/"));
        assert!(fits("api.dev", "api.dev/v1/me?x=1"));
        assert!(fits("https://API.dev/v1/*", "api.dev/v1/me"));
        assert!(fits("*.dev/v1/*", "api.dev/v1/me"));
        assert!(fits("api.dev/*/me", "api.dev/v1/v2/me"));
        assert!(fits("api.dev/v1/me", "API.DEV/V1/ME"));
        assert!(!fits("api.dev", "api.dev.evil.com/"));
        assert!(!fits("api.dev/v1/me", "api.dev/v1/me2"));
        assert!(!fits("api.dev/v1/*", "api.dev/v2/me"));
        assert!(!fits("other.dev", "api.dev/"));
    }

    #[test]
    fn a_pattern_cannot_be_made_slow() {
        let target = format!("{}b", "a".repeat(4000));
        let pattern = format!("{}c", "*a".repeat(40));
        let started = std::time::Instant::now();
        assert!(!glob_fits(pattern.as_bytes(), target.as_bytes()));
        assert!(
            started.elapsed().as_millis() < 200,
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn method_and_switch() {
        let mut only_post = block("api.dev");
        only_post.method = "post".to_owned();
        let rules = TrafficRules {
            rules: vec![only_post.clone()],
        };
        assert!(rules.plan("POST", "api.dev/").end.is_some());
        assert!(rules.plan("GET", "api.dev/").end.is_none());
        only_post.enabled = false;
        let rules = TrafficRules {
            rules: vec![only_post],
        };
        assert!(rules.plan("POST", "api.dev/").end.is_none());
    }

    #[test]
    fn the_first_end_wins_and_headers_all_apply() {
        let rules = TrafficRules {
            rules: vec![
                rule(
                    "api.dev",
                    Action::SetRequestHeader {
                        header: "x-a".into(),
                        value: "1".into(),
                    },
                ),
                block("api.dev"),
                rule(
                    "api.dev",
                    Action::MapRemote {
                        to: "http://localhost:3000".into(),
                    },
                ),
                rule(
                    "api.dev",
                    Action::SetResponseHeader {
                        header: "x-b".into(),
                        value: "2".into(),
                    },
                ),
            ],
        };
        let plan = rules.plan("GET", "api.dev/");
        assert!(matches!(plan.end, Some(End::Block(403))));
        assert_eq!(plan.request_headers.len(), 1);
        assert_eq!(plan.response_headers.len(), 1);
        assert_eq!(plan.names.len(), 3, "the rule that lost touched nothing");
    }

    #[test]
    fn nothing_outside_the_folder_can_be_reached() {
        let dir = std::env::temp_dir().join(format!("local-{}", std::process::id()));
        let root = dir.join("root");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("index.html"), "home").unwrap();
        std::fs::write(root.join("sub/a.json"), "{}").unwrap();
        std::fs::write(dir.join("secret"), "no").unwrap();

        assert_eq!(
            local_file(&root, "/sub/a.json"),
            root.join("sub/a.json").canonicalize().ok()
        );
        assert_eq!(
            local_file(&root, ""),
            root.join("index.html").canonicalize().ok()
        );
        for bad in [
            "../secret",
            "/../secret",
            "sub/../../secret",
            "sub\\..\\..\\secret",
            "nope.txt",
            "sub",
        ] {
            assert_eq!(local_file(&root, bad), None, "{bad}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("secret"), root.join("link")).unwrap();
            assert_eq!(local_file(&root, "link"), None, "out by a link");
        }
        // A file as the rule's path: whatever the URL says.
        let file = root.join("sub/a.json");
        assert_eq!(local_file(&file, "/anything/../at/all"), Some(file));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_cannot_work_is_refused() {
        for (bad, why) in [
            (rule("a.dev", Action::Block { status: 7 }), "not a status"),
            (
                rule(
                    "a.dev",
                    Action::MapLocal {
                        path: "rel/path".into(),
                    },
                ),
                "full path",
            ),
            (
                rule(
                    "a.dev",
                    Action::MapRemote {
                        to: "ftp://x".into(),
                    },
                ),
                "needs to be like",
            ),
            (
                rule(
                    "a.dev",
                    Action::MapRemote {
                        to: "no host".into(),
                    },
                ),
                "not a URL",
            ),
            (
                rule(
                    "a.dev",
                    Action::SetRequestHeader {
                        header: "bad name".into(),
                        value: "v".into(),
                    },
                ),
                "name",
            ),
            (block(" "), "a URL"),
        ] {
            let said = format!("{:#}", bad.check().unwrap_err());
            assert!(said.contains(why), "{said}");
        }
        assert!(block("a.dev").check().is_ok());
    }

    #[test]
    fn rules_go_to_a_file_and_back() {
        let dir = std::env::temp_dir().join(format!("rules-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rules.toml");
        assert!(TrafficRules::load(&path).unwrap().is_empty());
        let rules = TrafficRules {
            rules: vec![
                block("a.dev"),
                rule(
                    "b.dev/*",
                    Action::MapLocal {
                        path: "/tmp/x".into(),
                    },
                ),
                rule(
                    "c.dev",
                    Action::SetResponseHeader {
                        header: "x-a".into(),
                        value: "1".into(),
                    },
                ),
            ],
        };
        rules.save(&path).unwrap();
        assert_eq!(TrafficRules::load(&path).unwrap(), rules);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
