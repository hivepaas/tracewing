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
    pub request_headers: Vec<(HeaderName, HeaderValue)>,
    pub response_headers: Vec<(HeaderName, HeaderValue)>,
    /// Which rules touched it, to show on the flow.
    pub names: Vec<String>,
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

fn remote_uri(to: &str) -> Result<Uri> {
    let uri: Uri = to
        .trim()
        .parse()
        .with_context(|| format!("\"{to}\" is not a URL"))?;
    if uri.authority().is_none() || !matches!(uri.scheme_str(), Some("http" | "https")) {
        bail!("\"{to}\" needs to be like https://host or http://localhost:3000");
    }
    Ok(uri)
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
            Action::MapLocal { path } => {
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
        }
        Ok(())
    }

    fn fits(&self, method: &str, target: &str, pattern: &str) -> bool {
        self.enabled
            && (self.method.trim().is_empty() || self.method.trim().eq_ignore_ascii_case(method))
            && glob_fits(pattern.as_bytes(), target.as_bytes())
    }
}

impl TrafficRules {
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
                    .map(|pair| plan.request_headers.push(pair))
                    .is_ok(),
                Action::SetResponseHeader { header, value } => header_pair(header, value)
                    .map(|pair| plan.response_headers.push(pair))
                    .is_ok(),
                _ => false,
            };
            if touched {
                plan.names.push(rule.name.clone());
            }
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
