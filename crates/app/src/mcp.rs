//! An MCP server, so that AI tools can look at what tracewing recorded:
//! `tracewing mcp`, which an AI tool starts and talks to over stdin and stdout,
//! a JSON-RPC message a line.
//!
//! It reads the sessions on the disk, the one recording now among them, with
//! the same filter as the window's box. Tokens and cookies are left out
//! unless Settings say an AI tool may see them; the tool cannot ask for them
//! itself. Only if Settings allow that too may it add and remove rules, and
//! send requests through the running proxy, which records them.

use crate::compose;
use tracewing_proxy::traffic_rules::{Action, TrafficRule, TrafficRules};

use crate::{
    detail::{BodyFacts, Detail, Side},
    devices::{Devices, device_of},
    filter::Filter,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
use tracewing_store::{
    binjson,
    prost_reflect::MessageDescriptor,
    session::{self, BodyReader},
    settings::StorageSettings,
};

/// The newest first: the one answered with, unless the client asks for one
/// of the others.
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const LIMIT_DEFAULT: u64 = 50;
const LIMIT_MAX: u64 = 500;
const BODY_DEFAULT: u64 = 16 * 1024;
const BODY_MAX: u64 = 1 << 20;
/// Headers whose values are left out, as an exported HAR leaves them out.
const SECRET_HEADERS: [&str; 4] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
];
const REMOVED: &str = "(removed)";
/// How many fields or lines of a body `compare_flows` gives, unless asked.
const CHANGES_DEFAULT: u64 = 100;
const CHANGES_MAX: u64 = 2_000;

const INSTRUCTIONS: &str = "tracewing is an HTTP(S) debugging proxy. These tools read the \
traffic it recorded: list_sessions, then list_flows to find requests, with the same filter \
as tracewing's search box, then get_flow for one request's headers, bodies and timing, \
or compare_flows for what differs between two, as between a call that failed and one \
that did not. \
Values of credential headers and cookies are removed unless the user allowed them in \
tracewing's Settings. \
If the user allowed it in Settings too, add_rule and remove_rule change tracewing's rules \
(block, mock from a file, send elsewhere, set headers, pause...), which apply to the next \
request, and send_request sends a request through tracewing, a recorded one again or \
changed, and gives its answer. list_rules shows the rules.";

/// Runs until stdin closes.
pub fn serve(data_dir: &Path) -> Result<()> {
    let server = Server::new(data_dir);
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lines() {
        let line = line.context("read from the client")?;
        if line.trim().is_empty() {
            continue;
        }
        for reply in server.handle_line(&line) {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

pub struct Server {
    data_dir: PathBuf,
    /// The `.proto` files chosen in the app, which name Protobuf's fields.
    protos: std::sync::Arc<tracewing_store::schema::Library>,
}

impl Server {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_owned(),
            protos: tracewing_store::schema::Library::open(data_dir),
        }
    }

    /// The replies to one line: none for a notification, one for a request,
    /// and one each for a batch's requests.
    pub fn handle_line(&self, line: &str) -> Vec<Value> {
        match serde_json::from_str::<Value>(line) {
            Ok(Value::Array(batch)) => batch.iter().filter_map(|m| self.handle(m)).collect(),
            Ok(message) => self.handle(&message).into_iter().collect(),
            Err(e) => vec![json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("not JSON: {e}") },
            })],
        }
    }

    fn handle(&self, message: &Value) -> Option<Value> {
        // A reply from the client, to nothing this asked: there is nothing to say.
        let method = message.get("method")?.as_str()?;
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = match method {
            "initialize" => Ok(initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.call(&params),
            _ => Err((-32601, format!("there is no method {method}"))),
        };
        // A notification has no id, and gets no answer.
        let id = message.get("id")?.clone();
        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": code, "message": message },
            }),
        })
    }

    fn call(&self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, "a tool's name is needed".to_owned()))?;
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let outcome = match name {
            "list_sessions" => self.list_sessions(),
            "list_flows" => self.list_flows(&args),
            "get_flow" => self.get_flow(&args),
            "compare_flows" => self.compare_flows(&args),
            "list_rules" => self.list_rules(),
            "add_rule" => self.add_rule(&args),
            "remove_rule" => self.remove_rule(&args),
            "send_request" => self.send_request(&args),
            _ => return Err((-32602, format!("there is no tool {name}"))),
        };
        Ok(match outcome {
            Ok(value) => json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default(),
                }],
                "structuredContent": value,
                "isError": false,
            }),
            Err(e) => json!({
                "content": [{ "type": "text", "text": format!("{e:#}") }],
                "isError": true,
            }),
        })
    }

    fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }

    /// Read at each call, so that a change in Settings counts at once.
    fn secrets_allowed(&self) -> bool {
        StorageSettings::load(&self.data_dir.join("settings.toml"))
            .is_ok_and(|settings| settings.ai_secrets)
    }

    /// Whether Settings let an AI tool change rules and send requests, read
    /// at each call.
    fn may_act(&self) -> Result<()> {
        let allowed = StorageSettings::load(&self.data_dir.join("settings.toml"))
            .is_ok_and(|settings| settings.ai_actions);
        if !allowed {
            bail!(
                "tracewing's Settings do not let AI tools change rules or send requests. \
                 The user can allow it under AI tools"
            );
        }
        Ok(())
    }

    fn rules_path(&self) -> PathBuf {
        self.data_dir.join("traffic-rules.toml")
    }

    /// The session asked for by its id, or the newest.
    fn session(&self, args: &Value) -> Result<PathBuf> {
        let sessions = session::list(&self.sessions_dir());
        let chosen = match args.get("session").and_then(Value::as_str) {
            Some(id) => sessions
                .into_iter()
                .find(|s| session_id(&s.dir) == id)
                .with_context(|| format!("there is no session {id}"))?,
            None => sessions
                .into_iter()
                .next()
                .context("tracewing has recorded no session yet")?,
        };
        if !chosen.readable {
            bail!(
                "session {} cannot be read by this version",
                session_id(&chosen.dir)
            );
        }
        Ok(chosen.dir)
    }

    fn list_sessions(&self) -> Result<Value> {
        let sessions: Vec<Value> = session::list(&self.sessions_dir())
            .into_iter()
            .enumerate()
            .map(|(at, s)| {
                json!({
                    "id": session_id(&s.dir),
                    "name": s.name,
                    "newest": at == 0,
                    "flows": s.flows,
                    "first": s.first_us.map(time_text),
                    "last": s.last_us.map(time_text),
                    "readable": s.readable,
                })
            })
            .collect();
        Ok(json!({ "sessions": sessions }))
    }

    fn list_flows(&self, args: &Value) -> Result<Value> {
        let dir = self.session(args)?;
        let conn = open(&dir)?;
        let devices = Devices::load(&self.data_dir.join("devices.toml"));
        let filter = args.get("filter").and_then(Value::as_str).unwrap_or("");
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(LIMIT_DEFAULT)
            .clamp(1, LIMIT_MAX) as usize;
        let before = args.get("before").and_then(Value::as_i64);
        let mut ids = Filter::parse_with(filter, &devices).find(&conn, &dir)?;
        ids.sort_unstable();
        // The newest first, as someone asking about traffic wants it.
        let ids: Vec<i64> = ids
            .into_iter()
            .rev()
            .filter(|id| before.is_none_or(|before| *id < before))
            .take(limit + 1)
            .collect();
        let more = ids.len() > limit;
        let shown = &ids[..ids.len().min(limit)];
        let flows = rows(&conn, shown, &devices)?;
        Ok(json!({
            "session": session_id(&dir),
            "flows": flows,
            "more": more,
            "next_before": more.then(|| shown.last().copied()).flatten(),
        }))
    }

    fn get_flow(&self, args: &Value) -> Result<Value> {
        let dir = self.session(args)?;
        let id = args
            .get("id")
            .and_then(Value::as_i64)
            .context("get_flow needs the flow's id, as list_flows gives it")?;
        let conn = open(&dir)?;
        let detail = Detail::load(&conn, &dir, id)
            .with_context(|| format!("there is no flow {id} in this session"))?;
        let bodies = args.get("bodies").and_then(Value::as_bool).unwrap_or(true);
        let max = args
            .get("max_body_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(BODY_DEFAULT)
            .min(BODY_MAX);
        let secrets = self.secrets_allowed();
        let mut reader = BodyReader::new(&dir);
        let mut side = |side: Side| {
            let facts = detail.body(side);
            let headers = detail.headers(side).map(|headers| {
                headers
                    .iter()
                    .chain(detail.trailers(side))
                    .map(|(name, value)| {
                        let secret = SECRET_HEADERS.contains(&name.to_ascii_lowercase().as_str());
                        let value = if secret && !secrets { REMOVED } else { value };
                        json!([name, value])
                    })
                    .collect::<Vec<_>>()
            });
            let content_type = match side {
                Side::Request => detail.req_content_type.clone(),
                Side::Response => detail.content_type.clone(),
            };
            let (body, note) = if bodies {
                let message = crate::detail::message_of(
                    &self.protos,
                    &detail.url,
                    content_type.as_deref(),
                    side,
                );
                body_text(
                    &mut reader,
                    facts,
                    max,
                    content_type.as_deref(),
                    message.as_ref(),
                )
            } else {
                (None, None)
            };
            json!({
                "headers": headers,
                "content_type": content_type,
                "bytes": facts.bytes,
                "body": body,
                "body_note": note,
            })
        };
        let request = side(Side::Request);
        let response = side(Side::Response);
        let ms = |us: Option<i64>| us.map(|us| us as f64 / 1000.0);
        Ok(json!({
            "session": session_id(&dir),
            "id": detail.id,
            "method": detail.method,
            "url": detail.url,
            "kind": detail.kind,
            "status": detail.status,
            "grpc_status": detail.grpc_status,
            "grpc_message": detail.grpc_message(),
            "error": detail.error,
            "still_open": detail.open,
            "client_version": detail.client_version,
            "server_version": detail.server_version,
            "client": detail.client,
            "app": detail.app,
            "device": detail.app.is_none().then(|| device_of(&detail.client)).flatten(),
            "graphql": detail.graphql,
            "rpc": detail.rpc,
            "rpc_error": detail.rpc_error,
            "changed_by_rules": detail.rule,
            "timing_ms": {
                "dns": ms(detail.dns_us),
                "connect": ms(detail.connect_us),
                "tls": ms(detail.tls_us),
                "first_byte": ms(detail.ttfb_us),
                "total": ms(detail.duration_us),
            },
            "secrets_removed": !secrets,
            "request": request,
            "response": response,
        }))
    }
}

/// What an AI tool's rule is called, so that the user can tell it from theirs.
const RULE_MARK: &str = "AI: ";

impl Server {
    fn list_rules(&self) -> Result<Value> {
        let rules = TrafficRules::load(&self.rules_path())?;
        let listed: Vec<Value> = rules.rules.iter().map(rule_json).collect();
        Ok(json!({ "rules": listed }))
    }

    fn add_rule(&self, args: &Value) -> Result<Value> {
        self.may_act()?;
        let text = |key: &str| args.get(key).and_then(Value::as_str).map(str::trim);
        let name = text("name").context("add_rule needs a name for the rule")?;
        let url = text("url").context("add_rule needs the URL pattern it matches")?;
        let action: Action = serde_json::from_value(
            args.get("action")
                .cloned()
                .context("add_rule needs what the rule does, as `action`")?,
        )
        .context("`action` is not one tracewing knows; see the tool's description")?;
        let name = if name.starts_with(RULE_MARK) {
            name.to_owned()
        } else {
            format!("{RULE_MARK}{name}")
        };
        let rule = TrafficRule {
            name,
            enabled: true,
            url: url.to_owned(),
            method: text("method").unwrap_or_default().to_owned(),
            action,
        };
        rule.check()?;
        let path = self.rules_path();
        let mut rules = TrafficRules::load(&path)?;
        if rules.rules.iter().any(|r| r.name == rule.name) {
            bail!("there is a rule called \"{}\" already", rule.name);
        }
        rules.rules.push(rule.clone());
        rules.save(&path)?;
        Ok(json!({
            "added": rule_json(&rule),
            "note": "tracewing applies it from the next request; its Rules screen lists it",
        }))
    }

    fn remove_rule(&self, args: &Value) -> Result<Value> {
        self.may_act()?;
        let name = args
            .get("name")
            .and_then(Value::as_str)
            .context("remove_rule needs the rule's name, as list_rules gives it")?;
        let path = self.rules_path();
        let mut rules = TrafficRules::load(&path)?;
        let before = rules.rules.len();
        rules.rules.retain(|rule| rule.name != name);
        if rules.rules.len() == before {
            bail!("there is no rule called \"{name}\"");
        }
        rules.save(&path)?;
        Ok(json!({ "removed": name }))
    }

    /// Sends a request through the running proxy, which records it: a flow
    /// again, or one written here, changed as `args` say.
    fn send_request(&self, args: &Value) -> Result<Value> {
        self.may_act()?;
        let mut message = match args.get("id").and_then(Value::as_i64) {
            Some(id) => {
                let dir = self.session(args)?;
                let conn = open(&dir)?;
                let detail = Detail::load(&conn, &dir, id)
                    .with_context(|| format!("there is no flow {id} in this session"))?;
                let facts = &detail.request;
                let body = facts
                    .stored
                    .exists()
                    .then(|| {
                        BodyReader::new(&dir)
                            .unpacked(&facts.stored, tracewing_proxy::pause::HOLD_MAX)
                    })
                    .flatten();
                let whole = !facts.shed
                    && !facts.lost
                    && (facts.bytes == 0 || body.is_some())
                    && facts.bytes as usize <= tracewing_proxy::pause::HOLD_MAX;
                compose::from_flow(
                    &detail.method,
                    &detail.url,
                    detail.headers(Side::Request).unwrap_or_default(),
                    body,
                    whole,
                )?
            }
            None => tracewing_proxy::pause::Message {
                method: "GET".to_owned(),
                url: String::new(),
                status: 0,
                headers: Vec::new(),
                body: Vec::new(),
            },
        };
        if let Some(method) = args.get("method").and_then(Value::as_str) {
            message.method = method.trim().to_ascii_uppercase();
        }
        if let Some(url) = args.get("url").and_then(Value::as_str) {
            message.url = url.trim().to_owned();
        }
        if message.url.is_empty() {
            bail!("send_request needs a flow's `id` to send again, or a `url`");
        }
        if let Some(headers) = args.get("headers") {
            let headers = headers
                .as_object()
                .context("`headers` is an object of names, each a value or null")?;
            for (name, value) in headers {
                message
                    .headers
                    .retain(|(had, _)| !had.eq_ignore_ascii_case(name));
                match value {
                    Value::Null => {}
                    Value::String(value) => message.headers.push((name.clone(), value.clone())),
                    other => message.headers.push((name.clone(), other.to_string())),
                }
            }
        }
        if let Some(body) = args.get("body") {
            message.body = match body {
                Value::String(text) => text.clone().into_bytes(),
                Value::Null => Vec::new(),
                // JSON written as JSON is sent as its text.
                other => other.to_string().into_bytes(),
            };
        }
        let address =
            std::fs::read_to_string(self.data_dir.join(crate::engine::PROXY_ADDRESS_FILE))
                .ok()
                .and_then(|text| text.trim().parse::<std::net::SocketAddr>().ok())
                .context("tracewing is not running, so there is nothing to send through")?;
        let (_keep, stop) = compose::stopper();
        // As agreed: an AI tool has no say over the version, as it has none
        // over the proxy's leg to the server.
        let answer = compose::send(
            address,
            &self.data_dir.join("ca.pem"),
            &message,
            compose::Speak::Agreed,
            stop,
        )?
        .context("the request was stopped")?;
        let secrets = self.secrets_allowed();
        let headers: Vec<Value> = answer
            .headers
            .iter()
            .map(|(name, value)| {
                let secret = SECRET_HEADERS.contains(&name.to_ascii_lowercase().as_str());
                json!([name, if secret && !secrets { REMOVED } else { value }])
            })
            .collect();
        let max = args
            .get("max_body_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(BODY_DEFAULT)
            .min(BODY_MAX) as usize;
        let shown = &answer.body[..answer.body.len().min(max)];
        let (body, note) = match std::str::from_utf8(shown) {
            Ok(text) => (
                Some(text.to_owned()),
                (answer.body.len() > max)
                    .then(|| format!("only the first {max} bytes; {} came", answer.bytes)),
            ),
            Err(e) if e.error_len().is_none() => (
                Some(String::from_utf8_lossy(&shown[..e.valid_up_to()]).into_owned()),
                Some(format!(
                    "only the first {} bytes; {} came",
                    e.valid_up_to(),
                    answer.bytes
                )),
            ),
            Err(_) => (None, Some(format!("binary, {} bytes", answer.bytes))),
        };
        Ok(json!({
            "method": message.method,
            "url": message.url,
            "status": answer.status,
            "version": answer.version,
            "took_ms": answer.took.as_secs_f64() * 1000.0,
            "headers": headers,
            "body": body,
            "body_note": note,
            "recorded": "tracewing recorded it in the session it records now: list_flows finds it",
        }))
    }

    fn compare_flows(&self, args: &Value) -> Result<Value> {
        use crate::compare::{self, Change, Pair};
        let dir = self.session(args)?;
        let conn = open(&dir)?;
        let id_a = args
            .get("a")
            .and_then(Value::as_i64)
            .context("compare_flows needs `a`, a flow's id as list_flows gives it")?;
        let a = Detail::load(&conn, &dir, id_a)
            .with_context(|| format!("there is no flow {id_a} in this session"))?;
        let id_b = match args.get("b").and_then(Value::as_i64) {
            Some(id) => id,
            None => crate::compare_page::previous_call(&conn, &a).with_context(|| {
                format!("no call before flow {id_a} to the same method, host and path: give `b`")
            })?,
        };
        let b = Detail::load(&conn, &dir, id_b)
            .with_context(|| format!("there is no flow {id_b} in this session"))?;
        let max = args
            .get("max_changes")
            .and_then(Value::as_u64)
            .unwrap_or(CHANGES_DEFAULT)
            .clamp(1, CHANGES_MAX) as usize;
        let secrets = self.secrets_allowed();
        let change_name = |change: Change| match change {
            Change::Same => "same",
            Change::Added => "only_b",
            Change::Removed => "only_a",
            Change::Changed => "changed",
        };
        let shown_pairs = |pairs: Vec<Pair>, headers: bool| -> Vec<Value> {
            pairs
                .into_iter()
                .filter(|p| p.change() != Change::Same)
                .map(|p| {
                    let secret = headers
                        && !secrets
                        && SECRET_HEADERS.contains(&p.name.to_ascii_lowercase().as_str());
                    let value =
                        |v: Option<String>| v.map(|v| if secret { REMOVED.to_owned() } else { v });
                    json!({
                        "name": p.name,
                        "change": change_name(p.change()),
                        "a": value(p.a),
                        "b": value(p.b),
                    })
                })
                .collect()
        };
        let side = |side: Side| {
            let of = |d: &Detail| {
                d.headers(side)
                    .unwrap_or_default()
                    .iter()
                    .chain(d.trailers(side))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let headers = shown_pairs(compare::pairs(&of(&a), &of(&b), true), true);
            let body = match (
                crate::compare_page::read_body(&dir, &self.protos, &a, side),
                crate::compare_page::read_body(&dir, &self.protos, &b, side),
            ) {
                (Some(mut ta), Some(mut tb)) => {
                    let kinds = json!({ "a": ta.kind, "b": tb.kind });
                    let cut = ta.cut || tb.cut;
                    match (ta.json.take(), tb.json.take()) {
                        (Some(ja), Some(jb)) => {
                            let tree = compare::diff_tree(&ja, &jb);
                            let fields = compare::field_paths(&tree);
                            let count = fields.len();
                            let listed: Vec<Value> = fields
                                .into_iter()
                                .take(max)
                                .map(|(path, node)| {
                                    json!({
                                        "path": path,
                                        "change": change_name(node.change),
                                        "a": node.a,
                                        "b": node.b,
                                    })
                                })
                                .collect();
                            json!({
                                "kind": kinds,
                                "same": count == 0,
                                "fields_differ": count,
                                "fields": listed,
                                "more": count > max,
                                "cut_to_first_bytes": cut.then_some(compare::BODY_MAX),
                            })
                        }
                        _ => {
                            let lines = compare::diff_lines(&ta.lines, &tb.lines);
                            let changed: Vec<_> =
                                lines.iter().filter(|l| l.change != Change::Same).collect();
                            let count = changed.len();
                            let listed: Vec<Value> = changed
                                .into_iter()
                                .take(max)
                                .map(|l| {
                                    json!({
                                        "change": change_name(l.change),
                                        "a_line": l.a.map(|n| n + 1),
                                        "b_line": l.b.map(|n| n + 1),
                                        "a": l.a.and_then(|n| ta.lines.get(n)),
                                        "b": l.b.and_then(|n| tb.lines.get(n)),
                                    })
                                })
                                .collect();
                            json!({
                                "kind": kinds,
                                "same": count == 0,
                                "lines_differ": count,
                                "lines": listed,
                                "more": count > max,
                                "cut_to_first_bytes": cut.then_some(compare::BODY_MAX),
                            })
                        }
                    }
                }
                _ => json!({ "note": "a body was not kept, so the two cannot be compared" }),
            };
            json!({ "headers": headers, "body": body })
        };
        let size = |d: &Detail| json!({ "request": d.request.bytes, "response": d.response.bytes });
        let mut summary = Vec::new();
        for (field, va, vb) in [
            ("method", json!(a.method), json!(b.method)),
            ("url", json!(a.url), json!(b.url)),
            ("status", json!(a.status), json!(b.status)),
            ("grpc_status", json!(a.grpc_status), json!(b.grpc_status)),
            ("error", json!(a.error), json!(b.error)),
            ("protocols", json!(a.protocols()), json!(b.protocols())),
            (
                "request_type",
                json!(a.req_content_type),
                json!(b.req_content_type),
            ),
            (
                "response_type",
                json!(a.content_type),
                json!(b.content_type),
            ),
            ("bytes", size(&a), size(&b)),
            (
                "total_ms",
                json!(a.duration_us.map(|us| us as f64 / 1000.0)),
                json!(b.duration_us.map(|us| us as f64 / 1000.0)),
            ),
            ("app", json!(a.app), json!(b.app)),
        ] {
            if va != vb {
                summary.push(json!({ "field": field, "a": va, "b": vb }));
            }
        }
        Ok(json!({
            "session": session_id(&dir),
            "a": id_a,
            "b": id_b,
            "b_is_the_call_before": args.get("b").is_none(),
            "differs": summary,
            "query": shown_pairs(
                compare::pairs(&compare::query_of(&a.url), &compare::query_of(&b.url), false),
                false,
            ),
            "request": side(Side::Request),
            "response": side(Side::Response),
            "secrets_removed": !secrets,
        }))
    }
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|v| PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "tracewing", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "list_sessions",
            "description": "The sessions tracewing has recorded or imported, the newest first. \
                The newest is usually the one recording now.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "list_flows",
            "description": "Requests in a session, the newest first. `filter` is tracewing's \
                search box: host:api.example.com, method:POST, status:404, status:5xx, \
                type:json, app:chrome, device:phone, graphql:GetUser, rpc:eth_call, grpc:error, \
                grpc:NOT_FOUND, \
                and plain words to find in URLs and bodies. Page back with `before` set to `next_before`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": { "type": "string", "description": "A session's id; the newest if left out." },
                    "filter": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": LIMIT_MAX, "default": LIMIT_DEFAULT },
                    "before": { "type": "integer", "description": "Only flows with an id below this." },
                },
            },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "get_flow",
            "description": "One request and its response: headers, bodies as text (unpacked; \
                binary bodies are described, not given), timing, the connection, and any error.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": { "type": "string", "description": "A session's id; the newest if left out." },
                    "id": { "type": "integer" },
                    "bodies": { "type": "boolean", "default": true },
                    "max_body_bytes": { "type": "integer", "maximum": BODY_MAX, "default": BODY_DEFAULT },
                },
                "required": ["id"],
            },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "compare_flows",
            "description": "What differs between two requests and their responses: method, URL, \
                status, sizes and time; the query and headers, by name; and the bodies, JSON \
                (and MessagePack, CBOR, Protobuf) field by field as paths like items[3].price, \
                other text line by line. Only what differs is given. Leave `b` out to compare \
                `a` with the call before it to the same method, host and path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": { "type": "string", "description": "A session's id; the newest if left out." },
                    "a": { "type": "integer", "description": "The first flow's id." },
                    "b": { "type": "integer", "description": "The second's; the call before `a` if left out." },
                    "max_changes": { "type": "integer", "minimum": 1, "maximum": CHANGES_MAX, "default": CHANGES_DEFAULT },
                },
                "required": ["a"],
            },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "list_rules",
            "description": "tracewing's rules, in the order they apply, each with what it does as \
                add_rule takes it.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "add_rule",
            "description": "Adds a rule, which applies to the next request whose URL fits `url`: \
                api.example.com/v1/* (`*` is anything; no scheme). Its name gets \"AI: \" in front. \
                `action` is an object with an `action` field and its own, one of: \
                {\"action\":\"block\",\"status\":403}, \
                {\"action\":\"map_local\",\"path\":\"/full/path/to/file.json\"}, \
                {\"action\":\"map_remote\",\"to\":\"http://localhost:3000\"}, \
                {\"action\":\"set_request_header\",\"header\":\"x-a\",\"value\":\"1\"}, \
                {\"action\":\"set_response_header\",\"header\":\"x-a\",\"value\":\"1\"}, \
                {\"action\":\"remove_request_header\",\"header\":\"x-a\"}, \
                {\"action\":\"remove_response_header\",\"header\":\"x-a\"}, \
                {\"action\":\"set_status\",\"status\":500}, \
                {\"action\":\"set_query\",\"name\":\"page\",\"value\":\"2\"}, \
                {\"action\":\"remove_query\",\"name\":\"page\"}, \
                {\"action\":\"replace_request_body\",\"find\":\"a\",\"replace\":\"b\"} \
                (an empty `find` replaces the whole body), the same for replace_response_body, \
                {\"action\":\"pause_request\"}, {\"action\":\"pause_response\"}, \
                {\"action\":\"pause_messages_to_server\"}, {\"action\":\"pause_messages_to_app\"} \
                (these hold traffic on tracewing's Paused screen until the user lets it go), \
                {\"action\":\"script\",\"path\":\"/full/path/to/script.js\"} (runs its \
                onRequest(request) and onResponse(request, response) on matching traffic), \
                {\"action\":\"ask_server_with\",\"version\":\"http1\"} or \"http2\" (which \
                version tracewing asks that server with; `url` must be a host with no path, \
                and a cleartext connection is HTTP/1.1 whatever it says). \
                Only if the user allowed AI tools to act, in Settings.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "url": { "type": "string" },
                    "method": { "type": "string", "description": "Only this method; every method if left out." },
                    "action": { "type": "object" },
                },
                "required": ["name", "url", "action"],
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false },
        },
        {
            "name": "remove_rule",
            "description": "Removes the rule of that name, as list_rules gives it. Only if the \
                user allowed AI tools to act, in Settings.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"],
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": true, "idempotentHint": true },
        },
        {
            "name": "send_request",
            "description": "Sends a request through tracewing, which records it and applies its \
                rules: the flow `id` again, or a new one to `url`. `method`, `url`, `headers` \
                (an object; a value sets a header in place of any of that name, null removes it) \
                and `body` (text) change what is sent. Gives the answer: status, headers, body as \
                text. It reaches the real server: do not send what changes data unless the user \
                asked. Only if the user allowed AI tools to act, in Settings, and tracewing is running.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": { "type": "string", "description": "The session of `id`; the newest if left out." },
                    "id": { "type": "integer", "description": "A recorded flow to send again." },
                    "method": { "type": "string" },
                    "url": { "type": "string" },
                    "headers": { "type": "object" },
                    "body": { "type": "string" },
                    "max_body_bytes": { "type": "integer", "maximum": BODY_MAX, "default": BODY_DEFAULT },
                },
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": true, "openWorldHint": true },
        },
    ])
}

/// A rule as an AI tool reads it, its action as `add_rule` takes it.
fn rule_json(rule: &TrafficRule) -> Value {
    json!({
        "name": rule.name,
        "enabled": rule.enabled,
        "url": rule.url,
        "method": rule.method,
        "action": serde_json::to_value(&rule.action).unwrap_or(Value::Null),
    })
}

fn session_id(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn open(dir: &Path) -> Result<Connection> {
    let path = session::flows_db(dir);
    Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("open {}", path.display()))
}

fn time_text(us: i64) -> String {
    chrono::DateTime::from_timestamp_micros(us)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

/// The flows with these ids, in this order.
fn rows(conn: &Connection, ids: &[i64], devices: &Devices) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, started_us, method, scheme, host, path, status, kind, resp_bytes,
                duration_us, app, client, error, graphql, rule, open, grpc_status, rpc, rpc_error
         FROM flows WHERE id = ?1",
    )?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let row = stmt.query_row([id], |r| {
            let scheme: String = r.get(3)?;
            let host: String = r.get(4)?;
            let path: String = r.get(5)?;
            let app: Option<String> = r.get(10)?;
            let client: String = r.get(11)?;
            let device = app
                .is_none()
                .then(|| device_of(&client))
                .flatten()
                .map(|ip| devices.name(&ip).map_or(ip.clone(), str::to_owned));
            Ok(json!({
                "id": r.get::<_, i64>(0)?,
                "started": time_text(r.get(1)?),
                "method": r.get::<_, String>(2)?,
                "url": format!("{scheme}://{host}{path}"),
                "status": r.get::<_, Option<u16>>(6)?,
                "grpc_status": r.get::<_, Option<u16>>(16)?,
                "rpc": r.get::<_, Option<String>>(17)?,
                "rpc_error": r.get::<_, Option<i64>>(18)?,
                "kind": r.get::<_, String>(7)?,
                "response_bytes": r.get::<_, i64>(8)?,
                "duration_ms": r.get::<_, Option<i64>>(9)?.map(|us| us as f64 / 1000.0),
                "app": app,
                "device": device,
                "error": r.get::<_, Option<String>>(12)?,
                "graphql": r.get::<_, Option<String>>(13)?,
                "changed_by_rules": r.get::<_, Option<String>>(14)?,
                "still_open": r.get::<_, bool>(15)?,
            }))
        });
        match row {
            Ok(row) => out.push(row),
            // Gone with the oldest flows since the ids were found.
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}

/// MessagePack, CBOR or Protobuf as the JSON it holds, cut at `max`, and a note that
/// says what it was. `None` for any other body, or one that cannot be read.
fn as_json(
    bytes: &[u8],
    content_type: Option<&str>,
    sent: u64,
    max: u64,
    message: Option<&MessageDescriptor>,
) -> Option<(String, String)> {
    let format = binjson::format_of(content_type)?;
    let json = binjson::to_json_as(bytes, format, message)?;
    let mut end = json.len().min(max as usize);
    while !json.is_char_boundary(end) {
        end -= 1;
    }
    let mut note = format!("{}, {sent} bytes sent, given as JSON", format.name());
    match message {
        Some(message) => {
            note.push_str(&format!(
                " as {} from the .proto files",
                message.full_name()
            ));
        }
        None if format.numbered() => note.push_str(" with fields by number"),
        None => {}
    }
    if end < json.len() {
        note.push_str(&format!("; only the first {end} bytes of {}", json.len()));
    }
    Some((json[..end].to_owned(), note))
}

/// A body as text, cut at `max`, and what was left out of it, if anything.
/// MessagePack, CBOR and Protobuf are given as the JSON they hold, and say so.
fn body_text(
    reader: &mut BodyReader,
    facts: &BodyFacts,
    max: u64,
    content_type: Option<&str>,
    message: Option<&MessageDescriptor>,
) -> (Option<String>, Option<String>) {
    if facts.bytes == 0 {
        return (None, None);
    }
    if !facts.stored.exists() {
        return (None, Some("not kept".to_owned()));
    }
    if binjson::format_of(content_type).is_some()
        && let Some(bytes) = reader.unpacked(&facts.stored, binjson::DECODE_MAX as usize)
        && let Some((json, note)) = as_json(&bytes, content_type, facts.bytes, max, message)
    {
        return (Some(json), Some(note));
    }
    let Some(bytes) = reader.unpacked(&facts.stored, max as usize + 1) else {
        return (None, Some("cannot be read".to_owned()));
    };
    let cut = bytes.len() as u64 > max;
    let kept = &bytes[..bytes.len().min(max as usize)];
    let text = match std::str::from_utf8(kept) {
        Ok(text) => text,
        // Cut in the middle of a character: as far as the last whole one.
        Err(e) if e.error_len().is_none() => {
            std::str::from_utf8(&kept[..e.valid_up_to()]).unwrap_or_default()
        }
        Err(_) => {
            return (None, Some(format!("binary, {} bytes sent", facts.bytes)));
        }
    };
    let note = (cut || facts.truncated).then(|| {
        format!(
            "only the first {} bytes; {} were sent",
            text.len(),
            facts.bytes
        )
    });
    (Some(text.to_owned()), note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messagepack_is_given_as_json_and_says_so() {
        let mut bytes = Vec::new();
        let value = rmpv::Value::Map(vec![("city".into(), "Hà Nội".into())]);
        rmpv::encode::write_value(&mut bytes, &value).unwrap();
        let ct = Some("application/msgpack");
        let (json, note) = as_json(&bytes, ct, 99, 1000, None).unwrap();
        assert_eq!(json, r#"{"city":"Hà Nội"}"#);
        assert_eq!(note, "MessagePack, 99 bytes sent, given as JSON");
        // Cut on a character's edge: "à" is two bytes, at 10 and 11.
        let (cut, note) = as_json(&bytes, ct, 99, 11, None).unwrap();
        assert_eq!(cut, r#"{"city":"H"#);
        assert!(note.ends_with("only the first 10 bytes of 20"), "{note}");
        assert_eq!(
            as_json(&bytes, Some("application/json"), 99, 1000, None),
            None
        );
        // Protobuf has no names for its fields with no schema.
        let (json, note) =
            as_json(b"\x08\x2a", Some("application/x-protobuf"), 2, 1000, None).unwrap();
        assert_eq!(json, r#"{"1":42}"#);
        assert_eq!(
            note,
            "Protobuf, 2 bytes sent, given as JSON with fields by number"
        );
    }

    fn ask(server: &Server, message: Value) -> Value {
        let mut replies = server.handle_line(&message.to_string());
        assert_eq!(replies.len(), 1, "{message}");
        replies.remove(0)
    }

    #[test]
    fn it_says_hello_in_the_version_asked() {
        let server = Server::new(Path::new("/nowhere"));
        let reply = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": { "protocolVersion": "2025-03-26", "capabilities": {},
                                "clientInfo": { "name": "t", "version": "1" } } }),
        );
        assert_eq!(reply["id"], 1);
        assert_eq!(reply["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(reply["result"]["serverInfo"]["name"], "tracewing");
        let unknown = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize",
                    "params": { "protocolVersion": "1999-01-01" } }),
        );
        assert_eq!(unknown["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn notifications_get_no_answer_and_unknown_methods_an_error() {
        let server = Server::new(Path::new("/nowhere"));
        assert!(
            server
                .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_empty()
        );
        let reply = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": "a", "method": "resources/list" }),
        );
        assert_eq!(reply["error"]["code"], -32601);
        let bad = server.handle_line("{not json");
        assert_eq!(bad[0]["error"]["code"], -32700);
        let tools = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
        );
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "list_sessions",
                "list_flows",
                "get_flow",
                "compare_flows",
                "list_rules",
                "add_rule",
                "remove_rule",
                "send_request"
            ]
        );
    }

    #[test]
    fn a_tool_that_fails_says_why_as_its_result() {
        let dir = std::env::temp_dir().join(format!("tracewing-mcp-{}", std::process::id()));
        let server = Server::new(&dir);
        let reply = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                    "params": { "name": "list_flows", "arguments": {} } }),
        );
        assert_eq!(reply["result"]["isError"], true);
        assert!(
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("no session yet")
        );
        let none = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call",
                    "params": { "name": "rm_rf", "arguments": {} } }),
        );
        assert_eq!(none["error"]["code"], -32602);
    }
}
