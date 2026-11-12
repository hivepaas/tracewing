//! A call written out where calldata goes: `balanceOf(0xd8dA…)` in the
//! `data` of a JSON-RPC body, sent as the bytes it stands for. The same
//! writing works in a request composed in the window and in a message typed
//! into an open socket.

use std::sync::Arc;
use tracewing_i18n::tr;
use tracewing_store::{abi::Abi, web3};

/// The JSON-RPC methods whose first parameter carries calldata, where a call
/// written out in the body stands for the bytes to send.
const CARRY_CALLDATA: [&str; 4] = [
    "eth_call",
    "eth_estimateGas",
    "eth_sendTransaction",
    "eth_createAccessList",
];

/// Every call written out in the `data` or `input` of a JSON-RPC body, in
/// the order they are written. Hex, as a node is sent, is not one of them.
pub(crate) fn written_calls(body: &serde_json::Value) -> Vec<String> {
    let each = match body.as_array() {
        Some(batch) => batch.iter().collect(),
        None => vec![body],
    };
    let mut found = Vec::new();
    for message in each {
        let method = message.get("method").and_then(serde_json::Value::as_str);
        if !method.is_some_and(|method| CARRY_CALLDATA.contains(&method)) {
            continue;
        }
        let Some(first) = message.get("params").and_then(|params| params.get(0)) else {
            continue;
        };
        for key in ["data", "input"] {
            match first.get(key).and_then(serde_json::Value::as_str) {
                Some(text) if web3::written(text).is_some() => found.push(text.to_owned()),
                _ => {}
            }
        }
    }
    found
}

/// The body to send, where a call is written out in it rather than sent as
/// hex: `balanceOf(0xd8dA…)` becomes the calldata it stands for, and the
/// signatures it was taken to mean come back with it to be shown.
///
/// Only those strings are replaced, so the rest of the body goes exactly as
/// it was written.
pub fn as_calldata(
    text: &str,
    signatures: &dyn Fn(&str) -> Vec<String>,
) -> Result<(String, Vec<String>), String> {
    let Ok(body) = serde_json::from_str::<serde_json::Value>(text) else {
        return Ok((text.to_owned(), Vec::new()));
    };
    let written = written_calls(&body);
    if written.is_empty() {
        return Ok((text.to_owned(), Vec::new()));
    }
    let mut out = text.to_owned();
    let mut signed = Vec::new();
    for call in written {
        let encoded = web3::calldata(&call, signatures).map_err(trouble_text)?;
        let was = serde_json::Value::String(call).to_string();
        let now = serde_json::Value::String(web3::hex_text(&encoded.data)).to_string();
        out = out.replacen(&was, &now, 1);
        signed.push(encoded.signature);
    }
    // Nothing is sent as a call written out: a body this could not rewrite,
    // because it writes its strings another way, would reach the node as
    // text where it wants hex.
    let left = serde_json::from_str::<serde_json::Value>(&out)
        .map(|body| written_calls(&body))
        .unwrap_or_default();
    if let Some(call) = left.first() {
        return Err(tr!(
            "{call} could not be put in the body as calldata. Write it as hex, or let tracewing write the body.",
            call = call.clone()
        ));
    }
    Ok((out, signed))
}

/// Why a written call could not be turned into calldata, in words.
fn trouble_text(trouble: web3::Trouble) -> String {
    match trouble {
        web3::Trouble::NotACall => tr!("That is not a call written out."),
        web3::Trouble::Unknown(name) => tr!(
            "No call named {name} is known here. Write its types with it, as transfer(address,uint256)(…), or add its ABI under Contract ABIs in Settings.",
            name = name
        ),
        web3::Trouble::Overloaded(found) => tr!(
            "{name} is more than one call: {found}. Write the types with it, as {first}(…).",
            name = web3::name_of(found.first().map(String::as_str).unwrap_or_default()).to_owned(),
            found = found.join(", "),
            first = found.first().cloned().unwrap_or_default()
        ),
        web3::Trouble::Types(types) => tr!(
            "{types} are not types a call can be written with.",
            types = types
        ),
        web3::Trouble::Values(signature, why) => tr!(
            "The values do not fit {signature}: {why}",
            signature = signature,
            why = why
        ),
    }
}

/// What an ABI the user chose, and the calls every chain shares, know by
/// that name — for [`as_calldata`] to look a written call up with.
pub fn signatures_of(abi: &Arc<Abi>) -> impl Fn(&str) -> Vec<String> + '_ {
    move |name: &str| {
        let mut found = abi.signatures(name);
        found.extend(web3::known_signatures(name));
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The samples the inspector's tests read, which were captured from
    /// real nodes: `crates/store/testdata/jsonrpc/README.md`.
    macro_rules! sample {
        ($name:literal) => {
            include_str!(concat!("../../store/testdata/jsonrpc/", $name))
        };
    }

    /// Only the calls every chain shares: no ABI of the user's here.
    fn known(name: &str) -> Vec<String> {
        web3::known_signatures(name)
    }

    #[test]
    fn a_call_written_in_the_body_is_sent_as_the_calldata_it_stands_for() {
        let sent = sample!("call_balanceof.req.json");
        let json: serde_json::Value = serde_json::from_str(sent).unwrap();
        let hex = json["params"][0]["data"].as_str().unwrap();
        // Written out as the inspector shows it, in place of the hex.
        let data = web3::bytes_of(hex).unwrap();
        let shown = web3::arguments("balanceOf(address)", &data).unwrap();
        let written = sent.replace(hex, &format!("balanceOf({})", shown[0].text));
        assert!(written.contains("balanceOf(0x"), "{written}");

        let (body, calls) = as_calldata(&written, &known).unwrap();
        assert_eq!(body, sent, "the body goes as the node had it, hex and all");
        assert_eq!(calls, ["balanceOf(address)"]);
    }

    #[test]
    fn a_body_with_no_call_written_in_it_goes_as_it_is() {
        // Hex, as it is sent.
        let sent = sample!("call_balanceof.req.json");
        assert_eq!(
            as_calldata(sent, &known).unwrap(),
            (sent.to_owned(), vec![])
        );

        // Not JSON at all.
        let text = "balanceOf(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045)";
        assert_eq!(
            as_calldata(text, &known).unwrap(),
            (text.to_owned(), vec![]),
            "the body is the body: only JSON-RPC calldata is written out"
        );

        // A method that carries no calldata, whatever its parameters hold.
        let other = r#"{"jsonrpc":"2.0","id":1,"method":"eth_getBalance","params":[{"data":"balanceOf(0x00)"}]}"#;
        assert_eq!(
            as_calldata(other, &known).unwrap(),
            (other.to_owned(), vec![])
        );
    }

    #[test]
    fn every_call_in_a_batch_is_written_out() {
        let one = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let body = format!(
            r#"[{{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{{"to":"{one}","data":"balanceOf({one})"}},"latest"]}},
               {{"jsonrpc":"2.0","id":2,"method":"eth_sendTransaction","params":[{{"to":"{one}","input":"transfer({one}, 1000000)"}}]}}]"#
        );
        let (sent, calls) = as_calldata(&body, &known).unwrap();
        assert_eq!(
            calls,
            ["balanceOf(address)", "transfer(address,uint256)"],
            "both, and `input` as well as `data`"
        );
        // Each is the hex that web3 makes of it, and nothing else moved.
        let before: serde_json::Value = serde_json::from_str(&body).unwrap();
        let json: serde_json::Value = serde_json::from_str(&sent).unwrap();
        for (at, key) in [(0usize, "data"), (1usize, "input")] {
            let written = before[at]["params"][0][key].as_str().unwrap();
            let hex = web3::hex_text(&web3::calldata(written, &known).unwrap().data);
            assert_eq!(json[at]["params"][0][key].as_str().unwrap(), hex);
        }
        assert_eq!(json[0]["params"][1], "latest");
        assert_eq!(json[1]["id"], 2);
    }

    #[test]
    fn a_call_that_cannot_be_written_stops_the_send_and_says_why() {
        let body =
            r#"{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{"data":"frobnicate(1)"}]}"#;
        let problem = as_calldata(body, &known).unwrap_err();
        assert!(problem.contains("frobnicate"), "{problem}");

        let overloaded = r#"{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{"data":"safeTransferFrom(0x00, 0x00, 1)"}]}"#;
        let problem = as_calldata(overloaded, &known).unwrap_err();
        assert!(
            problem.contains("safeTransferFrom(address,address,uint256)"),
            "{problem}"
        );
    }
}
