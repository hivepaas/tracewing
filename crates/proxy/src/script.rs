//! Scripts in JavaScript, run on requests and responses by a rule: a file
//! with `onRequest(request)` and `onResponse(request, response)`, either or
//! both. Each changes what it is given, or returns it changed; `onRequest`
//! may return `{ response: … }` to answer at once, with no server asked.
//!
//! ```js
//! function onRequest(request) {
//!   request.headers["x-debug"] = "1";
//! }
//! function onResponse(request, response) {
//!   const body = JSON.parse(response.body);
//!   body.flag = true;
//!   response.body = JSON.stringify(body);
//! }
//! ```
//!
//! A request is `{ method, url, headers, body }`, a response `{ status,
//! headers, body }`. Headers are by lower-case name, a string each, or an
//! array of them for a header given more than once. A body is text, or
//! `null` when it is not, which then goes on as it came unless set. A global
//! `state` keeps what a script puts in it from one call to the next, until
//! the file changes. `console.log` writes to the script's log, which the
//! Rules screen shows.
//!
//! Each file runs in QuickJS on a thread of its own, one call at a time, and
//! is read again when it changes. A call that takes more than a second is
//! stopped, and so is one that uses more than 64 MiB; what it was given then
//! goes on as it came.

use crate::pause::Message;
use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Runtime, Value, function::Rest};
use serde_json::{Map, Value as Json, json};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime},
};

/// The longest a call may take.
pub const TIME_LIMIT: Duration = Duration::from_secs(1);
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const STACK_LIMIT: usize = 1024 * 1024;
/// Lines of a script's log kept to show.
const LOG_KEPT: usize = 200;
/// The most of a `console.log` line kept.
const LINE_MAX: usize = 2000;

/// What a script made of a request.
#[derive(Debug, PartialEq, Eq)]
pub enum OnRequest {
    /// It has no `onRequest`, or left the request as it was.
    Unchanged,
    Changed(Message),
    /// Its answer, with no server asked.
    Answer(Message),
}

/// What a script's log says, for the Rules screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptLog {
    /// The newest last.
    pub lines: Vec<String>,
    /// Why the last call failed, if it did; cleared by one that does not.
    pub error: Option<String>,
    pub calls: u64,
}

enum Job {
    /// Which of the two the script has.
    Hooks(tokio::sync::oneshot::Sender<Result<(bool, bool), String>>),
    Call {
        function: &'static str,
        args: Vec<Json>,
        reply: tokio::sync::oneshot::Sender<Result<Option<Json>, String>>,
    },
}

/// The scripts rules name, each on its thread, made the first time it runs.
#[derive(Default)]
pub struct Scripts {
    threads: Mutex<HashMap<PathBuf, mpsc::Sender<Job>>>,
    logs: Arc<Mutex<HashMap<PathBuf, ScriptLog>>>,
    /// Goes up as a log changes, for whoever shows them to tell.
    version: Arc<AtomicU64>,
}

impl std::fmt::Debug for Scripts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scripts").finish_non_exhaustive()
    }
}

impl Scripts {
    /// Whether the script has `onRequest`, and `onResponse`. One that cannot
    /// be read or run has neither, and its log says why.
    pub async fn hooks(&self, path: &Path) -> (bool, bool) {
        let (reply, answer) = tokio::sync::oneshot::channel();
        if !self.send(path, Job::Hooks(reply)) {
            return (false, false);
        }
        match answer.await {
            Ok(Ok(hooks)) => hooks,
            Ok(Err(why)) => {
                self.failed(path, why);
                (false, false)
            }
            Err(_) => (false, false),
        }
    }

    /// Runs `onRequest` on `request`.
    pub async fn on_request(&self, path: &Path, request: &Message) -> OnRequest {
        let Some(out) = self
            .call(path, "onRequest", vec![request_json(request)])
            .await
        else {
            return OnRequest::Unchanged;
        };
        if let Some(answer) = out.get("response").filter(|r| r.is_object()) {
            return OnRequest::Answer(response_from(answer, &Message::default_response()));
        }
        let changed = request_from(&out, request);
        if changed == *request {
            OnRequest::Unchanged
        } else {
            OnRequest::Changed(changed)
        }
    }

    /// Runs `onResponse` on `response` to `request`: what goes on in its place.
    pub async fn on_response(
        &self,
        path: &Path,
        request: &Message,
        response: &Message,
    ) -> Option<Message> {
        let out = self
            .call(
                path,
                "onResponse",
                vec![request_json(request), response_json(response)],
            )
            .await?;
        let changed = response_from(&out, response);
        (changed != *response).then_some(changed)
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    /// What each script's log says.
    pub fn log(&self, path: &Path) -> ScriptLog {
        self.logs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(path)
            .cloned()
            .unwrap_or_default()
    }

    async fn call(&self, path: &Path, function: &'static str, args: Vec<Json>) -> Option<Json> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        if !self.send(
            path,
            Job::Call {
                function,
                args,
                reply,
            },
        ) {
            return None;
        }
        match answer.await {
            Ok(Ok(out)) => {
                self.with_log(path, |log| log.error = None);
                out
            }
            Ok(Err(why)) => {
                self.failed(path, why);
                None
            }
            Err(_) => None,
        }
    }

    fn failed(&self, path: &Path, why: String) {
        self.with_log(path, |log| {
            push_line(&mut log.lines, format!("error: {why}"));
            log.error = Some(why);
        });
    }

    fn with_log(&self, path: &Path, change: impl FnOnce(&mut ScriptLog)) {
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        change(logs.entry(path.to_owned()).or_default());
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Hands the job to the script's thread, starting it the first time.
    fn send(&self, path: &Path, job: Job) -> bool {
        let mut threads = self.threads.lock().unwrap_or_else(|e| e.into_inner());
        let sender = threads.entry(path.to_owned()).or_insert_with(|| {
            let (sender, jobs) = mpsc::channel();
            let (path, logs, version) = (
                path.to_owned(),
                Arc::clone(&self.logs),
                Arc::clone(&self.version),
            );
            let _ = std::thread::Builder::new()
                .name("script".into())
                .spawn(move || Runner::new(path, logs, version).run(jobs));
            sender
        });
        sender.send(job).is_ok()
    }
}

fn push_line(lines: &mut Vec<String>, line: String) {
    lines.push(line);
    if lines.len() > LOG_KEPT {
        let extra = lines.len() - LOG_KEPT;
        lines.drain(..extra);
    }
}

/// One script's thread: its file, read again when it changes.
struct Runner {
    path: PathBuf,
    logs: Arc<Mutex<HashMap<PathBuf, ScriptLog>>>,
    version: Arc<AtomicU64>,
    /// The file as last read, and what came of it.
    read: Option<(Option<SystemTime>, Result<Loaded, String>)>,
}

struct Loaded {
    context: Context,
    /// Lines `console.log` wrote during the call under way.
    lines: Arc<Mutex<VecDeque<String>>>,
    /// When the call under way must end, in ms after `epoch`; 0 for none.
    deadline: Arc<AtomicU64>,
    epoch: Instant,
}

impl Runner {
    fn new(
        path: PathBuf,
        logs: Arc<Mutex<HashMap<PathBuf, ScriptLog>>>,
        version: Arc<AtomicU64>,
    ) -> Self {
        Self {
            path,
            logs,
            version,
            read: None,
        }
    }

    fn run(mut self, jobs: mpsc::Receiver<Job>) {
        while let Ok(job) = jobs.recv() {
            let loaded = self.load();
            match job {
                Job::Hooks(reply) => {
                    let _ = reply.send(loaded.map(|loaded| {
                        loaded.context.with(|ctx| {
                            let has = |name: &str| {
                                ctx.globals()
                                    .get::<_, Value>(name)
                                    .is_ok_and(|v| v.is_function())
                            };
                            (has("onRequest"), has("onResponse"))
                        })
                    }));
                }
                Job::Call {
                    function,
                    args,
                    reply,
                } => {
                    let out = loaded.and_then(|loaded| call(loaded, function, &args));
                    let lines: Vec<String> = match &self.read {
                        Some((_, Ok(loaded))) => loaded
                            .lines
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .drain(..)
                            .collect(),
                        _ => Vec::new(),
                    };
                    if !lines.is_empty() || out.is_ok() {
                        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
                        let log = logs.entry(self.path.clone()).or_default();
                        log.calls += 1;
                        for line in lines {
                            push_line(&mut log.lines, line);
                        }
                        self.version.fetch_add(1, Ordering::Relaxed);
                    }
                    let _ = reply.send(out);
                }
            }
        }
    }

    /// The script, read again if its file has changed since.
    fn load(&mut self) -> Result<&Loaded, String> {
        let modified = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok();
        let fresh =
            matches!(&self.read, Some((read, _)) if *read == modified && modified.is_some());
        if !fresh {
            self.read = Some((modified, read_script(&self.path)));
        }
        match &self.read {
            Some((_, Ok(loaded))) => Ok(loaded),
            Some((_, Err(why))) => Err(why.clone()),
            None => Err("not read".to_owned()),
        }
    }
}

fn read_script(path: &Path) -> Result<Loaded, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let runtime = Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_memory_limit(MEMORY_LIMIT);
    runtime.set_max_stack_size(STACK_LIMIT);
    let deadline = Arc::new(AtomicU64::new(0));
    let epoch = Instant::now();
    {
        let deadline = Arc::clone(&deadline);
        runtime.set_interrupt_handler(Some(Box::new(move || {
            let at = deadline.load(Ordering::Relaxed);
            at != 0 && epoch.elapsed().as_millis() as u64 > at
        })));
    }
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    let lines = Arc::new(Mutex::new(VecDeque::new()));
    let loaded = Loaded {
        context,
        lines,
        deadline,
        epoch,
    };
    loaded.deadline.store(limit_at(&loaded), Ordering::Relaxed);
    let evaluated = loaded.context.with(|ctx| -> Result<(), String> {
        let globals = ctx.globals();
        let written = Arc::clone(&loaded.lines);
        let log = Function::new(ctx.clone(), move |args: Rest<Value>| {
            let line = args.0.iter().map(shown).collect::<Vec<_>>().join(" ");
            let line: String = line.chars().take(LINE_MAX).collect();
            written
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push_back(line);
        })
        .map_err(|e| e.to_string())?;
        let console = Object::new(ctx.clone()).map_err(|e| e.to_string())?;
        for name in ["log", "info", "warn", "error", "debug"] {
            console.set(name, log.clone()).map_err(|e| e.to_string())?;
        }
        globals.set("console", console).map_err(|e| e.to_string())?;
        globals
            .set(
                "state",
                Object::new(ctx.clone()).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        ctx.eval::<(), _>(source.as_bytes())
            .catch(&ctx)
            .map_err(caught)
    });
    loaded.deadline.store(0, Ordering::Relaxed);
    evaluated.map_err(|why| interrupted_or(why, &loaded))?;
    Ok(loaded)
}

/// What a script threw, as `TypeError: … at line:column`.
fn caught(error: rquickjs::CaughtError<'_>) -> String {
    match error {
        rquickjs::CaughtError::Exception(e) => {
            let name: Option<String> = e.get("name").ok();
            let message = e.message().unwrap_or_default();
            let mut text = match name {
                Some(name) if !name.is_empty() => format!("{name}: {message}"),
                _ => message,
            };
            if let Some(stack) = e.stack().filter(|s| !s.trim().is_empty()) {
                text.push('\n');
                text.push_str(stack.trim_end());
            }
            text
        }
        other => other.to_string().trim_end().to_owned(),
    }
}

fn limit_at(loaded: &Loaded) -> u64 {
    (loaded.epoch.elapsed() + TIME_LIMIT).as_millis() as u64
}

/// A stopped call says why in its own words.
fn interrupted_or(why: String, loaded: &Loaded) -> String {
    let at = loaded.deadline.load(Ordering::Relaxed);
    if why.contains("interrupted") || (at != 0 && loaded.epoch.elapsed().as_millis() as u64 > at) {
        format!("stopped after {} s: {why}", TIME_LIMIT.as_secs())
    } else {
        why
    }
}

/// What `console.log` shows for a value: text as it is, the rest as JSON.
fn shown(value: &Value) -> String {
    if let Some(text) = value.as_string() {
        return text.to_string().unwrap_or_default();
    }
    let ctx = value.ctx();
    match ctx.json_stringify(value.clone()) {
        Ok(Some(json)) => json.to_string().unwrap_or_default(),
        _ => format!("{value:?}"),
    }
}

/// Calls `function` with `args`: what it returned, or the first argument as
/// it left it; `None` if the script has no such function.
fn call(loaded: &Loaded, function: &str, args: &[Json]) -> Result<Option<Json>, String> {
    loaded.deadline.store(limit_at(loaded), Ordering::Relaxed);
    let out = loaded.context.with(|ctx| -> Result<Option<Json>, String> {
        let Ok(found) = ctx.globals().get::<_, Value>(function) else {
            return Ok(None);
        };
        let Some(found) = found.as_function().cloned() else {
            return Ok(None);
        };
        let given = args
            .iter()
            .map(|arg| ctx.json_parse(arg.to_string()))
            .collect::<rquickjs::Result<Vec<Value>>>()
            .map_err(|e| e.to_string())?;
        let returned: Value = match given.len() {
            1 => found.call((given[0].clone(),)),
            _ => found.call((given[0].clone(), given[1].clone())),
        }
        .catch(&ctx)
        .map_err(caught)?;
        let out = if returned.is_undefined() || returned.is_null() {
            match function {
                // A request is the first argument, a response the second.
                "onResponse" => given[1].clone(),
                _ => given[0].clone(),
            }
        } else {
            returned
        };
        to_json(&ctx, out).map(Some)
    });
    loaded.deadline.store(0, Ordering::Relaxed);
    out.map_err(|why| interrupted_or(why, loaded))
}

fn to_json<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Json, String> {
    let text = ctx
        .json_stringify(value)
        .catch(ctx)
        .map_err(caught)?
        .ok_or("it gave back nothing that can be read")?
        .to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn headers_json(headers: &[(String, String)]) -> Json {
    let mut out = Map::new();
    for (name, value) in headers {
        let name = name.to_ascii_lowercase();
        match out.get_mut(&name) {
            None => {
                out.insert(name, Json::String(value.clone()));
            }
            Some(Json::Array(values)) => values.push(Json::String(value.clone())),
            Some(single) => {
                let first = single.take();
                *single = Json::Array(vec![first, Json::String(value.clone())]);
            }
        }
    }
    Json::Object(out)
}

fn body_json(body: &[u8]) -> Json {
    std::str::from_utf8(body).map_or(Json::Null, |text| Json::String(text.to_owned()))
}

fn request_json(request: &Message) -> Json {
    json!({
        "method": request.method,
        "url": request.url,
        "headers": headers_json(&request.headers),
        "body": body_json(&request.body),
    })
}

fn response_json(response: &Message) -> Json {
    json!({
        "status": response.status,
        "headers": headers_json(&response.headers),
        "body": body_json(&response.body),
    })
}

fn headers_from(json: Option<&Json>, was: &[(String, String)]) -> Vec<(String, String)> {
    let Some(Json::Object(headers)) = json else {
        return was.to_vec();
    };
    // Those that did not change keep their place and their case.
    let mut out: Vec<(String, String)> = Vec::new();
    let text = |value: &Json| match value {
        Json::String(text) => text.clone(),
        other => other.to_string(),
    };
    for (name, value) in headers {
        let values: Vec<String> = match value {
            Json::Null => continue,
            Json::Array(values) => values.iter().map(text).collect(),
            single => vec![text(single)],
        };
        let before: Vec<&(String, String)> = was
            .iter()
            .filter(|(had, _)| had.eq_ignore_ascii_case(name))
            .collect();
        let same =
            before.len() == values.len() && before.iter().zip(&values).all(|((_, a), b)| a == b);
        if same {
            out.extend(before.into_iter().cloned());
        } else {
            out.extend(values.into_iter().map(|value| (name.clone(), value)));
        }
    }
    // In the order they came, so that nothing moves for no reason.
    let place = |name: &str| {
        was.iter()
            .position(|(had, _)| had.eq_ignore_ascii_case(name))
            .unwrap_or(usize::MAX)
    };
    out.sort_by_key(|(name, _)| place(name));
    out
}

fn body_from(json: Option<&Json>, was: &[u8]) -> Vec<u8> {
    match json {
        Some(Json::String(text)) => text.clone().into_bytes(),
        // Not text, and left alone: as it came.
        Some(Json::Null) | None if std::str::from_utf8(was).is_err() => was.to_vec(),
        None => was.to_vec(),
        Some(Json::Null) => Vec::new(),
        Some(other) => other.to_string().into_bytes(),
    }
}

fn request_from(json: &Json, was: &Message) -> Message {
    let text = |key: &str, was: &str| {
        json.get(key)
            .and_then(Json::as_str)
            .map_or_else(|| was.to_owned(), str::to_owned)
    };
    Message {
        method: text("method", &was.method),
        url: text("url", &was.url),
        status: 0,
        headers: headers_from(json.get("headers"), &was.headers),
        body: body_from(json.get("body"), &was.body),
    }
}

fn response_from(json: &Json, was: &Message) -> Message {
    Message {
        method: was.method.clone(),
        url: was.url.clone(),
        status: json
            .get("status")
            .and_then(Json::as_u64)
            .and_then(|s| u16::try_from(s).ok())
            .filter(|s| (100..=999).contains(s))
            .unwrap_or(was.status),
        headers: headers_from(json.get("headers"), &was.headers),
        body: body_from(json.get("body"), &was.body),
    }
}

impl Message {
    /// What an answer a script makes starts from: 200, nothing in it.
    fn default_response() -> Self {
        Self {
            method: String::new(),
            url: String::new(),
            status: 200,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Message {
        Message {
            method: "POST".into(),
            url: "https://api.test/items?a=1".into(),
            status: 0,
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
                ("Accept".into(), "*/*".into()),
                ("x-many".into(), "1".into()),
                ("x-many".into(), "2".into()),
            ],
            body: br#"{"n":1}"#.to_vec(),
        }
    }

    fn response() -> Message {
        Message {
            method: "POST".into(),
            url: "https://api.test/items?a=1".into(),
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: br#"{"ok":true}"#.to_vec(),
        }
    }

    fn script(dir: &Path, name: &str, source: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, source).unwrap();
        path
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scripts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn block<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn a_script_changes_a_request_and_its_response() {
        let dir = dir("change");
        let path = script(
            &dir,
            "change.js",
            r#"
            function onRequest(request) {
              request.headers["x-debug"] = "1";
              delete request.headers["accept"];
              const body = JSON.parse(request.body);
              body.n += 1;
              request.body = JSON.stringify(body);
              state.seen = (state.seen || 0) + 1;
              console.log("seen", state.seen, { url: request.url });
            }
            function onResponse(request, response) {
              return { status: 201, headers: response.headers,
                       body: request.method + " " + state.seen };
            }
            "#,
        );
        let scripts = Scripts::default();
        assert_eq!(block(scripts.hooks(&path)), (true, true));
        let OnRequest::Changed(changed) = block(scripts.on_request(&path, &request())) else {
            panic!("changed");
        };
        assert_eq!(changed.body, br#"{"n":2}"#);
        assert_eq!(
            changed.headers,
            [
                ("Content-Type".to_owned(), "application/json".to_owned()),
                ("x-many".to_owned(), "1".to_owned()),
                ("x-many".to_owned(), "2".to_owned()),
                ("x-debug".to_owned(), "1".to_owned()),
            ],
            "the rest as they were, in their place and case"
        );
        let changed = block(scripts.on_response(&path, &request(), &response())).unwrap();
        assert_eq!(changed.status, 201);
        assert_eq!(changed.body, b"POST 1", "state is kept between calls");
        let log = scripts.log(&path);
        assert_eq!(
            log.lines,
            [r#"seen 1 {"url":"https://api.test/items?a=1"}"#]
        );
        assert_eq!(log.error, None);

        // Changed on the disk: read again, its state new.
        std::thread::sleep(Duration::from_millis(20));
        script(
            &dir,
            "change.js",
            "function onRequest(r) { r.method = 'PUT'; }",
        );
        let OnRequest::Changed(changed) = block(scripts.on_request(&path, &request())) else {
            panic!("changed again");
        };
        assert_eq!(changed.method, "PUT");
        assert_eq!(changed.body, request().body);
        assert_eq!(block(scripts.hooks(&path)), (true, false));
        assert_eq!(
            block(scripts.on_response(&path, &request(), &response())),
            None
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_script_can_answer_by_itself() {
        let dir = dir("answer");
        let path = script(
            &dir,
            "mock.js",
            r#"function onRequest(request) {
                 if (request.url.includes("items")) {
                   return { response: { status: 404, headers: { "x-mock": "yes" }, body: "none" } };
                 }
               }"#,
        );
        let scripts = Scripts::default();
        let OnRequest::Answer(answer) = block(scripts.on_request(&path, &request())) else {
            panic!("answered");
        };
        assert_eq!(answer.status, 404);
        assert_eq!(answer.headers, [("x-mock".to_owned(), "yes".to_owned())]);
        assert_eq!(answer.body, b"none");
        let mut other = request();
        other.url = "https://api.test/other".into();
        assert_eq!(
            block(scripts.on_request(&path, &other)),
            OnRequest::Unchanged
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_body_that_is_not_text_goes_on_as_it_came() {
        let dir = dir("binary");
        let path = script(
            &dir,
            "binary.js",
            "function onResponse(q, r) { r.headers['x-seen'] = String(r.body); }",
        );
        let scripts = Scripts::default();
        let mut binary = response();
        binary.body = vec![0xff, 0x00, 0xfe];
        let changed = block(scripts.on_response(&path, &request(), &binary)).unwrap();
        assert_eq!(changed.body, [0xff, 0x00, 0xfe]);
        assert!(
            changed
                .headers
                .contains(&("x-seen".to_owned(), "null".to_owned()))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_script_that_fails_leaves_traffic_alone_and_says_why() {
        let dir = dir("fail");
        let scripts = Scripts::default();
        let broken = script(&dir, "broken.js", "function onRequest( {");
        assert_eq!(block(scripts.hooks(&broken)), (false, false));
        let why = scripts.log(&broken).error.unwrap();
        assert!(
            why.starts_with("SyntaxError: ") && why.contains(":1:"),
            "what and where: {why}"
        );

        let throws = script(&dir, "throws.js", "function onRequest(r) { r.x.y = 1; }");
        assert_eq!(
            block(scripts.on_request(&throws, &request())),
            OnRequest::Unchanged
        );
        assert!(scripts.log(&throws).error.unwrap().contains("TypeError"));

        let forever = script(&dir, "forever.js", "function onRequest(r) { for (;;) {} }");
        let started = Instant::now();
        assert_eq!(
            block(scripts.on_request(&forever, &request())),
            OnRequest::Unchanged
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(
            scripts
                .log(&forever)
                .error
                .unwrap()
                .contains("stopped after 1 s"),
            "{:?}",
            scripts.log(&forever)
        );
        // And the next call runs again.
        assert_eq!(
            block(scripts.on_request(&forever, &request())),
            OnRequest::Unchanged
        );

        let greedy = script(
            &dir,
            "greedy.js",
            "function onRequest(r) { const a = []; for (;;) a.push('x'.repeat(1 << 20)); }",
        );
        assert_eq!(
            block(scripts.on_request(&greedy, &request())),
            OnRequest::Unchanged
        );
        assert!(scripts.log(&greedy).error.is_some());

        let missing = dir.join("missing.js");
        assert_eq!(
            block(scripts.on_request(&missing, &request())),
            OnRequest::Unchanged
        );
        assert!(scripts.log(&missing).error.unwrap().contains("cannot read"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
