//! Which JSON-RPC method a request calls, to name it in the list: every call
//! goes to the same URL with the same method, so the path says nothing, as a
//! GraphQL path says nothing.
//!
//! Only the shape is read here, not what the call means: Ethereum's
//! `eth_call`, Solana's `getAccountInfo` and MCP's `tools/call` are all the
//! same three fields. What a chain's methods and their hex hold is for a
//! lens on top of this to say.

use serde_json::Value;

/// Past this a reply is not looked at for a failure: a failure is a line or
/// two of JSON, and anything larger is an answer that went through. Without
/// it every large reply of a session would be parsed a second time for
/// nothing, which is what [`faults`] is cheap enough to be called for.
pub const FAULT_MAX: usize = 64 << 10;

/// Past this a method name is cut: a name is a word, and a longer one came
/// from something that is not a client.
const NAME_MAX: usize = 80;

/// `eth_call`, or `eth_chainId +2` for a batch that holds three calls.
/// `None` when the request is not JSON-RPC.
pub fn method(http_method: &str, content_type: Option<&str>, body: &[u8]) -> Option<String> {
    if !http_method.eq_ignore_ascii_case("POST") {
        return None;
    }
    if !is_json(content_type?) || !holds(body, b"\"method\"") {
        return None;
    }
    match serde_json::from_slice::<Value>(body).ok()? {
        Value::Array(batch) => {
            let mut names = batch.iter().filter_map(name_of);
            let first = names.next()?;
            Some(match names.count() {
                0 => first,
                rest => format!("{first} +{rest}"),
            })
        }
        value => name_of(&value),
    }
}

/// One call a request sends.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// What the reply will be matched by. A notification has none, and does
    /// not get a reply at all.
    pub id: Option<Value>,
    pub method: String,
    /// By name or by place, as the client chose to send them.
    pub params: Option<Value>,
}

/// What a request calls, each call of a batch on its own. Empty when the
/// request is not JSON-RPC.
pub fn calls(http_method: &str, content_type: Option<&str>, body: &[u8]) -> Vec<Call> {
    if !http_method.eq_ignore_ascii_case("POST") || !content_type.is_some_and(is_json) {
        return Vec::new();
    }
    calls_in(body)
}

/// The calls a message holds, where there is no HTTP around it to say what it
/// is: a WebSocket carries JSON-RPC with no content type and no method, and a
/// call or a notification a message.
pub fn calls_in(body: &[u8]) -> Vec<Call> {
    if !holds(body, b"\"method\"") {
        return Vec::new();
    }
    let read = |value: &Value| {
        Some(Call {
            id: value.get("id").filter(|v| !v.is_null()).cloned(),
            method: name_of(value)?,
            params: value.get("params").filter(|v| !v.is_null()).cloned(),
        })
    };
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Array(batch)) => batch.iter().filter_map(read).collect(),
        Ok(value) => read(&value).into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

/// A call that failed. JSON-RPC answers 200 and puts the failure in the
/// body, so from the outside a failed call looks like a good one: this is
/// what tells them apart.
#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    /// `-32601` for a method a server does not have, `3` for a reverted
    /// Ethereum call, and a server's own numbers below `-32000`.
    pub code: i64,
    pub message: String,
    /// What the server sent with it, which for a revert holds the reason.
    pub data: Option<Value>,
}

/// The failures a reply carries, one per failed call of a batch. Empty when
/// every call went through, when the body is not a JSON-RPC reply, and when
/// it is longer than [`FAULT_MAX`]. Worth calling only for a flow whose
/// request [`method`] named, so that other JSON is not parsed twice.
pub fn faults(content_type: Option<&str>, body: &[u8]) -> Vec<Fault> {
    if !content_type.is_some_and(is_json) {
        return Vec::new();
    }
    faults_in(body)
}

/// The failures a message carries, where there is no HTTP around it.
pub fn faults_in(body: &[u8]) -> Vec<Fault> {
    if body.len() > FAULT_MAX || !holds(body, b"\"error\"") {
        return Vec::new();
    }
    let read = |value: &Value| {
        let error = value.get("error")?.as_object()?;
        Some(Fault {
            code: error.get("code").and_then(Value::as_i64)?,
            message: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            data: error.get("data").filter(|v| !v.is_null()).cloned(),
        })
    };
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Array(batch)) => batch.iter().filter_map(read).collect(),
        Ok(value) => read(&value).into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

/// The method one call names, if the object is a call at all. A `jsonrpc` of
/// its own is the sure sign; Bitcoin Core and the older clients leave it out
/// or send `1.0`, and then an `id` to match the reply by has to do.
fn name_of(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    let name = object.get("method")?.as_str()?;
    if name.is_empty() {
        return None;
    }
    let spoken = object.get("jsonrpc").is_some_and(Value::is_string);
    if !spoken && !object.contains_key("id") {
        return None;
    }
    Some(match name.char_indices().nth(NAME_MAX) {
        Some((end, _)) => format!("{}…", &name[..end]),
        None => name.to_owned(),
    })
}

/// Whether the body holds `key` at all, which a scan finds far more cheaply
/// than parsing it: a call always names a `"method"`, and a failed reply an
/// `"error"`, so a body without those bytes cannot be either. Every other
/// JSON body of a session stops here instead of being parsed a second time
/// after [`crate::graphql`] has already parsed it.
///
/// A key written with an escape, as `"meth\u006fd"`, is legal JSON and is
/// not found: no client writes one.
fn holds(body: &[u8], key: &[u8]) -> bool {
    body.len() >= key.len() && body.windows(key.len()).any(|window| window == key)
}

/// `application/json`, and the `application/json-rpc` some servers ask for.
fn is_json(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .ends_with("json")
}

/// What a code is called: the five JSON-RPC answers with, the ones every
/// Ethereum node adds (EIP-1474), and `3`, which is a reverted call. `None`
/// for a server's own number, which only its message explains.
pub fn code_name(code: i64) -> Option<&'static str> {
    Some(match code {
        -32700 => "parse error",
        -32600 => "invalid request",
        -32601 => "method not found",
        -32602 => "invalid params",
        -32603 => "internal error",
        -32000 => "server error",
        -32001 => "resource not found",
        -32002 => "resource unavailable",
        -32003 => "transaction rejected",
        -32004 => "method not supported",
        -32005 => "limit exceeded",
        3 => "reverted",
        _ => return None,
    })
}

/// `method not found (-32601)`, or the number alone for one with no name.
pub fn code_text(code: i64) -> String {
    match code_name(code) {
        Some(name) => format!("{name} ({code})"),
        None => code.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! sample {
        ($name:literal) => {
            include_bytes!(concat!("../testdata/jsonrpc/", $name))
        };
    }

    fn post(body: &[u8]) -> Option<String> {
        method("POST", Some("application/json"), body)
    }

    #[test]
    fn a_call_is_named_by_its_method() {
        assert_eq!(
            post(sample!("call_balanceof.req.json")).as_deref(),
            Some("eth_call")
        );
        assert_eq!(
            post(sample!("send_raw.req.json")).as_deref(),
            Some("eth_sendRawTransaction")
        );
        assert_eq!(
            post(sample!("get_logs.req.json")).as_deref(),
            Some("eth_getLogs")
        );
    }

    #[test]
    fn a_batch_is_named_by_its_first_call_and_how_many_follow() {
        assert_eq!(
            post(sample!("batch.req.json")).as_deref(),
            Some("eth_chainId +2")
        );
        assert_eq!(
            post(br#"[{"jsonrpc":"2.0","id":1,"method":"a"}]"#).as_deref(),
            Some("a")
        );
        assert_eq!(post(b"[]"), None);
    }

    #[test]
    fn every_json_rpc_is_read_the_same_way_whatever_it_speaks_to() {
        // MCP over HTTP is JSON-RPC too, and gets the same column.
        assert_eq!(post(sample!("mcp.req.json")).as_deref(), Some("tools/call"));
        // Solana, and Bitcoin Core, which sends 1.0 or nothing at all.
        let solana = br#"{"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":["x"]}"#;
        assert_eq!(post(solana).as_deref(), Some("getAccountInfo"));
        let bitcoin = br#"{"jsonrpc":"1.0","id":"c","method":"getblockhash","params":[1]}"#;
        assert_eq!(post(bitcoin).as_deref(), Some("getblockhash"));
        let old = br#"{"id":1,"method":"getinfo","params":[]}"#;
        assert_eq!(post(old).as_deref(), Some("getinfo"));
    }

    #[test]
    fn what_a_request_calls_is_read_whole() {
        let one = super::calls(
            "POST",
            Some("application/json"),
            sample!("call_balanceof.req.json"),
        );
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].method, "eth_call");
        assert_eq!(one[0].id, Some(serde_json::json!(5)));
        let params = one[0].params.as_ref().unwrap();
        assert_eq!(params[1], serde_json::json!("latest"));
        assert_eq!(
            params[0]["data"].as_str().unwrap(),
            "0x70a08231000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045"
        );
        let batch = super::calls("POST", Some("application/json"), sample!("batch.req.json"));
        assert_eq!(
            batch.iter().map(|c| c.method.as_str()).collect::<Vec<_>>(),
            ["eth_chainId", "eth_blockNumber", "net_version"]
        );
        assert_eq!(batch[0].params, None, "a call may send none");
    }

    #[test]
    fn a_notification_has_no_id_but_is_still_a_call() {
        let body = br#"{"jsonrpc":"2.0","method":"eth_subscription","params":{"result":{}}}"#;
        assert_eq!(post(body).as_deref(), Some("eth_subscription"));
        let one = super::calls("POST", Some("application/json"), body);
        assert_eq!(one[0].id, None);
    }

    #[test]
    fn a_failed_call_answers_200_and_says_so_in_the_body() {
        let json = Some("application/json");
        let revert = faults(json, sample!("revert.res.json"));
        assert_eq!(revert.len(), 1);
        assert_eq!(revert[0].code, 3);
        assert_eq!(
            revert[0].message,
            "execution reverted: ERC20: transfer amount exceeds balance"
        );
        assert!(
            revert[0]
                .data
                .as_ref()
                .and_then(Value::as_str)
                .is_some_and(|d| d.starts_with("0x08c379a0")),
            "the reason is in the data, as Error(string)"
        );
        let missing = faults(json, sample!("error_no_method.res.json"));
        assert_eq!(missing[0].code, -32601);
        assert!(
            faults(json, sample!("batch.res.json")).is_empty(),
            "every call went through"
        );
        assert!(faults(json, sample!("call_balanceof.res.json")).is_empty());
    }

    #[test]
    fn one_failed_call_of_a_batch_is_found_among_the_good_ones() {
        let body = br#"[{"jsonrpc":"2.0","id":1,"result":"0x1"},
                        {"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"bad params"}}]"#;
        let found = super::faults(Some("application/json"), body);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, -32602);
        assert_eq!(found[0].data, None);
    }

    #[test]
    fn a_message_with_no_http_around_it_is_read_the_same_way() {
        // A WebSocket sends a call a message, with nothing to say what it is:
        // a subscription asked for, then what it sends back.
        let asked = br#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["newHeads"]}"#;
        let asking = calls_in(asked);
        assert_eq!(asking[0].method, "eth_subscribe");
        assert_eq!(asking[0].id, Some(serde_json::json!(1)));
        let sent = br#"{"jsonrpc":"2.0","method":"eth_subscription",
                        "params":{"subscription":"0xcd0c3e8af5","result":{}}}"#;
        assert_eq!(calls_in(sent)[0].method, "eth_subscription");
        assert!(calls_in(br#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#).is_empty());
        let failed = br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"no"}}"#;
        assert_eq!(faults_in(failed)[0].code, -32601);
        // The ones that take HTTP still want it.
        assert!(calls("POST", None, asked).is_empty());
        assert!(faults(None, failed).is_empty());
    }

    #[test]
    fn a_body_without_the_words_is_not_parsed_at_all() {
        // What the scan rules out, parsing would rule out too: the answer is
        // the same, and the cost is not.
        assert!(!holds(br#"{"a":1}"#, b"\"method\""));
        assert!(holds(br#"{ "method" : "a" }"#, b"\"method\""));
        assert!(!holds(b"", b"\"method\""));
        assert_eq!(post(br#"{"jsonrpc":"2.0","id":1,"params":[]}"#), None);
        assert!(
            faults(
                Some("application/json"),
                br#"{"jsonrpc":"2.0","result":"0x1"}"#
            )
            .is_empty()
        );
    }

    #[test]
    fn a_reply_too_large_to_be_a_failure_is_not_parsed() {
        let padding = "x".repeat(FAULT_MAX);
        let body =
            format!(r#"{{"jsonrpc":"2.0","id":1,"error":{{"code":-1,"message":"{padding}"}}}}"#);
        assert!(
            faults(Some("application/json"), body.as_bytes()).is_empty(),
            "a failure is short; a long body is an answer, and is left alone"
        );
    }

    #[test]
    fn other_json_is_left_alone() {
        assert_eq!(
            post(br#"{"method":"POST","url":"/x"}"#),
            None,
            "no id, no jsonrpc"
        );
        assert_eq!(post(br#"{"jsonrpc":"2.0","id":1,"method":""}"#), None);
        assert_eq!(post(br#"{"jsonrpc":"2.0","id":1,"method":42}"#), None);
        assert_eq!(
            post(br#"{"jsonrpc":"2.0","id":1,"result":"0x1"}"#),
            None,
            "a reply"
        );
        assert_eq!(post(b"not json"), None);
        assert_eq!(
            method(
                "POST",
                Some("text/plain"),
                br#"{"jsonrpc":"2.0","id":1,"method":"a"}"#
            ),
            None
        );
        assert_eq!(
            method(
                "GET",
                Some("application/json"),
                br#"{"jsonrpc":"2.0","id":1,"method":"a"}"#
            ),
            None
        );
        assert_eq!(
            method("POST", None, br#"{"jsonrpc":"2.0","id":1,"method":"a"}"#),
            None
        );
    }

    #[test]
    fn codes_have_names_where_a_name_says_more_than_the_number() {
        assert_eq!(code_text(-32601), "method not found (-32601)");
        assert_eq!(code_text(3), "reverted (3)");
        assert_eq!(code_text(-32099), "-32099", "a server's own number");
    }

    #[test]
    fn a_method_name_long_enough_to_fill_a_column_is_cut() {
        let name = "e".repeat(200);
        let body = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{name}"}}"#);
        let got = post(body.as_bytes()).unwrap();
        assert_eq!(
            got.chars().count(),
            NAME_MAX + 1,
            "cut, with the sign of it"
        );
        assert!(got.ends_with('…'));
    }
}
