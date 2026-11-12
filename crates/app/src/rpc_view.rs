//! The inspector's RPC part: what each JSON-RPC call of a request asks for,
//! with the hex read as what it means — the function a `data` field names,
//! the transaction behind 800 bytes of RLP, an amount in ether, the reason a
//! call reverted. And the Settings part that says which ABIs to name
//! contracts by.
//!
//! What to show is worked out by [`shown`], which touches no window: the view
//! only draws what it returns.

use crate::{colors::colors, select::Selection, traffic::Traffic, widgets::Reachable};
use gpui::{
    AppContext as _, Context, ElementId, FontWeight, IntoElement, PathPromptOptions, SharedString,
    div, prelude::*, px, rgb,
};
use std::{path::PathBuf, sync::Arc};
use tracewing_i18n::{tr, tr_n};
use tracewing_store::{
    abi::Abi,
    jsonrpc, session,
    session::BodyReader,
    web3::{self, Revert, Transaction},
};

/// The most of each body read for this: a call is small, and a reply that is
/// not is a page of logs the Body part already shows.
const BODY_MAX: usize = 4 << 20;
/// Past this many calls of a batch the rest are counted, not laid out.
const CALLS_SHOWN: usize = 50;
/// Past this many arguments the rest are counted: a call that takes more is
/// one the Body part reads better.
const ARGUMENTS_SHOWN: usize = 24;
/// The most of a socket's messages read for this. A socket that has said
/// more than this has said it in the Messages part, which reads them all.
const MESSAGES_MAX: i64 = 200;
/// How many of a reply's logs are said one by one.
const LOGS_SHOWN: usize = 12;
/// How many problems of one ABI source are shown in Settings.
const PROBLEMS_SHOWN: usize = 3;

/// One line of a section: what it is, and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The name the wire gives it (`to`, `value`, `chainId`), left as it
    /// comes: it is what a reader of the body will look for.
    pub label: String,
    pub value: String,
    /// Hex, an address or a signature, which read better in the mono font.
    pub code: bool,
}

impl Line {
    fn said(label: &str, value: impl Into<String>) -> Self {
        Self {
            label: label.to_owned(),
            value: value.into(),
            code: false,
        }
    }

    fn code(label: &str, value: impl Into<String>) -> Self {
        Self {
            code: true,
            ..Self::said(label, value)
        }
    }
}

/// One call of the request, or the reply to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub title: String,
    /// Said under the title when there is something to say about the whole.
    pub note: Option<String>,
    pub lines: Vec<Line>,
}

/// What a flow's calls hold, ready to draw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    pub sections: Vec<Section>,
    /// Calls past [`CALLS_SHOWN`], which are counted instead.
    pub more: usize,
}

/// What the calls in `request` ask for, and what `reply` answered them, with
/// contracts named by `abi` where it knows them.
pub fn shown(
    method: &str,
    content_type: Option<&str>,
    request: &[u8],
    reply: &[u8],
    abi: &Abi,
) -> Shown {
    let calls = jsonrpc::calls(method, content_type, request);
    let replies: Vec<serde_json::Value> = match serde_json::from_slice(reply) {
        Ok(serde_json::Value::Array(batch)) => batch,
        Ok(one) => vec![one],
        Err(_) => Vec::new(),
    };
    let batch = calls.len() > 1;
    let mut out = Shown {
        more: calls.len().saturating_sub(CALLS_SHOWN),
        ..Shown::default()
    };
    for (at, call) in calls.iter().take(CALLS_SHOWN).enumerate() {
        let title = match batch {
            true => format!("{} · {}", at + 1, call.method),
            false => call.method.clone(),
        };
        out.sections.push(Section {
            title,
            note: None,
            lines: asked(call, abi),
        });
        // The reply with the same id, which is what JSON-RPC matches by.
        let answer = match &call.id {
            Some(id) => replies.iter().find(|r| r.get("id") == Some(id)),
            None => None,
        }
        .or_else(|| (!batch).then(|| replies.first()).flatten());
        if let Some(answer) = answer {
            out.sections.push(answered(answer, abi, call));
        }
    }
    out
}

/// What the calls in a socket's messages hold. A WebSocket carries JSON-RPC
/// with no HTTP around it: a call a message, the reply somewhere after it
/// with the same id, and notifications the server sends of its own, which
/// have no id because nothing asked for each one.
///
/// `messages` are in the order they went, each said to be from the client or
/// not.
pub fn shown_messages(messages: &[(bool, Vec<u8>)], abi: &Abi) -> Shown {
    let mut out = Shown::default();
    // The server messages that answer a call, so that what is left over is
    // what the server said of its own accord.
    let mut answered_at = vec![false; messages.len()];
    let mut pairs: Vec<(usize, Option<usize>, jsonrpc::Call)> = Vec::new();
    for (at, (from_client, data)) in messages.iter().enumerate() {
        if !from_client {
            continue;
        }
        for call in jsonrpc::calls_in(data) {
            let reply = call.id.as_ref().and_then(|id| {
                messages
                    .iter()
                    .enumerate()
                    .skip(at + 1)
                    .find(|(n, (from, data))| !from && !answered_at[*n] && answers(data, id))
                    .map(|(n, _)| n)
            });
            if let Some(n) = reply {
                answered_at[n] = true;
            }
            pairs.push((at, reply, call));
        }
    }
    // Back in the order they went, with each call's reply after it.
    let mut shown = 0usize;
    for (at, (from_client, data)) in messages.iter().enumerate() {
        let here: Vec<&(usize, Option<usize>, jsonrpc::Call)> =
            pairs.iter().filter(|(from, _, _)| *from == at).collect();
        if *from_client {
            for (_, reply, call) in here {
                shown += 1;
                if shown > CALLS_SHOWN {
                    out.more += 1;
                    continue;
                }
                out.sections.push(Section {
                    title: call.method.clone(),
                    note: None,
                    lines: asked(call, abi),
                });
                if let Some(n) = reply
                    && let Ok(value) = serde_json::from_slice(&messages[*n].1)
                {
                    out.sections.push(answered(&value, abi, call));
                }
            }
            continue;
        }
        if answered_at[at] {
            continue;
        }
        // The server, of its own accord: what a subscription sends.
        for call in jsonrpc::calls_in(data) {
            shown += 1;
            if shown > CALLS_SHOWN {
                out.more += 1;
                continue;
            }
            out.sections.push(Section {
                title: call.method.clone(),
                note: Some(tr!("Sent by the server, not asked for")),
                lines: asked(&call, abi),
            });
        }
    }
    out
}

/// Whether a message is the reply to `id`.
fn answers(data: &[u8], id: &serde_json::Value) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) else {
        return false;
    };
    let is_reply = |one: &serde_json::Value| {
        one.get("id") == Some(id) && (one.get("result").is_some() || one.get("error").is_some())
    };
    match &value {
        serde_json::Value::Array(batch) => batch.iter().any(is_reply),
        one => is_reply(one),
    }
}

/// What one call asks for, by what its method does with its params.
fn asked(call: &jsonrpc::Call, abi: &Abi) -> Vec<Line> {
    let params = call.params.as_ref();
    let at = |n: usize| params.and_then(|p| p.get(n));
    let text_at = |n: usize| at(n).and_then(|v| v.as_str()).map(str::to_owned);
    let mut lines = Vec::new();
    match call.method.as_str() {
        "eth_call" | "eth_estimateGas" | "eth_createAccessList" => {
            let object = at(0);
            let field = |name: &str| {
                object
                    .and_then(|o| o.get(name))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            };
            lines.extend(field("from").map(|from| Line::code("from", address(&from))));
            lines.extend(field("to").map(|to| Line::code("to", address(&to))));
            lines.extend(
                field("value")
                    .filter(|value| value != "0x0")
                    .and_then(|value| web3::quantity_bytes(&value))
                    .map(|wei| Line::said("value", web3::ether(&wei))),
            );
            if let Some(data) = field("data").or_else(|| field("input")) {
                lines.extend(calldata(&data, abi));
            }
            lines.extend(text_at(1).map(|block| Line::said("block", block)));
        }
        "eth_sendRawTransaction" => {
            if let Some(raw) = text_at(0) {
                lines.extend(raw_transaction(&raw, abi));
            }
        }
        "eth_getLogs" | "eth_newFilter" => {
            let filter = at(0);
            let field = |name: &str| filter.and_then(|f| f.get(name));
            lines.extend(
                field("address")
                    .and_then(|v| v.as_str())
                    .map(|a| Line::code("address", address(a))),
            );
            for name in ["fromBlock", "toBlock", "blockHash"] {
                lines.extend(
                    field(name)
                        .and_then(|v| v.as_str())
                        .map(|v| Line::said(name, block_text(v))),
                );
            }
            if let Some(topics) = field("topics").and_then(|t| t.as_array()) {
                for (n, topic) in topics.iter().enumerate() {
                    let Some(hex) = topic.as_str() else {
                        lines.push(Line::said(&format!("topic {n}"), tr!("any")));
                        continue;
                    };
                    lines.push(Line::code(&format!("topic {n}"), topic_text(hex, abi, n)));
                }
            }
        }
        "eth_subscribe" => {
            lines.extend(text_at(0).map(|what| Line::said("kind", what)));
            if let Some(filter) = at(1) {
                lines.extend(
                    filter
                        .get("address")
                        .and_then(|v| v.as_str())
                        .map(|a| Line::code("address", address(a))),
                );
                if let Some(topics) = filter.get("topics").and_then(|t| t.as_array()) {
                    for (n, topic) in topics.iter().enumerate() {
                        if let Some(hex) = topic.as_str() {
                            lines.push(Line::code(&format!("topic {n}"), topic_text(hex, abi, n)));
                        }
                    }
                }
            }
        }
        "eth_unsubscribe" => {
            lines.extend(text_at(0).map(|id| Line::code("subscription", id)));
        }
        // What a subscription sends: the id it belongs to, then the block or
        // the log itself.
        "eth_subscription" => {
            let field = |name: &str| params.and_then(|p| p.get(name));
            lines.extend(
                field("subscription")
                    .and_then(|v| v.as_str())
                    .map(|id| Line::code("subscription", id)),
            );
            let result = field("result");
            let said = |name: &str| result.and_then(|r| r.get(name)).and_then(|v| v.as_str());
            // A block has a number, which reads as one; a hash does not.
            lines.extend(said("number").map(|n| Line::code("block", block_text(n))));
            lines.extend(said("hash").map(|hash| Line::code("hash", hash)));
            lines.extend(
                result
                    .and_then(|r| r.get("address"))
                    .and_then(|v| v.as_str())
                    .map(|a| Line::code("address", address(a))),
            );
            if let Some(topics) = result
                .and_then(|r| r.get("topics"))
                .and_then(|t| t.as_array())
            {
                for (n, topic) in topics.iter().enumerate() {
                    if let Some(hex) = topic.as_str() {
                        lines.push(Line::code(&format!("topic {n}"), topic_text(hex, abi, n)));
                    }
                }
            }
        }
        "eth_getBalance" | "eth_getTransactionCount" | "eth_getCode" => {
            lines.extend(text_at(0).map(|who| Line::code("address", address(&who))));
            lines.extend(text_at(1).map(|block| Line::said("block", block_text(&block))));
        }
        "eth_getStorageAt" => {
            lines.extend(text_at(0).map(|who| Line::code("address", address(&who))));
            lines.extend(text_at(1).map(|slot| Line::code("slot", number_text(&slot))));
            lines.extend(text_at(2).map(|block| Line::said("block", block_text(&block))));
        }
        "eth_getBlockByNumber" | "eth_getBlockReceipts" => {
            lines.extend(text_at(0).map(|block| Line::said("block", block_text(&block))));
        }
        "eth_getTransactionByHash" | "eth_getTransactionReceipt" | "eth_getBlockByHash" => {
            lines.extend(text_at(0).map(|hash| Line::code("hash", hash)));
        }
        // Everything else: the params as they came, with hex read where it is
        // a number.
        _ => {
            if let Some(serde_json::Value::Array(list)) = params {
                for (n, param) in list.iter().enumerate() {
                    let label = format!("param {n}");
                    match param.as_str() {
                        Some(text) => lines.push(Line::code(&label, number_text(text))),
                        None => lines.push(Line::code(&label, param.to_string())),
                    }
                }
            }
        }
    }
    lines
}

/// What the reply to one call holds: what came back, or why it did not.
/// `method` is the call that asked, which is what says how to read a result.
fn answered(answer: &serde_json::Value, abi: &Abi, call: &jsonrpc::Call) -> Section {
    let method = call.method.as_str();
    if let Some(error) = answer.get("error") {
        let code = error.get("code").and_then(serde_json::Value::as_i64);
        let mut lines = Vec::new();
        lines.extend(code.map(|code| Line::said("code", jsonrpc::code_text(code))));
        lines.extend(
            error
                .get("message")
                .and_then(|m| m.as_str())
                .map(|message| Line::said("message", message)),
        );
        if let Some(data) = error.get("data").and_then(|d| d.as_str()) {
            lines.extend(revert(data, abi));
        }
        return Section {
            title: tr!("Failed"),
            note: Some(tr!(
                "The server answered 200: a call that fails says so in the body."
            )),
            lines,
        };
    }
    let mut lines = Vec::new();
    match answer.get("result") {
        Some(serde_json::Value::String(hex)) => {
            lines.push(Line::code("result", result_text(method, hex)));
            // A call to a contract answers in words whose meaning is in the
            // function's own types, which an ABI has and the hex has not.
            lines.extend(answered_by_types(call, abi, hex));
        }
        Some(serde_json::Value::Array(list)) => {
            lines.push(Line::said(
                "result",
                tr_n!(
                    list.len(),
                    "{count} item",
                    "{count} items",
                    count = list.len()
                ),
            ));
            lines.extend(logs_in(list, abi));
        }
        Some(serde_json::Value::Object(fields)) => {
            let said = object_lines(method, fields, abi);
            match said.is_empty() {
                true => lines.push(Line::said(
                    "result",
                    tr_n!(
                        fields.len(),
                        "{count} field",
                        "{count} fields",
                        count = fields.len()
                    ),
                )),
                false => lines.extend(said),
            }
            // A receipt carries the logs of what the transaction did.
            if let Some(logs) = fields.get("logs").and_then(|l| l.as_array()) {
                lines.extend(logs_in(logs, abi));
            }
        }
        Some(other) => lines.push(Line::code("result", other.to_string())),
        None => {}
    }
    Section {
        title: tr!("Answer"),
        note: None,
        lines,
    }
}

/// What an `eth_call` came back with, read by the types the function it
/// called answers with. Nothing when the function is not one that is known,
/// which is where an ABI of the user's says more than the shared list.
fn answered_by_types(call: &jsonrpc::Call, abi: &Abi, hex: &str) -> Vec<Line> {
    if call.method != "eth_call" {
        return Vec::new();
    }
    let data = call
        .params
        .as_ref()
        .and_then(|params| params.get(0))
        .and_then(|object| object.get("data").or_else(|| object.get("input")))
        .and_then(|data| data.as_str())
        .and_then(web3::bytes_of)
        .unwrap_or_default();
    let types = abi
        .returns(&data)
        .map(str::to_owned)
        .or_else(|| web3::known_returns(&data).map(str::to_owned));
    let Some(types) = types.filter(|types| types != "()") else {
        return Vec::new();
    };
    let Some(bytes) = web3::bytes_of(hex) else {
        return Vec::new();
    };
    let name = |data: &[u8]| {
        abi.function(data)
            .map(str::to_owned)
            .or_else(|| web3::known_function(data).map(str::to_owned))
    };
    web3::returned_named(&types, &bytes, &name)
        .unwrap_or_default()
        .into_iter()
        .map(|one| Line::code(&one.kind, one.text))
        .collect()
}

/// The logs in a reply, each said by the event it is. A log's topics and its
/// data are two halves of one thing, and neither reads alone.
fn logs_in(list: &[serde_json::Value], abi: &Abi) -> Vec<Line> {
    let mut lines = Vec::new();
    for log in list.iter().take(LOGS_SHOWN) {
        let topics: Vec<Vec<u8>> = log
            .get("topics")
            .and_then(|topics| topics.as_array())
            .map(|topics| {
                topics
                    .iter()
                    .filter_map(|topic| topic.as_str().and_then(web3::bytes_of))
                    .collect()
            })
            .unwrap_or_default();
        let Some(first) = topics.first() else {
            continue;
        };
        let Some(signature) = abi.event(first).or_else(|| web3::known_event(first)) else {
            continue;
        };
        let data = log
            .get("data")
            .and_then(|data| data.as_str())
            .and_then(web3::bytes_of)
            .unwrap_or_default();
        let said = web3::event_arguments(signature, abi.indexed(first), &topics, &data)
            .map(|read| {
                read.iter()
                    .map(|one| one.text.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        lines.push(Line::code(
            web3::name_of(signature),
            match said.is_empty() {
                true => signature.to_owned(),
                false => format!("({said})"),
            },
        ));
    }
    let more = list.len().saturating_sub(LOGS_SHOWN);
    if more > 0 {
        lines.push(Line::said("", tr!("and {count} more", count = more)));
    }
    lines
}

/// What an object a node answered with holds, where its shape is known: a
/// receipt, a transaction, a block. Empty for one that is not, which is then
/// counted as before.
fn object_lines(
    method: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
    abi: &Abi,
) -> Vec<Line> {
    match method {
        "eth_getTransactionReceipt" => receipt_lines(fields),
        "eth_getTransactionByHash"
        | "eth_getTransactionByBlockHashAndIndex"
        | "eth_getTransactionByBlockNumberAndIndex" => transaction_lines(fields, abi),
        "eth_getBlockByNumber" | "eth_getBlockByHash" => block_lines(fields),
        _ => Vec::new(),
    }
}

/// A receipt: whether it went through at all, what it cost, and what it
/// made. Its `status` is the whole point of asking for one.
fn receipt_lines(fields: &serde_json::Map<String, serde_json::Value>) -> Vec<Line> {
    let said = |name: &str| fields.get(name).and_then(|v| v.as_str());
    let mut lines = Vec::new();
    // A transaction that was mined and did not go through costs its gas and
    // looks like any other until this is read.
    lines.extend(said("status").map(|status| match status {
        "0x1" => Line::said("status", tr!("succeeded")),
        "0x0" => Line::said(
            "status",
            tr!("failed: it was mined, and what it asked for did not happen"),
        ),
        other => Line::code("status", other.to_owned()),
    }));
    lines.extend(said("blockNumber").map(|at| Line::said("blockNumber", block_text(at))));
    lines.extend(said("gasUsed").map(|gas| Line::said("gasUsed", block_text(gas))));
    lines.extend(
        said("effectiveGasPrice")
            .and_then(web3::quantity)
            .map(|wei| Line::said("effectiveGasPrice", web3::gwei(wei))),
    );
    // A deployment: where the contract it made now lives.
    lines.extend(said("contractAddress").map(|made| Line::code("contractAddress", address(made))));
    lines
}

/// A transaction as the node reads it, which is the same handful of fields
/// its RLP holds, and the call its input makes.
fn transaction_lines(fields: &serde_json::Map<String, serde_json::Value>, abi: &Abi) -> Vec<Line> {
    let said = |name: &str| fields.get(name).and_then(|v| v.as_str());
    let mut lines = Vec::new();
    lines.extend(
        said("type")
            .and_then(web3::quantity)
            .map(|kind| Line::said("type", kind_text(kind as u8))),
    );
    lines.extend(
        said("chainId")
            .and_then(web3::quantity)
            .map(|id| Line::said("chainId", chain_text(id))),
    );
    lines.extend(said("nonce").map(|nonce| Line::said("nonce", block_text(nonce))));
    lines.extend(said("from").map(|from| Line::code("from", address(from))));
    lines.push(match said("to") {
        Some(to) => Line::code("to", address(to)),
        None => Line::said("to", tr!("none: it makes a contract")),
    });
    lines.extend(
        said("value")
            .and_then(web3::quantity_bytes)
            .map(|wei| Line::said("value", web3::ether(&wei))),
    );
    lines.extend(said("gas").map(|gas| Line::said("gas", block_text(gas))));
    for (name, label) in [
        ("gasPrice", "gasPrice"),
        ("maxFeePerGas", "maxFeePerGas"),
        ("maxPriorityFeePerGas", "maxPriorityFeePerGas"),
    ] {
        lines.extend(
            said(name)
                .and_then(web3::quantity)
                .map(|wei| Line::said(label, web3::gwei(wei))),
        );
    }
    if let Some(input) = said("input").filter(|input| *input != "0x") {
        lines.extend(calldata(input, abi));
    }
    lines
}

/// A block: which one, when, and how much is in it.
fn block_lines(fields: &serde_json::Map<String, serde_json::Value>) -> Vec<Line> {
    let said = |name: &str| fields.get(name).and_then(|v| v.as_str());
    let mut lines = Vec::new();
    lines.extend(said("number").map(|at| Line::said("number", block_text(at))));
    lines.extend(said("hash").map(|hash| Line::code("hash", hash.to_owned())));
    lines.extend(said("gasUsed").map(|gas| Line::said("gasUsed", block_text(gas))));
    lines.extend(
        said("baseFeePerGas")
            .and_then(web3::quantity)
            .map(|wei| Line::said("baseFeePerGas", web3::gwei(wei))),
    );
    lines.extend(
        fields
            .get("transactions")
            .and_then(|list| list.as_array())
            .map(|list| {
                Line::said(
                    "transactions",
                    tr_n!(
                        list.len(),
                        "{count} item",
                        "{count} items",
                        count = list.len()
                    ),
                )
            }),
    );
    lines
}

/// What a result holds, which only the call that asked can say: a block
/// number is a number, a subscription's id is a handle that means nothing as
/// digits, and a hash is neither. Hex is left as it came unless the method is
/// one whose answer is a quantity.
fn result_text(method: &str, hex: &str) -> String {
    match method {
        "eth_getBalance" => match web3::quantity_bytes(hex) {
            Some(wei) => web3::ether(&wei),
            None => hex.to_owned(),
        },
        "eth_gasPrice" | "eth_maxPriorityFeePerGas" | "eth_blobBaseFee" => {
            match web3::quantity(hex) {
                Some(wei) => web3::gwei(wei),
                None => hex.to_owned(),
            }
        }
        "eth_blockNumber"
        | "eth_chainId"
        | "eth_getTransactionCount"
        | "eth_estimateGas"
        | "eth_getBlockTransactionCountByNumber"
        | "net_version"
        // A word of data, whose digits are what the contract answered.
        | "eth_call" => number_text(hex),
        _ => hex.to_owned(),
    }
}

/// A `data` field: the function it names, then its arguments.
fn calldata(data: &str, abi: &Abi) -> Vec<Line> {
    let Some(bytes) = web3::bytes_of(data) else {
        return vec![Line::code("data", data.to_owned())];
    };
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let signature = abi
        .function(&bytes)
        .map(str::to_owned)
        .or_else(|| web3::known_function(&bytes).map(str::to_owned));
    match &signature {
        Some(signature) => lines.push(Line::code("function", signature.clone())),
        None => lines.push(Line::code(
            "selector",
            match bytes.get(..4) {
                Some(four) => web3::hex_text(four),
                None => web3::hex_text(&bytes),
            },
        )),
    }
    // An ABI of the user's names the calls inside a multicall too, not just
    // the one at the top.
    let name = |data: &[u8]| {
        abi.function(data)
            .map(str::to_owned)
            .or_else(|| web3::known_function(data).map(str::to_owned))
    };
    let arguments = signature
        .as_deref()
        .and_then(|signature| web3::arguments_named(signature, &bytes, &name));
    match arguments {
        Some(arguments) => {
            let more = arguments.len().saturating_sub(ARGUMENTS_SHOWN);
            for argument in arguments.into_iter().take(ARGUMENTS_SHOWN) {
                lines.push(Line::code(&argument.kind, argument.text));
            }
            if more > 0 {
                lines.push(Line::said("", tr!("and {count} more", count = more)));
            }
        }
        None => lines.push(Line::said(
            "arguments",
            tr_n!(
                bytes.len().saturating_sub(4),
                "{count} byte, which the signature does not say how to read",
                "{count} bytes, which the signature does not say how to read",
                count = bytes.len().saturating_sub(4)
            ),
        )),
    }
    lines
}

/// A signed transaction, as the hex of `eth_sendRawTransaction` holds it.
fn raw_transaction(raw: &str, abi: &Abi) -> Vec<Line> {
    let Some(bytes) = web3::bytes_of(raw) else {
        return vec![Line::code("transaction", raw.to_owned())];
    };
    let Some(tx) = Transaction::decode(&bytes) else {
        return vec![
            Line::said(
                "transaction",
                tr_n!(
                    bytes.len(),
                    "{count} byte that does not read as one",
                    "{count} bytes that do not read as one",
                    count = bytes.len()
                ),
            ),
            Line::code("raw", raw.to_owned()),
        ];
    };
    let mut lines = vec![Line::said("type", kind_text(tx.kind))];
    lines.extend(tx.chain_id.map(|id| Line::said("chainId", chain_text(id))));
    lines.push(Line::said("nonce", tx.nonce.to_string()));
    // Not in the transaction at all: worked out from the signature, and the
    // first thing anyone looking at one wants to know.
    lines.extend(tx.from.clone().map(|from| Line::code("from", from)));
    lines.push(match &tx.to {
        Some(to) => Line::code("to", to.clone()),
        None => Line::said("to", tr!("none: it makes a contract")),
    });
    lines.push(Line::said("value", web3::ether(&tx.value)));
    lines.push(Line::said("gas", tx.gas.to_string()));
    lines.extend(
        tx.gas_price
            .map(|price| Line::said("gasPrice", web3::gwei(price))),
    );
    lines.extend(
        tx.max_fee
            .map(|fee| Line::said("maxFeePerGas", web3::gwei(fee))),
    );
    lines.extend(
        tx.max_priority
            .map(|tip| Line::said("maxPriorityFeePerGas", web3::gwei(tip))),
    );
    if tx.access_list > 0 {
        lines.push(Line::said(
            "accessList",
            tr_n!(
                tx.access_list,
                "{count} place",
                "{count} places",
                count = tx.access_list
            ),
        ));
    }
    if tx.blobs > 0 {
        lines.push(Line::said(
            "blobs",
            tr_n!(tx.blobs, "{count} blob", "{count} blobs", count = tx.blobs),
        ));
    }
    if !tx.data.is_empty() {
        lines.extend(calldata(&web3::hex_text(&tx.data), abi));
    }
    lines
}

/// Why a call reverted, from the data its error carried.
fn revert(data: &str, abi: &Abi) -> Vec<Line> {
    let Some(bytes) = web3::bytes_of(data) else {
        return Vec::new();
    };
    match Revert::of(&bytes) {
        Some(Revert::Custom(four)) => {
            let named = abi.error(&four);
            vec![match named {
                Some(signature) => Line::code("error", signature.to_owned()),
                None => Line::code("error", web3::hex_text(&four)),
            }]
        }
        Some(found) => vec![Line::said("reverted", found.text())],
        None => Vec::new(),
    }
}

/// `0x1` as `1 (Ethereum)`, for the chains most traffic goes to.
fn chain_text(id: u128) -> String {
    match chain_name(id) {
        Some(name) => format!("{id} ({name})"),
        None => id.to_string(),
    }
}

/// The chains whose names a reader is likely to want. Not a list to keep up
/// to date: an unknown one shows its number, which is no worse than it was.
fn chain_name(id: u128) -> Option<&'static str> {
    Some(match id {
        1 => "Ethereum",
        10 => "OP Mainnet",
        56 => "BNB Chain",
        100 => "Gnosis",
        137 => "Polygon",
        8453 => "Base",
        42161 => "Arbitrum One",
        43114 => "Avalanche",
        11155111 => "Sepolia",
        31337 => "Anvil or Hardhat",
        1337 => "Ganache",
        _ => return None,
    })
}

fn kind_text(kind: u8) -> String {
    match kind {
        0 => tr!("0, the kind from before EIP-2718"),
        1 => "1 (EIP-2930)".to_owned(),
        2 => "2 (EIP-1559)".to_owned(),
        3 => "3 (EIP-4844)".to_owned(),
        4 => "4 (EIP-7702)".to_owned(),
        other => other.to_string(),
    }
}

/// An address with the capitals that check it, or as it came if it is not
/// one.
fn address(hex: &str) -> String {
    match web3::bytes_of(hex).filter(|bytes| bytes.len() == 20) {
        Some(bytes) => web3::address_text(&bytes),
        None => hex.to_owned(),
    }
}

/// A block, which comes as a number in hex or as a word like `latest`.
fn block_text(hex: &str) -> String {
    match web3::quantity(hex) {
        Some(number) => crate::flows::group_digits(number as u64),
        None => hex.to_owned(),
    }
}

/// Hex as the number it is, with the hex kept when it is not a short one: a
/// word of 32 bytes is a number, an address or a hash, and only the first is
/// worth reading as digits.
fn number_text(hex: &str) -> String {
    let Some(bytes) = web3::quantity_bytes(hex) else {
        return hex.to_owned();
    };
    // A hash and an address are not numbers anybody reads as digits.
    if bytes.len() == 20 || bytes.len() > 32 {
        return hex.to_owned();
    }
    let digits = web3::digits(&bytes);
    if digits == hex.trim_start_matches("0x").trim_start_matches('0') && digits.len() < 2 {
        return hex.to_owned();
    }
    format!("{hex} = {digits}")
}

/// A topic: the event its first one names, else the hash as it came.
fn topic_text(hex: &str, abi: &Abi, at: usize) -> String {
    if at > 0 {
        return hex.to_owned();
    }
    let Some(bytes) = web3::bytes_of(hex) else {
        return hex.to_owned();
    };
    match abi.event(&bytes).or_else(|| web3::known_event(&bytes)) {
        Some(signature) => signature.to_owned(),
        None => hex.to_owned(),
    }
}

impl Traffic {
    /// Reads what the chosen flow's calls hold, if it made any.
    pub(crate) fn load_rpc(&mut self, cx: &mut Context<Self>) {
        self.inspector.rpc = None;
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        if detail.kind == "websocket" {
            self.load_rpc_messages(cx);
            return;
        }
        if detail.rpc.is_none() {
            return;
        }
        let id = detail.id;
        let method = detail.method.clone();
        let content_type = detail.req_content_type.clone();
        let request = detail.request.stored.clone();
        let response = detail.response.stored.clone();
        let session = self.session_dir().to_owned();
        let abis = Arc::clone(&self.abis);
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_spawn(async move {
                    let mut reader = BodyReader::new(&session);
                    let mut whole = |stored: &session::StoredBody| {
                        if stored.exists() {
                            reader.unpacked(stored, BODY_MAX).unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    };
                    let (request, response) = (whole(&request), whole(&response));
                    // Reads the disk the first time, so not on the UI thread.
                    let abi = abis.abi();
                    shown(&method, content_type.as_deref(), &request, &response, &abi)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.inspector.detail.as_ref().map(|d| d.id) == Some(id) {
                    this.inspector.rpc = Some(read);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The same for a socket, whose calls are its messages. The list of them
    /// is read here, where the session's database is; only their bytes are
    /// read on another thread.
    fn load_rpc_messages(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let id = detail.id;
        let Some(flows) = self.flows() else {
            return;
        };
        let stored = match session::stored_messages(flows.conn(), id, 0, MESSAGES_MAX) {
            Ok(stored) => stored,
            Err(_) => return,
        };
        let places: Vec<(bool, Option<session::LogPlace>)> = stored
            .iter()
            .filter(|message| matches!(message.kind.as_str(), "text" | "binary"))
            .map(|message| (message.from_client, message.place))
            .collect();
        if places.is_empty() {
            return;
        }
        let session = self.session_dir().to_owned();
        let abis = Arc::clone(&self.abis);
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_spawn(async move {
                    let mut reader = BodyReader::new(&session);
                    let messages: Vec<(bool, Vec<u8>)> = places
                        .into_iter()
                        .map(|(from_client, place)| {
                            let data = place
                                .and_then(|place| reader.read_place(place).ok())
                                .unwrap_or_default();
                            (from_client, data)
                        })
                        .collect();
                    shown_messages(&messages, &abis.abi())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.inspector.detail.as_ref().map(|d| d.id) == Some(id) {
                    this.inspector.rpc = Some(read);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The calls of the chosen flow, their text selectable with `sel`.
    pub(crate) fn render_rpc(&self, sel: &Selection) -> impl IntoElement {
        let c = colors();
        let mut view = div()
            .id("rpc")
            .size_full()
            .overflow_y_scroll()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(4.));
        let Some(shown) = &self.inspector.rpc else {
            return view.child(
                div()
                    .text_color(rgb(c.muted))
                    .child(tr!("Reading the call…")),
            );
        };
        if shown.sections.is_empty() {
            return view.child(div().text_color(rgb(c.muted)).child(tr!(
                "The bodies were not kept, so what the call sent cannot be shown."
            )));
        }
        for (at, section) in shown.sections.iter().enumerate() {
            view = view.child(
                div()
                    .when(at > 0, |d| d.pt(px(12.)))
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(c.muted))
                    // Left as it is: a method name is what a reader looks
                    // for, and `ETH_SENDRAWTRANSACTION` is not that name.
                    .child(sel.piece(
                        section.title.clone(),
                        if at == 0 { "" } else { "\n\n" },
                        Vec::new(),
                    )),
            );
            view = view.children(section.note.clone().map(|note| {
                div()
                    .pb(px(2.))
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(sel.piece(note, "\n", Vec::new()))
            }));
            for line in &section.lines {
                let value = div()
                    .flex_1()
                    .min_w_0()
                    .when(line.code, |d| {
                        d.font_family(crate::MONO_FONT)
                            .font_features(gpui::FontFeatures::disable_ligatures())
                            .text_size(px(12.))
                    })
                    .child(sel.piece(line.value.clone(), "\n", Vec::new()));
                view = view.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap(px(10.))
                        .py(px(1.))
                        .child(
                            div()
                                .w(px(150.))
                                .flex_shrink_0()
                                .text_color(rgb(c.muted))
                                .child(sel.piece(line.label.clone(), "\t", Vec::new())),
                        )
                        .child(value),
                );
            }
        }
        if shown.more > 0 {
            view = view.child(div().pt(px(12.)).text_color(rgb(c.muted)).child(sel.piece(
                tr!("and {count} more calls", count = shown.more),
                "\n\n",
                Vec::new(),
            )));
        }
        view
    }

    /// Reads the ABIs chosen, so that Settings can say what is in them.
    pub fn read_abis(&mut self, cx: &mut Context<Self>) {
        if self.abis_busy {
            return;
        }
        self.abis_busy = true;
        let abis = Arc::clone(&self.abis);
        cx.spawn(async move |this, cx| {
            let abi = cx.background_spawn(async move { abis.abi() }).await;
            let _ = this.update(cx, |this, cx| {
                this.abis_busy = false;
                this.abis_read = Some(abi);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn add_abis(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.change_abis(cx, move |abis| abis.add(paths));
    }

    pub fn remove_abi(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.change_abis(cx, move |abis| abis.remove(&path));
    }

    fn change_abis(
        &mut self,
        cx: &mut Context<Self>,
        change: impl FnOnce(&tracewing_store::abi::Library) -> std::io::Result<Arc<Abi>>
        + Send
        + 'static,
    ) {
        self.abis_busy = true;
        let abis = Arc::clone(&self.abis);
        cx.spawn(async move |this, cx| {
            let done = cx.background_spawn(async move { change(&abis) }).await;
            let _ = this.update(cx, |this, cx| {
                this.abis_busy = false;
                match done {
                    Ok(abi) => {
                        this.abis_read = Some(abi);
                        this.abis_problem = None;
                        // The calls shown may read differently now.
                        this.load_rpc(cx);
                    }
                    Err(e) => {
                        this.abis_problem =
                            Some(tr!("The list was not saved: {e}", e = e.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn ask_for_abis(&mut self, folders: bool, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: !folders,
            directories: folders,
            multiple: true,
            prompt: Some(tr!("Add").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = chosen.await
                && !paths.is_empty()
            {
                let _ = this.update(cx, |this, cx| this.add_abis(paths, cx));
            }
        })
        .detach();
    }

    /// Settings' part: which ABIs name the contracts in a call.
    pub(crate) fn render_abis(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let sources = self.abis.sources();
        if self.abis_read.is_none() && !sources.is_empty() {
            self.read_abis(cx);
        }
        let read = self.abis_read.clone();
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .h(px(32.))
                .px(px(14.))
                .rounded(px(16.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(px(12.5))
                .font_weight(FontWeight::BOLD)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(label)
        };
        let rows = sources.iter().enumerate().map(|(i, path)| {
            let report = read
                .as_ref()
                .and_then(|abi| abi.sources.iter().find(|s| &s.path == path));
            let summary: SharedString = match report {
                None => tr!("Reading…").into(),
                Some(report) => [
                    tr_n!(
                        report.files,
                        "{count} file",
                        "{count} files",
                        count = report.files
                    ),
                    tr_n!(
                        report.functions,
                        "{count} function",
                        "{count} functions",
                        count = report.functions
                    ),
                    tr_n!(
                        report.events,
                        "{count} event",
                        "{count} events",
                        count = report.events
                    ),
                    tr_n!(
                        report.errors,
                        "{count} error",
                        "{count} errors",
                        count = report.errors
                    ),
                ]
                .join(" · ")
                .into(),
            };
            let problems = report.map(|r| r.problems.clone()).unwrap_or_default();
            let more = problems.len().saturating_sub(PROBLEMS_SHOWN);
            let remove_path = path.clone();
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(10.))
                .p(px(10.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .text_size(px(12.))
                                .child(path.display().to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(c.muted))
                                .child(summary),
                        )
                        .children(problems.into_iter().take(PROBLEMS_SHOWN).map(|problem| {
                            div()
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .text_size(px(11.5))
                                .text_color(rgb(c.error))
                                .child(problem)
                        }))
                        .when(more > 0, |d| {
                            d.child(div().text_size(px(11.5)).text_color(rgb(c.error)).child(
                                tr_n!(
                                    more,
                                    "and {count} more problem",
                                    "and {count} more problems",
                                    count = more
                                ),
                            ))
                        }),
                )
                .child(
                    div()
                        .id(ElementId::NamedInteger("abi-remove".into(), i as u64))
                        .flex_shrink_0()
                        .h(px(28.))
                        .px(px(12.))
                        .rounded(px(14.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .reachable()
                        .text_size(px(12.))
                        .font_weight(FontWeight::BOLD)
                        .bg(rgb(c.chip))
                        .text_color(rgb(c.on_chip))
                        .child(tr!("Remove"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_abi(remove_path.clone(), cx)
                        })),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(crate::widgets::eyebrow(tr!("Contract ABIs")).px(px(6.)))
            .child(
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("Folders or files of ABI JSON, as solc, Hardhat and Foundry write it, to name the functions, events and errors in a JSON-RPC call instead of showing four bytes. They are read again when they change. Nothing is sent anywhere: a signature is worked out from the ABI on this machine.")),
            )
            .children(rows)
            .children(self.abis_problem.clone().map(|problem| {
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(c.error))
                    .child(problem)
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        button("abis-add-folder", tr!("Add a folder…"))
                            .on_click(cx.listener(|this, _, _, cx| this.ask_for_abis(true, cx))),
                    )
                    .child(
                        button("abis-add-files", tr!("Add files…"))
                            .on_click(cx.listener(|this, _, _, cx| this.ask_for_abis(false, cx))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The samples are the ones the store's own tests read: real traffic,
    /// captured from Ethereum mainnet. See
    /// `crates/store/testdata/jsonrpc/README.md`.
    macro_rules! sample {
        ($name:literal) => {
            include_bytes!(concat!("../../store/testdata/jsonrpc/", $name))
        };
    }

    fn of(request: &[u8], reply: &[u8]) -> Shown {
        shown(
            "POST",
            Some("application/json"),
            request,
            reply,
            &Abi::default(),
        )
    }

    fn titles(shown: &Shown) -> Vec<&str> {
        shown
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect()
    }

    /// The label and value of each line, for reading an expectation whole.
    fn lines(section: &Section) -> Vec<(&str, &str)> {
        section
            .lines
            .iter()
            .map(|line| (line.label.as_str(), line.value.as_str()))
            .collect()
    }

    #[test]
    fn an_eth_call_reads_as_the_function_it_calls() {
        let shown = of(
            sample!("call_balanceof.req.json"),
            sample!("call_balanceof.res.json"),
        );
        assert_eq!(shown.sections.len(), 2, "what was asked, and the answer");
        let asked = &shown.sections[0];
        assert_eq!(asked.title, "eth_call");
        assert_eq!(
            lines(asked),
            [
                ("to", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
                ("function", "balanceOf(address)"),
                ("address", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
                ("block", "latest"),
            ]
        );
        let answer = &shown.sections[1];
        assert_eq!(answer.title, "Answer");
        assert_eq!(
            answer.lines[0].value,
            "0x000000000000000000000000000000000000000000000000000000000256063c = 39192124",
            "the hex stays, and what it comes to is said"
        );
    }

    /// The word an eth_call comes back with means nothing on its own. The
    /// function it called says what it is.
    #[test]
    fn what_a_call_answered_is_read_as_what_it_asked_for() {
        let shown = of(
            sample!("call_balanceof.req.json"),
            sample!("call_balanceof.res.json"),
        );
        let answer = &shown.sections[1];
        assert_eq!(
            lines(answer),
            [
                (
                    "result",
                    "0x000000000000000000000000000000000000000000000000000000000256063c = 39192124"
                ),
                // balanceOf answers with a uint256, which the shared list
                // knows without any ABI.
                ("uint256", "39192124"),
            ]
        );
    }

    /// A receipt's logs are what the transaction actually did; before this
    /// they were counted and left at that.
    /// A transaction that was mined and did not go through costs its gas and
    /// looks like any other, unless the receipt's status is read. This one
    /// really failed on mainnet.
    #[test]
    fn a_transaction_that_did_not_go_through_says_so() {
        let asked = br#"{"jsonrpc":"2.0","id":1,"method":"eth_getTransactionReceipt",
                         "params":["0x6dd4134ad889bbb0fd2589146eac5c6b73a1f8c5f23f47924f09fc9a2cca1708"]}"#;
        let shown = of(asked, sample!("receipt_failed.res.json"));
        let lines = lines(&shown.sections[1]);
        assert_eq!(
            lines[0],
            (
                "status",
                "failed: it was mined, and what it asked for did not happen"
            )
        );
        assert!(
            lines.contains(&("gasUsed", "552,030")),
            "and it was paid for anyway: {lines:?}"
        );
        assert!(
            !lines.iter().any(|(label, _)| *label == "result"),
            "the count it used to show is gone, not kept beside it: {lines:?}"
        );
    }

    #[test]
    fn a_receipt_that_went_through_says_that_too() {
        let asked = br#"{"jsonrpc":"2.0","id":1,"method":"eth_getTransactionReceipt",
                         "params":["0xc0e96fa11b3d298c3f37b7fc2fc43dc8ac11d9e0fab3f3d3816d8b2497757b6e"]}"#;
        let shown = of(asked, sample!("receipt.res.json"));
        let lines = lines(&shown.sections[1]);
        assert_eq!(lines[0], ("status", "succeeded"));
        // And the logs are still said under it.
        assert_eq!(
            lines
                .iter()
                .filter(|(label, _)| *label == "Transfer")
                .count(),
            2
        );
    }

    /// The node's own view of a transaction holds the same fields its RLP
    /// does, and the call its input makes.
    #[test]
    fn a_transaction_a_node_answered_with_is_laid_out_too() {
        let asked = br#"{"jsonrpc":"2.0","id":1,"method":"eth_getTransactionByHash",
                         "params":["0xc0e96fa11b3d298c3f37b7fc2fc43dc8ac11d9e0fab3f3d3816d8b2497757b6e"]}"#;
        let shown = of(asked, sample!("tx.res.json"));
        let lines = lines(&shown.sections[1]);
        for said in [
            ("type", "2 (EIP-1559)"),
            ("chainId", "1 (Ethereum)"),
            ("nonce", "204,580"),
            ("from", "0x5875Db54CD9AE2b2A875E09bB731772297aE9d92"),
            ("value", "0 ETH"),
            ("maxFeePerGas", "0.346815537 gwei"),
            ("selector", "0x09c5eabe"),
        ] {
            assert!(lines.contains(&said), "{said:?} in {lines:?}");
        }
    }

    #[test]
    fn the_logs_of_a_reply_say_which_event_each_one_is() {
        let asked = br#"{"jsonrpc":"2.0","id":1,"method":"eth_getTransactionReceipt",
                         "params":["0xc0e96fa11b3d298c3f37b7fc2fc43dc8ac11d9e0fab3f3d3816d8b2497757b6e"]}"#;
        let shown = of(asked, sample!("receipt.res.json"));
        let answer = &shown.sections[1];
        let lines = lines(answer);
        // The receipt is laid out now, rather than counted: see
        // `a_receipt_that_went_through_says_that_too`.
        assert_eq!(lines[0], ("status", "succeeded"));
        // Two real USDC Transfers, each out of its topics and its data.
        assert_eq!(
            lines
                .iter()
                .filter(|(label, _)| *label == "Transfer")
                .map(|(_, value)| *value)
                .collect::<Vec<_>>(),
            [
                "(0x000000000004444c5dc75cB358380D2e3dE08A90, 0x0000000aa232009084Bd71A5797d089AA4Edfad4, 856819229)",
                "(0x000000000004444c5dc75cB358380D2e3dE08A90, 0x0000000aa232009084Bd71A5797d089AA4Edfad4, 360010611832)",
            ]
        );
    }

    #[test]
    fn the_logs_a_filter_came_back_with_are_read_too() {
        let shown = of(sample!("get_logs.req.json"), sample!("get_logs.res.json"));
        let answer = &shown.sections[1];
        let lines = lines(answer);
        assert_eq!(lines[0], ("result", "3 items"));
        assert_eq!(
            lines
                .iter()
                .filter(|(label, _)| *label == "Transfer")
                .count(),
            3,
            "each one said: {lines:?}"
        );
    }

    #[test]
    fn a_raw_transaction_is_laid_out_by_its_fields() {
        let shown = of(sample!("send_raw.req.json"), b"");
        let asked = &shown.sections[0];
        assert_eq!(asked.title, "eth_sendRawTransaction");
        let lines = lines(asked);
        // By label, not by place: a field added in the middle should not
        // move what every other line of this says.
        for said in [
            ("type", "2 (EIP-1559)"),
            ("chainId", "1 (Ethereum)"),
            ("nonce", "204580"),
            // Who signed it, which the transaction does not carry.
            ("from", "0x5875Db54CD9AE2b2A875E09bB731772297aE9d92"),
            // EIP-55's capitals, which the store's own tests check against
            // addresses anyone can look up.
            ("to", "0x0000000aa232009084Bd71A5797d089AA4Edfad4"),
            ("value", "0 ETH"),
            ("gas", "462389"),
            ("maxFeePerGas", "0.346815537 gwei"),
            ("maxPriorityFeePerGas", "0 gwei"),
            // What it calls is nobody's standard function, so the four bytes
            // stand for it until an ABI names them.
            ("selector", "0x09c5eabe"),
        ] {
            assert!(lines.contains(&said), "{said:?} in {lines:?}");
        }
        // 772 bytes of calldata, four of them the selector.
        assert!(
            lines.contains(&(
                "arguments",
                "768 bytes, which the signature does not say how to read"
            )),
            "{lines:?}"
        );
    }

    #[test]
    fn a_reverted_call_says_why_it_reverted() {
        let shown = of(sample!("revert.req.json"), sample!("revert.res.json"));
        let asked = &shown.sections[0];
        // It sends a `from` as well, so the function is not the first line:
        // found by its label rather than counted to.
        assert_eq!(
            lines(asked)
                .into_iter()
                .find(|(label, _)| *label == "function"),
            Some(("function", "transfer(address,uint256)")),
            "named by the list every chain shares"
        );
        let failed = &shown.sections[1];
        assert_eq!(failed.title, "Failed");
        assert!(
            failed.note.as_deref().is_some_and(|n| n.contains("200")),
            "the status says nothing is wrong, so this has to"
        );
        assert_eq!(lines(failed)[0], ("code", "reverted (3)"));
        assert_eq!(
            lines(failed)[2],
            ("reverted", "ERC20: transfer amount exceeds balance")
        );
    }

    #[test]
    fn each_call_of_a_batch_is_paired_with_its_own_reply() {
        let shown = of(sample!("batch.req.json"), sample!("batch.res.json"));
        let titles: Vec<&str> = shown
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect();
        assert_eq!(
            titles,
            [
                "1 · eth_chainId",
                "Answer",
                "2 · eth_blockNumber",
                "Answer",
                "3 · net_version",
                "Answer",
            ]
        );
        // Matched by id, not by order: the second call's own answer.
        assert_eq!(shown.sections[3].lines[0].value, "0x192bbdc = 26393564");
    }

    #[test]
    fn a_filter_names_the_event_it_asks_for() {
        let shown = of(sample!("get_logs.req.json"), b"");
        assert_eq!(
            lines(&shown.sections[0]),
            [
                ("address", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
                ("fromBlock", "26,393,566"),
                ("toBlock", "26,393,566"),
                ("topic 0", "Transfer(address,address,uint256)"),
            ]
        );
    }

    #[test]
    fn a_method_of_its_own_still_shows_what_it_sent() {
        let call = br#"{"jsonrpc":"2.0","id":1,"method":"anvil_setBalance",
                        "params":["0xd8da6bf26964af9d7eed9e03e53415d37aa96045","0xde0b6b3a7640000"]}"#;
        let shown = of(call, b"");
        assert_eq!(
            lines(&shown.sections[0]),
            [
                ("param 0", "0xd8da6bf26964af9d7eed9e03e53415d37aa96045"),
                ("param 1", "0xde0b6b3a7640000 = 1000000000000000000"),
            ]
        );
    }

    /// A socket as a dapp uses one: subscribe, what came back, then what the
    /// server sends of its own until it is told to stop.
    #[test]
    fn a_sockets_calls_are_read_from_its_messages() {
        let messages: Vec<(bool, Vec<u8>)> = [
            (true, r#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["newHeads"]}"#),
            (false, r#"{"jsonrpc":"2.0","id":1,"result":"0xcd0c3e8af5"}"#),
            (false, r#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"subscription":"0xcd0c3e8af5","result":{"number":"0x192bbde","hash":"0x7dde"}}}"#),
            (true, r#"{"jsonrpc":"2.0","id":2,"method":"eth_unsubscribe","params":["0xcd0c3e8af5"]}"#),
            (false, r#"{"jsonrpc":"2.0","id":2,"result":true}"#),
        ]
        .into_iter()
        .map(|(from_client, text)| (from_client, text.as_bytes().to_vec()))
        .collect();
        let shown = shown_messages(&messages, &Abi::default());
        assert_eq!(
            titles(&shown),
            [
                "eth_subscribe",
                "Answer",
                "eth_subscription",
                "eth_unsubscribe",
                "Answer",
            ],
            "in the order they went, each reply after the call it answers"
        );
        assert_eq!(lines(&shown.sections[0]), [("kind", "newHeads")]);
        assert_eq!(shown.sections[1].lines[0].value, "0xcd0c3e8af5");
        // What the server sent without being asked for that one.
        let sent = &shown.sections[2];
        assert_eq!(
            sent.note.as_deref(),
            Some("Sent by the server, not asked for")
        );
        assert_eq!(
            lines(sent),
            [
                ("subscription", "0xcd0c3e8af5"),
                ("block", "26,393,566"),
                ("hash", "0x7dde"),
            ]
        );
        assert_eq!(
            lines(&shown.sections[3]),
            [("subscription", "0xcd0c3e8af5")]
        );
    }

    #[test]
    fn a_socket_that_says_something_else_has_nothing_to_show() {
        let messages = [(true, b"hello".to_vec()), (false, b"{}".to_vec())];
        assert!(
            shown_messages(&messages, &Abi::default())
                .sections
                .is_empty()
        );
    }

    /// A reply is matched by its id, not by what came next.
    #[test]
    fn a_reply_out_of_order_still_finds_its_call() {
        let messages: Vec<(bool, Vec<u8>)> = [
            (
                true,
                r#"{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber"}"#,
            ),
            (true, r#"{"jsonrpc":"2.0","id":2,"method":"eth_chainId"}"#),
            (false, r#"{"jsonrpc":"2.0","id":2,"result":"0x1"}"#),
            (false, r#"{"jsonrpc":"2.0","id":1,"result":"0x192bbde"}"#),
        ]
        .into_iter()
        .map(|(from_client, text)| (from_client, text.as_bytes().to_vec()))
        .collect();
        let shown = shown_messages(&messages, &Abi::default());
        assert_eq!(
            titles(&shown),
            ["eth_blockNumber", "Answer", "eth_chainId", "Answer"]
        );
        assert_eq!(
            shown.sections[1].lines[0].value, "0x192bbde = 26393566",
            "the block number went with the call that asked for it"
        );
        // `0x1 = 1` would be saying it twice, so the hex stands alone.
        assert_eq!(shown.sections[3].lines[0].value, "0x1");
    }

    #[test]
    fn an_abi_names_what_the_shared_list_does_not() {
        let dir = std::env::temp_dir().join(format!("tracewing-rpc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("vault.json"),
            r#"[{"type":"function","name":"redeem",
                 "inputs":[{"name":"shares","type":"uint256"},{"name":"to","type":"address"}]},
                {"type":"error","name":"Paused","inputs":[]}]"#,
        )
        .unwrap();
        let abi = Abi::load(std::slice::from_ref(&dir));
        let data = web3::hex_text(&{
            let mut call = web3::selector_of("redeem(uint256,address)").to_vec();
            call.extend(web3::bytes_of(&format!("0x{:064x}", 5_000_000u64)).unwrap());
            call.extend(
                web3::bytes_of(
                    "0x000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045",
                )
                .unwrap(),
            );
            call
        });
        let request = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{{"to":"0xdead","data":"{data}"}},"latest"]}}"#
        );
        let reply = format!(
            r#"{{"jsonrpc":"2.0","id":1,"error":{{"code":3,"message":"execution reverted","data":"{}"}}}}"#,
            web3::hex_text(&web3::selector_of("Paused()"))
        );
        let shown = shown(
            "POST",
            Some("application/json"),
            request.as_bytes(),
            reply.as_bytes(),
            &abi,
        );
        assert_eq!(
            lines(&shown.sections[0])[1..],
            [
                ("function", "redeem(uint256,address)"),
                ("uint256", "5000000"),
                ("address", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
                ("block", "latest"),
            ]
        );
        // A custom error is four bytes until an ABI says what they are.
        assert_eq!(
            lines(&shown.sections[1]).last(),
            Some(&("error", "Paused()"))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
