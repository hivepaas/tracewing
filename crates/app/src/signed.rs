//! A body that carries a signed transaction, and whether what is written now
//! still carries the one that was signed.
//!
//! Signing is not tracewing's: it holds no keys and makes no signature. The
//! most it can do is notice that an edit has left a signature standing over
//! bytes that are gone, and say so before the body goes to a node that will
//! only answer with a puzzle.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use tracewing_i18n::tr;
use tracewing_store::{bitcoin, solana, web3};

/// What a body carries, as far as a signature goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signed {
    /// An Ethereum transaction, and the address its signature works out to.
    /// The sender is not in a transaction: it is recovered from the
    /// signature over the rest of it, so it changes as soon as any of that
    /// does. This is the one chain here where an edit can be caught for
    /// certain rather than guessed at.
    Ethereum(Option<String>),
    /// A Solana transaction: this many signatures over the message after
    /// them. Nothing here can check them.
    Solana(usize),
    /// A Bitcoin transaction, by the id its own bytes give it.
    Bitcoin(String),
}

/// What the JSON-RPC in `body` carries, where that is a signed transaction
/// this can read. `None` for everything else, a body half typed included.
pub fn signed_in(body: &[u8]) -> Option<Signed> {
    let json: Value = serde_json::from_slice(body).ok()?;
    let each = match json.as_array() {
        Some(batch) => batch.iter().collect(),
        None => vec![&json],
    };
    // The first one that carries a signature: a batch of them is rare, and
    // one is enough to say what an edit would cost.
    each.into_iter().find_map(carried)
}

fn carried(message: &Value) -> Option<Signed> {
    let method = message.get("method").and_then(Value::as_str)?;
    let first = message.get("params").and_then(|params| params.get(0));
    let text = first.and_then(Value::as_str);
    match method {
        "eth_sendRawTransaction" => {
            let bytes = web3::bytes_of(text?)?;
            let tx = web3::Transaction::decode(&bytes)?;
            Some(Signed::Ethereum(tx.from))
        }
        "sendTransaction" => {
            // Base64 as a node takes it now, base58 as the older clients
            // sent it, the way the inspector reads both.
            let raw = text?;
            let bytes = match STANDARD.decode(raw) {
                Ok(bytes) => bytes,
                Err(_) => solana::from_base58(raw)?,
            };
            let tx = solana::Transaction::decode(&bytes)?;
            Some(Signed::Solana(tx.signatures.len()))
        }
        "sendrawtransaction" => {
            let raw =
                text.or_else(|| first.and_then(|list| list.get(0)).and_then(Value::as_str))?;
            let bytes = bitcoin::bytes_of(raw)?;
            let tx = bitcoin::Transaction::decode(&bytes)?;
            Some(Signed::Bitcoin(tx.txid))
        }
        _ => None,
    }
}

/// What to say about a body that carried `was` and now carries `now`, where
/// the two are not the same transaction any more. `None` while nothing is
/// the matter: the signature still works out to whom it did, or what is
/// written now is another signed transaction, pasted whole.
pub fn trouble(was: &Signed, now: Option<&Signed>) -> Option<String> {
    match (was, now) {
        // Recovered the same address: the signature still stands over these
        // bytes, whatever else in the body was changed.
        (Signed::Ethereum(was), Some(Signed::Ethereum(now))) if was == now => None,
        (Signed::Ethereum(was), Some(Signed::Ethereum(Some(now)))) => Some(tr!(
            "The signature in this body now works out to {now}, not {was}: it is no longer the transaction that was signed. tracewing holds no keys, so it cannot sign the new one.",
            now = now.clone(),
            was = was.clone().unwrap_or_default()
        )),
        // Not a signed transaction any more, or not one this can read.
        (Signed::Ethereum(_), _) => Some(tr!(
            "What this body carries no longer reads as a signed transaction. tracewing holds no keys, so it cannot sign it again."
        )),
        // Solana and Bitcoin keep their signatures apart from what was
        // signed, and nothing here can check them: that the bytes changed at
        // all is as much as can honestly be said.
        (Signed::Solana(_) | Signed::Bitcoin(_), _) => Some(tr!(
            "This body carries a signed transaction and its bytes have changed. The signatures are over the bytes as they were, and tracewing can neither check them nor make new ones."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The samples the inspector's tests read, captured from real nodes:
    /// `crates/store/testdata/jsonrpc/README.md` and the ones beside it.
    macro_rules! sample {
        ($dir:literal, $name:literal) => {
            include_str!(concat!("../../store/testdata/", $dir, "/", $name))
        };
    }

    /// The hex of the one raw transaction a body carries.
    fn raw_of(body: &str) -> String {
        let json: Value = serde_json::from_str(body).unwrap();
        json["params"][0].as_str().unwrap().to_owned()
    }

    #[test]
    fn a_body_says_what_it_carries() {
        let body = sample!("jsonrpc", "send_raw.req.json");
        let Some(Signed::Ethereum(Some(from))) = signed_in(body.as_bytes()) else {
            panic!("{:?}", signed_in(body.as_bytes()));
        };
        // The same sender the inspector shows, worked out from the signature.
        let bytes = web3::bytes_of(&raw_of(body)).unwrap();
        assert_eq!(
            Some(from),
            web3::Transaction::decode(&bytes).unwrap().from,
            "recovered, not read off"
        );

        let solana = sample!("solana", "send_transaction.req.json");
        assert!(matches!(
            signed_in(solana.as_bytes()),
            Some(Signed::Solana(1..))
        ));

        let bitcoin = sample!("bitcoin", "segwit.req.json");
        let Some(Signed::Bitcoin(txid)) = signed_in(bitcoin.as_bytes()) else {
            panic!("{:?}", signed_in(bitcoin.as_bytes()));
        };
        assert_eq!(txid.len(), 64, "a txid as hex: {txid}");

        // Bodies that carry no signature at all.
        for body in [
            sample!("jsonrpc", "call_balanceof.req.json"),
            sample!("jsonrpc", "get_logs.req.json"),
            r#"{"jsonrpc":"2.0","id":1,"method":"eth_sendRawTransaction","params":["0xdeadbeef"]}"#,
            "not json",
            r#"{"jsonrpc":"2.0","id":1,"metho"#,
        ] {
            assert_eq!(signed_in(body.as_bytes()), None, "{body}");
        }
    }

    #[test]
    fn an_edit_that_leaves_the_signature_behind_is_caught() {
        let body = sample!("jsonrpc", "send_raw.req.json");
        let was = signed_in(body.as_bytes()).expect("a signed transaction");

        // The same body again: nothing is the matter.
        assert_eq!(trouble(&was, signed_in(body.as_bytes()).as_ref()), None);

        // A byte of the transaction changed, which is what editing one in
        // the window does. The signature is over what it was, so it works
        // out to someone else: that is how it is caught.
        let raw = raw_of(body);
        let mut bytes = web3::bytes_of(&raw).unwrap();
        let last = bytes.len() - 40;
        bytes[last] ^= 0x01;
        let edited = body.replace(&raw, &web3::hex_text(&bytes));
        let now = signed_in(edited.as_bytes());
        assert_ne!(now, Some(was.clone()), "another sender, or none");
        let said = trouble(&was, now.as_ref()).expect("it says so");
        match now {
            Some(Signed::Ethereum(Some(whom))) => {
                assert!(said.contains(&whom), "who it reads as now: {said}")
            }
            _ => assert!(said.contains("no longer reads"), "{said}"),
        }

        // Half typed, so there is nothing to read: still worth saying.
        assert!(trouble(&was, None).is_some());

        // What Solana and Bitcoin can say: the bytes changed, no more.
        let said = trouble(&Signed::Solana(1), None).expect("it says so");
        assert!(said.contains("neither check them nor make"), "{said}");
        assert_eq!(
            trouble(&Signed::Bitcoin("ab".repeat(32)), None),
            Some(said),
            "the same words: neither chain can be checked here"
        );
    }
}
