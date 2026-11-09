//! Calls to LLM APIs, put back together: what was asked (the system prompt,
//! the messages, the tools offered) and what came back (the text, the
//! thinking, the tool calls, why it stopped and how many tokens), from a body
//! or from the hundreds of events a streamed answer comes in.
//!
//! Known by the request's path, so OpenAI's format serves every API that
//! copies it: OpenRouter, Groq, Together, LM Studio, vLLM and the rest.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    /// `/chat/completions`, OpenAI's and everyone's who speaks it.
    OpenAiChat,
    /// `/responses`.
    OpenAiResponses,
    /// `/v1/messages`.
    Anthropic,
    /// `:generateContent` and `:streamGenerateContent`.
    Gemini,
    /// Ollama's own `/api/chat`.
    Ollama,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenAiChat => "Chat Completions",
            Self::OpenAiResponses => "OpenAI Responses",
            Self::Anthropic => "Anthropic Messages",
            Self::Gemini => "Gemini",
            Self::Ollama => "Ollama",
        }
    }
}

/// What a POST to `url` is, if it is a call to a model.
pub fn provider_of(method: &str, url: &str) -> Option<Provider> {
    if !method.eq_ignore_ascii_case("POST") {
        return None;
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let path = path.trim_end_matches('/');
    if path.ends_with("/chat/completions") {
        Some(Provider::OpenAiChat)
    } else if path.ends_with("/v1/responses") || path.ends_with("/openai/responses") {
        Some(Provider::OpenAiResponses)
    } else if path.ends_with("/v1/messages") {
        Some(Provider::Anthropic)
    } else if path.contains(":generateContent") || path.contains(":streamGenerateContent") {
        Some(Provider::Gemini)
    } else if path.ends_with("/api/chat") {
        Some(Provider::Ollama)
    } else {
        None
    }
}

/// One message of the conversation sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    /// As JSON, laid out if it is JSON.
    pub arguments: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Exchange {
    pub model: Option<String>,
    /// Asked to stream.
    pub stream: bool,
    /// The events or lines the answer came in, if it streamed.
    pub chunks: usize,
    pub messages: Vec<Message>,
    /// The names of the tools the model was offered.
    pub tools: Vec<String>,
    pub thinking: String,
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub stop: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    /// What the API said went wrong.
    pub error: Option<String>,
}

/// The call put back together, from its request and response bodies, both
/// unpacked. `None` if the request is not what `provider` sends.
pub fn read(provider: Provider, url: &str, request: &[u8], response: &[u8]) -> Option<Exchange> {
    let asked: Value = serde_json::from_slice(request).ok()?;
    let mut exchange = Exchange::default();
    read_request(provider, url, &asked, &mut exchange);
    read_response(provider, response, &mut exchange);
    Some(exchange)
}

fn str_of(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// Arguments as they came: laid out if they are JSON, and as given if not
/// (a stream cut short leaves them half written).
fn arguments(value: &Value) -> String {
    match value {
        Value::String(text) => match serde_json::from_str::<Value>(text) {
            Ok(json) => serde_json::to_string_pretty(&json).unwrap_or_else(|_| text.clone()),
            Err(_) => text.clone(),
        },
        Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

/// The text of a message's content, whatever shape it takes: a string, or a
/// list of parts of text, images, tool calls and their results.
fn content_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .map(part_text)
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => part_text(other),
    }
}

fn part_text(part: &Value) -> String {
    if let Some(text) = part.as_str() {
        return text.to_owned();
    }
    let kind = part.get("type").and_then(Value::as_str).unwrap_or("");
    if let Some(text) = part.get("text").and_then(Value::as_str) {
        return text.to_owned();
    }
    if let Some(call) = part.get("functionCall") {
        let name = call.get("name").and_then(Value::as_str).unwrap_or("?");
        return format!(
            "→ {name}({})",
            compact(call.get("args").unwrap_or(&Value::Null))
        );
    }
    if let Some(answer) = part.get("functionResponse") {
        let name = answer.get("name").and_then(Value::as_str).unwrap_or("?");
        return format!(
            "← {name}: {}",
            compact(answer.get("response").unwrap_or(&Value::Null))
        );
    }
    match kind {
        "tool_use" | "function_call" => {
            let name = part.get("name").and_then(Value::as_str).unwrap_or("?");
            let input = part.get("input").or_else(|| part.get("arguments"));
            format!("→ {name}({})", compact(input.unwrap_or(&Value::Null)))
        }
        "tool_result" | "function_call_output" => {
            let result = part.get("content").or_else(|| part.get("output"));
            format!("← {}", content_text(result.unwrap_or(&Value::Null)))
        }
        "thinking" => part
            .get("thinking")
            .and_then(Value::as_str)
            .map(|t| format!("(thinking) {t}"))
            .unwrap_or_default(),
        "image" | "image_url" | "input_image" => "[image]".to_owned(),
        "document" | "file" | "input_file" => "[file]".to_owned(),
        _ if part.get("inlineData").is_some() => "[inline data]".to_owned(),
        _ => String::new(),
    }
}

/// JSON on one line, as a call's arguments show in a message.
fn compact(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn read_request(provider: Provider, url: &str, asked: &Value, out: &mut Exchange) {
    out.model = asked.get("model").and_then(str_of);
    out.stream = asked
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut messages = Vec::new();
    match provider {
        Provider::OpenAiChat | Provider::Ollama | Provider::Anthropic => {
            if let Some(system) = asked.get("system") {
                messages.push(Message {
                    role: "system".to_owned(),
                    text: content_text(system),
                });
            }
            for message in asked
                .get("messages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let role = message.get("role").and_then(str_of).unwrap_or_default();
                let mut text = content_text(message.get("content").unwrap_or(&Value::Null));
                // A tool call the model made before, as OpenAI keeps it.
                for call in message
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let function = call.get("function").unwrap_or(call);
                    let name = function.get("name").and_then(Value::as_str).unwrap_or("?");
                    let args = compact(function.get("arguments").unwrap_or(&Value::Null));
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&format!("→ {name}({args})"));
                }
                messages.push(Message { role, text });
            }
            // Ollama streams unless told not to.
            if provider == Provider::Ollama && asked.get("stream").is_none() {
                out.stream = true;
            }
        }
        Provider::OpenAiResponses => {
            if let Some(instructions) = asked.get("instructions").and_then(Value::as_str) {
                messages.push(Message {
                    role: "system".to_owned(),
                    text: instructions.to_owned(),
                });
            }
            match asked.get("input") {
                Some(Value::String(text)) => messages.push(Message {
                    role: "user".to_owned(),
                    text: text.clone(),
                }),
                Some(Value::Array(items)) => {
                    for item in items {
                        let role = item
                            .get("role")
                            .and_then(str_of)
                            .or_else(|| item.get("type").and_then(str_of))
                            .unwrap_or_default();
                        let text = match item.get("content") {
                            Some(content) => content_text(content),
                            None => part_text(item),
                        };
                        messages.push(Message { role, text });
                    }
                }
                _ => {}
            }
        }
        Provider::Gemini => {
            // `…/models/gemini-2.5-pro:generateContent`.
            out.model = url
                .split(['?', '#'])
                .next()
                .and_then(|path| path.rsplit_once("/models/"))
                .and_then(|(_, rest)| rest.split(':').next())
                .map(str::to_owned);
            out.stream = url.contains(":streamGenerateContent");
            if let Some(system) = asked
                .get("systemInstruction")
                .or_else(|| asked.get("system_instruction"))
            {
                messages.push(Message {
                    role: "system".to_owned(),
                    text: content_text(system.get("parts").unwrap_or(&Value::Null)),
                });
            }
            for content in asked
                .get("contents")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                messages.push(Message {
                    role: content
                        .get("role")
                        .and_then(str_of)
                        .unwrap_or_else(|| "user".to_owned()),
                    text: content_text(content.get("parts").unwrap_or(&Value::Null)),
                });
            }
        }
    }
    out.messages = messages;
    out.tools = tool_names(asked.get("tools"));
}

fn tool_names(tools: Option<&Value>) -> Vec<String> {
    let mut names = Vec::new();
    for tool in tools.and_then(Value::as_array).into_iter().flatten() {
        if let Some(declarations) = tool
            .get("functionDeclarations")
            .or_else(|| tool.get("function_declarations"))
            .and_then(Value::as_array)
        {
            names.extend(
                declarations
                    .iter()
                    .filter_map(|d| d.get("name").and_then(str_of)),
            );
            continue;
        }
        let name = tool
            .get("function")
            .and_then(|f| f.get("name"))
            .or_else(|| tool.get("name"))
            .and_then(str_of)
            // OpenAI's own tools, as `web_search`, have only a type.
            .or_else(|| tool.get("type").and_then(str_of));
        names.extend(name);
    }
    names
}

/// `event:` and `data:` of each server-sent event, the data's lines joined.
fn events(text: &str) -> Vec<(Option<String>, String)> {
    let text = text.replace("\r\n", "\n");
    text.split("\n\n")
        .filter_map(|block| {
            let mut event = None;
            let mut data: Vec<&str> = Vec::new();
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("event:") {
                    event = Some(value.trim().to_owned());
                } else if let Some(value) = line.strip_prefix("data:") {
                    data.push(value.strip_prefix(' ').unwrap_or(value));
                }
            }
            (!data.is_empty()).then(|| (event, data.join("\n")))
        })
        .collect()
}

fn looks_like_events(text: &str) -> bool {
    let start = text.trim_start();
    start.starts_with("data:") || start.starts_with("event:") || start.starts_with(':')
}

fn read_response(provider: Provider, body: &[u8], out: &mut Exchange) {
    let text = String::from_utf8_lossy(body);
    if looks_like_events(&text) {
        let events = events(&text);
        out.chunks = events.len();
        let mut calls = Calls::default();
        for (event, data) in &events {
            let Ok(chunk) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            stream_event(provider, event.as_deref(), &chunk, out, &mut calls);
        }
        out.tool_calls.extend(calls.done());
        return;
    }
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    match serde_json::from_str::<Value>(&text) {
        // Gemini's stream without `alt=sse`: a list of the same chunks.
        Ok(Value::Array(chunks)) => {
            out.chunks = chunks.len();
            let mut calls = Calls::default();
            for chunk in &chunks {
                stream_event(provider, None, chunk, out, &mut calls);
            }
            out.tool_calls.extend(calls.done());
        }
        Ok(whole) => whole_answer(provider, &whole, out),
        // Ollama's stream: a JSON object a line.
        Err(_) if lines.len() > 1 => {
            out.chunks = lines.len();
            let mut calls = Calls::default();
            for line in lines {
                if let Ok(chunk) = serde_json::from_str::<Value>(line) {
                    stream_event(provider, None, &chunk, out, &mut calls);
                }
            }
            out.tool_calls.extend(calls.done());
        }
        Err(_) => {}
    }
}

/// Tool calls as they arrive in pieces, by their place in the answer.
#[derive(Default)]
struct Calls(Vec<(i64, ToolCall)>);

impl Calls {
    fn at(&mut self, index: i64) -> &mut ToolCall {
        if let Some(at) = self.0.iter().position(|(i, _)| *i == index) {
            return &mut self.0[at].1;
        }
        self.0.push((index, ToolCall::default()));
        &mut self.0.last_mut().expect("just pushed").1
    }

    fn done(self) -> impl Iterator<Item = ToolCall> {
        self.0.into_iter().map(|(_, mut call)| {
            call.arguments = arguments(&Value::String(call.arguments));
            call
        })
    }
}

fn error_of(value: &Value) -> Option<String> {
    let error = value.get("error")?;
    error
        .get("message")
        .and_then(str_of)
        .or_else(|| str_of(error))
        .or_else(|| Some(error.to_string()))
}

fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut at = value;
    for key in path {
        at = at.get(key)?;
    }
    at.as_u64()
}

fn whole_answer(provider: Provider, answer: &Value, out: &mut Exchange) {
    out.error = error_of(answer);
    if let Some(model) = answer
        .get("model")
        .or_else(|| answer.get("modelVersion"))
        .and_then(str_of)
    {
        out.model = Some(model);
    }
    match provider {
        Provider::OpenAiChat => {
            let choice = &answer["choices"][0];
            let message = &choice["message"];
            out.text = content_text(&message["content"]);
            out.thinking = message
                .get("reasoning_content")
                .or_else(|| message.get("reasoning"))
                .and_then(str_of)
                .unwrap_or_default();
            for call in message["tool_calls"].as_array().into_iter().flatten() {
                out.tool_calls.push(ToolCall {
                    id: call.get("id").and_then(str_of),
                    name: call["function"]["name"].as_str().unwrap_or("?").to_owned(),
                    arguments: arguments(&call["function"]["arguments"]),
                });
            }
            out.stop = choice.get("finish_reason").and_then(str_of);
            out.input_tokens = u64_at(answer, &["usage", "prompt_tokens"]);
            out.output_tokens = u64_at(answer, &["usage", "completion_tokens"]);
        }
        Provider::OpenAiResponses => {
            for item in answer["output"].as_array().into_iter().flatten() {
                match item.get("type").and_then(Value::as_str) {
                    Some("message") => {
                        push_text(&mut out.text, &content_text(&item["content"]));
                    }
                    Some("reasoning") => {
                        for summary in item["summary"].as_array().into_iter().flatten() {
                            push_text(&mut out.thinking, summary["text"].as_str().unwrap_or(""));
                        }
                    }
                    Some("function_call") => out.tool_calls.push(ToolCall {
                        id: item.get("call_id").and_then(str_of),
                        name: item["name"].as_str().unwrap_or("?").to_owned(),
                        arguments: arguments(&item["arguments"]),
                    }),
                    Some(other) => out.tool_calls.push(ToolCall {
                        id: item.get("id").and_then(str_of),
                        name: other.to_owned(),
                        arguments: String::new(),
                    }),
                    None => {}
                }
            }
            out.stop = answer
                .get("incomplete_details")
                .and_then(|d| d.get("reason"))
                .and_then(str_of)
                .or_else(|| answer.get("status").and_then(str_of));
            out.input_tokens = u64_at(answer, &["usage", "input_tokens"]);
            out.output_tokens = u64_at(answer, &["usage", "output_tokens"]);
        }
        Provider::Anthropic => {
            for block in answer["content"].as_array().into_iter().flatten() {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => push_text(&mut out.text, block["text"].as_str().unwrap_or("")),
                    Some("thinking") => {
                        push_text(&mut out.thinking, block["thinking"].as_str().unwrap_or(""))
                    }
                    Some("tool_use" | "server_tool_use") => out.tool_calls.push(ToolCall {
                        id: block.get("id").and_then(str_of),
                        name: block["name"].as_str().unwrap_or("?").to_owned(),
                        arguments: arguments(&block["input"]),
                    }),
                    _ => {}
                }
            }
            out.stop = answer.get("stop_reason").and_then(str_of);
            out.input_tokens = u64_at(answer, &["usage", "input_tokens"]);
            out.output_tokens = u64_at(answer, &["usage", "output_tokens"]);
        }
        Provider::Gemini => gemini_chunk(answer, out, &mut Calls::default(), true),
        Provider::Ollama => {
            let mut calls = Calls::default();
            stream_event(provider, None, answer, out, &mut calls);
            out.tool_calls.extend(calls.done());
        }
    }
}

fn push_text(to: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    if !to.is_empty() {
        to.push_str("\n\n");
    }
    to.push_str(text);
}

fn gemini_chunk(chunk: &Value, out: &mut Exchange, calls: &mut Calls, whole: bool) {
    if let Some(error) = error_of(chunk) {
        out.error = Some(error);
    }
    if let Some(model) = chunk.get("modelVersion").and_then(str_of) {
        out.model = Some(model);
    }
    let candidate = &chunk["candidates"][0];
    for part in candidate["content"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            let to = if part.get("thought").and_then(Value::as_bool) == Some(true) {
                &mut out.thinking
            } else {
                &mut out.text
            };
            to.push_str(text);
        } else if let Some(call) = part.get("functionCall") {
            let index = calls.0.len() as i64;
            let slot = calls.at(index);
            slot.name = call["name"].as_str().unwrap_or("?").to_owned();
            slot.arguments = call.get("args").map(Value::to_string).unwrap_or_default();
        }
    }
    if let Some(reason) = candidate.get("finishReason").and_then(str_of) {
        out.stop = Some(reason);
    }
    if let Some(n) = u64_at(chunk, &["usageMetadata", "promptTokenCount"]) {
        out.input_tokens = Some(n);
    }
    if let Some(n) = u64_at(chunk, &["usageMetadata", "candidatesTokenCount"]) {
        out.output_tokens = Some(n);
    }
    if whole {
        out.tool_calls.extend(std::mem::take(calls).done());
    }
}

fn stream_event(
    provider: Provider,
    event: Option<&str>,
    chunk: &Value,
    out: &mut Exchange,
    calls: &mut Calls,
) {
    match provider {
        Provider::OpenAiChat => {
            if let Some(error) = error_of(chunk) {
                out.error = Some(error);
            }
            if let Some(model) = chunk.get("model").and_then(str_of) {
                out.model = Some(model);
            }
            let choice = &chunk["choices"][0];
            let delta = &choice["delta"];
            if let Some(text) = delta.get("content").and_then(Value::as_str) {
                out.text.push_str(text);
            }
            if let Some(text) = delta
                .get("reasoning_content")
                .or_else(|| delta.get("reasoning"))
                .and_then(Value::as_str)
            {
                out.thinking.push_str(text);
            }
            for piece in delta["tool_calls"].as_array().into_iter().flatten() {
                let index = piece.get("index").and_then(Value::as_i64).unwrap_or(0);
                let call = calls.at(index);
                if let Some(id) = piece.get("id").and_then(str_of) {
                    call.id = Some(id);
                }
                if let Some(name) = piece["function"].get("name").and_then(Value::as_str) {
                    call.name.push_str(name);
                }
                if let Some(args) = piece["function"].get("arguments").and_then(Value::as_str) {
                    call.arguments.push_str(args);
                }
            }
            if let Some(reason) = choice.get("finish_reason").and_then(str_of) {
                out.stop = Some(reason);
            }
            if let Some(n) = u64_at(chunk, &["usage", "prompt_tokens"]) {
                out.input_tokens = Some(n);
            }
            if let Some(n) = u64_at(chunk, &["usage", "completion_tokens"]) {
                out.output_tokens = Some(n);
            }
        }
        Provider::OpenAiResponses => {
            let kind = event
                .map(str::to_owned)
                .or_else(|| chunk.get("type").and_then(str_of))
                .unwrap_or_default();
            match kind.as_str() {
                "response.created" | "response.in_progress" => {
                    if let Some(model) = chunk["response"].get("model").and_then(str_of) {
                        out.model = Some(model);
                    }
                }
                "response.output_text.delta" => {
                    out.text.push_str(chunk["delta"].as_str().unwrap_or(""));
                }
                "response.reasoning_summary_text.delta" => {
                    out.thinking.push_str(chunk["delta"].as_str().unwrap_or(""));
                }
                "response.output_item.added" => {
                    let item = &chunk["item"];
                    if item.get("type").and_then(Value::as_str) == Some("function_call") {
                        let index = chunk
                            .get("output_index")
                            .and_then(Value::as_i64)
                            .unwrap_or(0);
                        let call = calls.at(index);
                        call.name = item["name"].as_str().unwrap_or("?").to_owned();
                        call.id = item.get("call_id").and_then(str_of);
                    }
                }
                "response.function_call_arguments.delta" => {
                    let index = chunk
                        .get("output_index")
                        .and_then(Value::as_i64)
                        .unwrap_or(0);
                    calls
                        .at(index)
                        .arguments
                        .push_str(chunk["delta"].as_str().unwrap_or(""));
                }
                // The whole answer again, as it ended: it says it best.
                "response.completed" | "response.incomplete" | "response.failed" => {
                    let mut whole = Exchange {
                        messages: std::mem::take(&mut out.messages),
                        tools: std::mem::take(&mut out.tools),
                        stream: out.stream,
                        chunks: out.chunks,
                        ..Exchange::default()
                    };
                    whole_answer(Provider::OpenAiResponses, &chunk["response"], &mut whole);
                    if whole.model.is_none() {
                        whole.model = out.model.take();
                    }
                    *out = whole;
                    *calls = Calls::default();
                }
                "error" => {
                    out.error = chunk
                        .get("message")
                        .and_then(str_of)
                        .or_else(|| error_of(chunk))
                }
                _ => {}
            }
        }
        Provider::Anthropic => {
            let kind = event
                .map(str::to_owned)
                .or_else(|| chunk.get("type").and_then(str_of))
                .unwrap_or_default();
            match kind.as_str() {
                "message_start" => {
                    let message = &chunk["message"];
                    if let Some(model) = message.get("model").and_then(str_of) {
                        out.model = Some(model);
                    }
                    out.input_tokens = u64_at(message, &["usage", "input_tokens"]);
                }
                "content_block_start" => {
                    let block = &chunk["content_block"];
                    let kind = block.get("type").and_then(Value::as_str).unwrap_or("");
                    if matches!(kind, "tool_use" | "server_tool_use") {
                        let index = chunk.get("index").and_then(Value::as_i64).unwrap_or(0);
                        let call = calls.at(index);
                        call.id = block.get("id").and_then(str_of);
                        call.name = block["name"].as_str().unwrap_or("?").to_owned();
                    } else if kind == "text" && !out.text.is_empty() {
                        out.text.push_str("\n\n");
                    }
                }
                "content_block_delta" => {
                    let delta = &chunk["delta"];
                    match delta.get("type").and_then(Value::as_str) {
                        Some("text_delta") => {
                            out.text.push_str(delta["text"].as_str().unwrap_or(""))
                        }
                        Some("thinking_delta") => out
                            .thinking
                            .push_str(delta["thinking"].as_str().unwrap_or("")),
                        Some("input_json_delta") => {
                            let index = chunk.get("index").and_then(Value::as_i64).unwrap_or(0);
                            calls
                                .at(index)
                                .arguments
                                .push_str(delta["partial_json"].as_str().unwrap_or(""));
                        }
                        _ => {}
                    }
                }
                "message_delta" => {
                    if let Some(reason) = chunk["delta"].get("stop_reason").and_then(str_of) {
                        out.stop = Some(reason);
                    }
                    if let Some(n) = u64_at(chunk, &["usage", "output_tokens"]) {
                        out.output_tokens = Some(n);
                    }
                }
                "error" => out.error = error_of(chunk),
                _ => {}
            }
        }
        Provider::Gemini => gemini_chunk(chunk, out, calls, false),
        Provider::Ollama => {
            if let Some(error) = error_of(chunk) {
                out.error = Some(error);
            }
            if let Some(model) = chunk.get("model").and_then(str_of) {
                out.model = Some(model);
            }
            let message = &chunk["message"];
            out.text.push_str(message["content"].as_str().unwrap_or(""));
            out.thinking
                .push_str(message["thinking"].as_str().unwrap_or(""));
            for call in message["tool_calls"].as_array().into_iter().flatten() {
                let index = calls.0.len() as i64;
                let slot = calls.at(index);
                slot.name = call["function"]["name"].as_str().unwrap_or("?").to_owned();
                slot.arguments = call["function"]
                    .get("arguments")
                    .map(Value::to_string)
                    .unwrap_or_default();
            }
            if let Some(reason) = chunk.get("done_reason").and_then(str_of) {
                out.stop = Some(reason);
            }
            if let Some(n) = chunk.get("prompt_eval_count").and_then(Value::as_u64) {
                out.input_tokens = Some(n);
            }
            if let Some(n) = chunk.get("eval_count").and_then(Value::as_u64) {
                out.output_tokens = Some(n);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sse(events: &[(&str, &str)]) -> Vec<u8> {
        events
            .iter()
            .map(|(event, data)| {
                if event.is_empty() {
                    format!("data: {data}\n\n")
                } else {
                    format!("event: {event}\ndata: {data}\n\n")
                }
            })
            .collect::<String>()
            .into_bytes()
    }

    #[test]
    fn calls_are_known_by_their_path() {
        let of = |url| provider_of("POST", url);
        assert_eq!(
            of("https://api.openai.com/v1/chat/completions"),
            Some(Provider::OpenAiChat)
        );
        assert_eq!(
            of("https://openrouter.ai/api/v1/chat/completions"),
            Some(Provider::OpenAiChat)
        );
        assert_eq!(
            of("https://api.openai.com/v1/responses"),
            Some(Provider::OpenAiResponses)
        );
        assert_eq!(
            of("https://api.anthropic.com/v1/messages?beta=true"),
            Some(Provider::Anthropic)
        );
        assert_eq!(
            of(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
            ),
            Some(Provider::Gemini)
        );
        assert_eq!(
            of("http://localhost:11434/api/chat"),
            Some(Provider::Ollama)
        );
        assert_eq!(
            of("https://api.anthropic.com/v1/messages/count_tokens"),
            None
        );
        assert_eq!(
            provider_of("GET", "https://api.openai.com/v1/chat/completions"),
            None
        );
    }

    #[test]
    fn a_streamed_chat_completion_with_a_tool_call() {
        let request = br#"{"model":"gpt-4.1","stream":true,
            "messages":[{"role":"system","content":"Be brief."},
                        {"role":"user","content":[{"type":"text","text":"Weather in Hanoi?"},{"type":"image_url","image_url":{"url":"x"}}]}],
            "tools":[{"type":"function","function":{"name":"get_weather","parameters":{}}}]}"#;
        let response = sse(&[
            (
                "",
                r#"{"model":"gpt-4.1-2025-04-14","choices":[{"index":0,"delta":{"role":"assistant","content":"Let me "}}]}"#,
            ),
            (
                "",
                r#"{"choices":[{"index":0,"delta":{"content":"check."}}]}"#,
            ),
            (
                "",
                r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"get_weather","arguments":""}}]}}]}"#,
            ),
            (
                "",
                r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"city\":"}}]}}]}"#,
            ),
            (
                "",
                r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"Hanoi\"}"}}]}}]}"#,
            ),
            (
                "",
                r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":52,"completion_tokens":18}}"#,
            ),
            ("", "[DONE]"),
        ]);
        let call = read(Provider::OpenAiChat, "", request, &response).unwrap();
        assert_eq!(call.model.as_deref(), Some("gpt-4.1-2025-04-14"));
        assert!(call.stream);
        assert_eq!(call.chunks, 7);
        assert_eq!(
            call.messages[0],
            Message {
                role: "system".into(),
                text: "Be brief.".into()
            }
        );
        assert_eq!(call.messages[1].text, "Weather in Hanoi?\n[image]");
        assert_eq!(call.tools, ["get_weather"]);
        assert_eq!(call.text, "Let me check.");
        assert_eq!(call.tool_calls.len(), 1);
        assert_eq!(call.tool_calls[0].name, "get_weather");
        assert_eq!(call.tool_calls[0].id.as_deref(), Some("call_1"));
        assert_eq!(call.tool_calls[0].arguments, "{\n  \"city\": \"Hanoi\"\n}");
        assert_eq!(call.stop.as_deref(), Some("tool_calls"));
        assert_eq!(
            (call.input_tokens, call.output_tokens),
            (Some(52), Some(18))
        );
    }

    #[test]
    fn a_whole_chat_completion_and_an_error() {
        let request = br#"{"model":"llama3","messages":[{"role":"user","content":"Hi"}]}"#;
        let response = br#"{"model":"llama3","choices":[{"message":{"role":"assistant","content":"Hello!","reasoning_content":"Greet back."},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
        let call = read(Provider::OpenAiChat, "", request, response).unwrap();
        assert_eq!(call.text, "Hello!");
        assert_eq!(call.thinking, "Greet back.");
        assert!(!call.stream && call.chunks == 0);
        let failed = read(
            Provider::OpenAiChat,
            "",
            request,
            br#"{"error":{"message":"Rate limit reached","type":"requests"}}"#,
        )
        .unwrap();
        assert_eq!(failed.error.as_deref(), Some("Rate limit reached"));
        assert!(read(Provider::OpenAiChat, "", b"not json", response).is_none());
    }

    #[test]
    fn a_streamed_anthropic_message_with_thinking_and_a_tool() {
        let request = br#"{"model":"claude-sonnet-4-5","max_tokens":1024,"stream":true,
            "system":[{"type":"text","text":"You are terse."}],
            "messages":[{"role":"user","content":"Find flights"},
                        {"role":"assistant","content":[{"type":"tool_use","id":"t0","name":"search","input":{"q":"HAN"}}]},
                        {"role":"user","content":[{"type":"tool_result","tool_use_id":"t0","content":"none"}]}],
            "tools":[{"name":"search","input_schema":{}},{"name":"book","input_schema":{}}]}"#;
        let response = sse(&[
            (
                "message_start",
                r#"{"type":"message_start","message":{"model":"claude-sonnet-4-5-20250929","usage":{"input_tokens":120,"output_tokens":1}}}"#,
            ),
            (
                "content_block_start",
                r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
            ),
            (
                "content_block_delta",
                r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Try another day."}}"#,
            ),
            (
                "content_block_stop",
                r#"{"type":"content_block_stop","index":0}"#,
            ),
            (
                "content_block_start",
                r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            ),
            (
                "content_block_delta",
                r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Trying tomorrow."}}"#,
            ),
            (
                "content_block_start",
                r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"t1","name":"search","input":{}}}"#,
            ),
            (
                "content_block_delta",
                r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"q\":\"HAN"}}"#,
            ),
            (
                "content_block_delta",
                r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":" tomorrow\"}"}}"#,
            ),
            (
                "message_delta",
                r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":64}}"#,
            ),
            ("message_stop", r#"{"type":"message_stop"}"#),
        ]);
        let call = read(Provider::Anthropic, "", request, &response).unwrap();
        assert_eq!(call.model.as_deref(), Some("claude-sonnet-4-5-20250929"));
        assert_eq!(call.messages[0].text, "You are terse.");
        assert_eq!(call.messages[2].text, "→ search({\"q\":\"HAN\"})");
        assert_eq!(call.messages[3].text, "← none");
        assert_eq!(call.tools, ["search", "book"]);
        assert_eq!(call.thinking, "Try another day.");
        assert_eq!(call.text, "Trying tomorrow.");
        assert_eq!(call.tool_calls[0].name, "search");
        assert_eq!(
            call.tool_calls[0].arguments,
            "{\n  \"q\": \"HAN tomorrow\"\n}"
        );
        assert_eq!(call.stop.as_deref(), Some("tool_use"));
        assert_eq!(
            (call.input_tokens, call.output_tokens),
            (Some(120), Some(64))
        );
    }

    #[test]
    fn a_whole_anthropic_message() {
        let request = br#"{"model":"claude-haiku-4-5","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}"#;
        let response = br#"{"model":"claude-haiku-4-5","content":[{"type":"text","text":"Hello."},{"type":"tool_use","id":"t","name":"wave","input":{"hand":"left"}}],"stop_reason":"tool_use","usage":{"input_tokens":8,"output_tokens":12}}"#;
        let call = read(Provider::Anthropic, "", request, response).unwrap();
        assert_eq!(call.text, "Hello.");
        assert_eq!(call.tool_calls[0].arguments, "{\n  \"hand\": \"left\"\n}");
        assert_eq!(call.output_tokens, Some(12));
    }

    #[test]
    fn a_streamed_openai_response_ends_as_its_completed_event_says() {
        let request = br#"{"model":"gpt-5","instructions":"Be kind.","input":"Hello","stream":true,"tools":[{"type":"web_search"}]}"#;
        let response = sse(&[
            (
                "response.created",
                r#"{"type":"response.created","response":{"model":"gpt-5"}}"#,
            ),
            (
                "response.output_text.delta",
                r#"{"type":"response.output_text.delta","delta":"Hi "}"#,
            ),
            (
                "response.output_text.delta",
                r#"{"type":"response.output_text.delta","delta":"there"}"#,
            ),
            (
                "response.completed",
                r#"{"type":"response.completed","response":{"model":"gpt-5-2025-08-07","status":"completed","output":[{"type":"reasoning","summary":[{"type":"summary_text","text":"Say hi."}]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Hi there"}]},{"type":"function_call","call_id":"c1","name":"lookup","arguments":"{\"x\":1}"}],"usage":{"input_tokens":9,"output_tokens":4}}}"#,
            ),
        ]);
        let call = read(Provider::OpenAiResponses, "", request, &response).unwrap();
        assert_eq!(call.messages[0].text, "Be kind.");
        assert_eq!(
            call.messages[1],
            Message {
                role: "user".into(),
                text: "Hello".into()
            }
        );
        assert_eq!(call.tools, ["web_search"]);
        assert_eq!(call.model.as_deref(), Some("gpt-5-2025-08-07"));
        assert_eq!(call.text, "Hi there");
        assert_eq!(call.thinking, "Say hi.");
        assert_eq!(call.tool_calls[0].name, "lookup");
        assert_eq!(call.stop.as_deref(), Some("completed"));
        assert_eq!(call.chunks, 4);
        assert_eq!(call.output_tokens, Some(4));
    }

    #[test]
    fn a_streamed_gemini_answer_as_events_and_as_a_list() {
        let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse";
        let request = br#"{"systemInstruction":{"parts":[{"text":"Be short."}]},"contents":[{"role":"user","parts":[{"text":"Hi"}]}],"tools":[{"functionDeclarations":[{"name":"now"}]}]}"#;
        let chunks = [
            r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Thinking…","thought":true}]}}],"modelVersion":"gemini-2.5-flash"}"#,
            r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"Hel"}]}}]}"#,
            r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"lo"},{"functionCall":{"name":"now","args":{}}}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":3}}"#,
        ];
        let events: Vec<(&str, &str)> = chunks.iter().map(|c| ("", *c)).collect();
        let call = read(Provider::Gemini, url, request, &sse(&events)).unwrap();
        assert_eq!(call.model.as_deref(), Some("gemini-2.5-flash"));
        assert!(call.stream);
        assert_eq!(call.messages[0].text, "Be short.");
        assert_eq!(call.tools, ["now"]);
        assert_eq!(call.thinking, "Thinking…");
        assert_eq!(call.text, "Hello");
        assert_eq!(call.tool_calls[0].name, "now");
        assert_eq!(call.stop.as_deref(), Some("STOP"));
        assert_eq!((call.input_tokens, call.output_tokens), (Some(5), Some(3)));
        let list = format!("[{}]", chunks.join(","));
        let same = read(Provider::Gemini, url, request, list.as_bytes()).unwrap();
        assert_eq!((same.text, same.chunks), ("Hello".to_owned(), 3));
    }

    #[test]
    fn an_ollama_stream_line_by_line() {
        let request = br#"{"model":"qwen3","messages":[{"role":"user","content":"Hi"}]}"#;
        let response = b"{\"model\":\"qwen3\",\"message\":{\"role\":\"assistant\",\"content\":\"\",\"thinking\":\"Hm.\"},\"done\":false}\n\
{\"model\":\"qwen3\",\"message\":{\"role\":\"assistant\",\"content\":\"Hey\"},\"done\":false}\n\
{\"model\":\"qwen3\",\"message\":{\"role\":\"assistant\",\"content\":\"!\"},\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":11,\"eval_count\":7}\n";
        let call = read(Provider::Ollama, "", request, response).unwrap();
        assert!(call.stream, "Ollama streams unless told not to");
        assert_eq!(call.chunks, 3);
        assert_eq!(call.thinking, "Hm.");
        assert_eq!(call.text, "Hey!");
        assert_eq!(call.stop.as_deref(), Some("stop"));
        assert_eq!((call.input_tokens, call.output_tokens), (Some(11), Some(7)));
    }

    #[test]
    fn a_stream_cut_short_keeps_what_came() {
        let request = br#"{"model":"m","stream":true,"messages":[]}"#;
        let response = sse(&[(
            "",
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c","function":{"name":"f","arguments":"{\"a\":"}}]}}]}"#,
        )]);
        let call = read(Provider::OpenAiChat, "", request, &response).unwrap();
        assert_eq!(
            call.tool_calls[0].arguments, "{\"a\":",
            "half written, as it came"
        );
        assert_eq!(call.stop, None);
    }
}
