//! A node that answers like an Ethereum one, for the window to read.
//!
//! What it says is real where it can be: the calls and answers captured from
//! mainnet in `crates/store/testdata/jsonrpc` (chain 1, block `0x192bbde`)
//! are served back for the calls they were captured for, and the rest is
//! written in the same shape. `cases()` is what the playground's `/web3`
//! page offers, one button each, so that a window has more than one kind of
//! web3 traffic in it.

use std::{fmt::Write as _, sync::OnceLock};
use tracewing_store::{jsonrpc, web3};

/// Traffic captured from mainnet, kept with the store's samples so that the
/// window is tried against what a node really said.
macro_rules! captured {
    ($name:literal) => {
        include_str!(concat!("../../store/testdata/jsonrpc/", $name))
    };
}

/// The transaction the samples follow, through its send, its place in the
/// chain and its receipt.
const SENT: &str = "0xc0e96fa11b3d298c3f37b7fc2fc43dc8ac11d9e0fab3f3d3816d8b2497757b6e";
/// One that was mined and did not go through.
const FAILED: &str = "0x6dd4134ad889bbb0fd2589146eac5c6b73a1f8c5f23f47924f09fc9a2cca1708";
/// The kind from before EIP-2718, which carries its chain in `v`.
const LEGACY: &str = "0x2df8175a692bdb2a19a77b3e09906844970cd1983d45f0cc72ac74f3c525e9bb";
/// USDC, whose balances and transfers the samples are of.
const TOKEN: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
/// An address with something in it, as the samples have it.
const WHO: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
/// The block the samples were taken at.
const BLOCK: &str = "0x192bbde";

/// One thing to send, as the `/web3` page offers it.
pub struct Case {
    /// How the page names it in its HTML, and the tests by hand.
    pub id: &'static str,
    /// The heading it goes under.
    pub group: &'static str,
    /// What the button says.
    pub name: &'static str,
    /// The line beside it: what there is to see in the window.
    pub what: &'static str,
    /// Where it is sent. `/rpc` for nearly all of them.
    pub url: String,
    /// What is sent, as it goes on the wire.
    pub body: String,
}

/// Everything the page offers, built once.
pub fn cases() -> &'static [Case] {
    static CASES: OnceLock<Vec<Case>> = OnceLock::new();
    CASES.get_or_init(build)
}

/// The case of that name, for a test to send what the page sends.
pub fn case(id: &str) -> Option<&'static Case> {
    cases().iter().find(|case| case.id == id)
}

fn build() -> Vec<Case> {
    let mut at = 100;
    let mut call = |group, name, what, method: &str, params: String| {
        at += 1;
        Case {
            id: "",
            group,
            name,
            what,
            url: "/rpc".to_owned(),
            body: format!(r#"{{"jsonrpc":"2.0","id":{at},"method":"{method}","params":{params}}}"#),
        }
    };
    let named = |id, case: Case| Case { id, ..case };
    let sample = |id, group, name, what, body: &str| Case {
        id,
        group,
        name,
        what,
        url: "/rpc".to_owned(),
        body: body.trim().to_owned(),
    };
    vec![
        // Calls: what a wallet or a page asks a contract, and what comes
        // back. The answers are worth reading only once they are decoded.
        sample(
            "balance-of",
            "Calls",
            "balanceOf",
            "ERC-20 balance in USDC: a word of hex that reads as 39.192124",
            captured!("call_balanceof.req.json"),
        ),
        named(
            "symbol",
            call(
                "Calls",
                "symbol()",
                "A string comes back, not a number: offset, length, bytes",
                "eth_call",
                format!(
                    r#"[{{"to":"{TOKEN}","data":"{}"}},"latest"]"#,
                    data("symbol()", &[])
                ),
            ),
        ),
        sample(
            "multicall",
            "Calls",
            "aggregate3",
            "Multicall3: tuples inside an array inside a call, 4.7 KB of it",
            captured!("multicall.req.json"),
        ),
        named(
            "estimate-gas",
            call(
                "Calls",
                "eth_estimateGas",
                "What a transfer would cost, before sending it",
                "eth_estimateGas",
                format!(
                    r#"[{{"from":"{WHO}","to":"{TOKEN}","data":"{}"}}]"#,
                    data(
                        "transfer(address,uint256)",
                        &[address_word(WHO), number_word(1_000_000)]
                    )
                ),
            ),
        ),
        // Reverts: a call that did not go through still comes back 200, and
        // what it reverted with is in the error's data.
        sample(
            "revert-reason",
            "Reverts",
            "Error(string)",
            "HTTP 200, code 3: ERC20: transfer amount exceeds balance",
            captured!("revert.req.json"),
        ),
        named(
            "revert-panic",
            call(
                "Reverts",
                "Panic(uint256)",
                "What the compiler raises: a sum that overflowed",
                "eth_call",
                format!(
                    r#"[{{"from":"{WHO}","to":"{TOKEN}","data":"{}"}},"latest"]"#,
                    data("withdraw(uint256)", &[max_word()])
                ),
            ),
        ),
        named(
            "revert-custom",
            call(
                "Reverts",
                "A custom error",
                "Four bytes with no name, until an ABI gives it one",
                "eth_call",
                format!(
                    r#"[{{"from":"{WHO}","to":"{TOKEN}","data":"{}"}},"latest"]"#,
                    data(
                        "approve(address,uint256)",
                        &[address_word(TOKEN), number_word(0)]
                    )
                ),
            ),
        ),
        // Transactions: signed, unsigned, and as the chain gives them back.
        sample(
            "send-raw",
            "Transactions",
            "sendRawTransaction (EIP-1559)",
            "884 bytes of signed transaction: its sender is worked out from it",
            captured!("send_raw.req.json"),
        ),
        sample(
            "send-raw-legacy",
            "Transactions",
            "sendRawTransaction (legacy)",
            "The kind from before EIP-2718, with the chain inside its v",
            captured!("send_raw_legacy.req.json"),
        ),
        sample(
            "send-transaction",
            "Transactions",
            "eth_sendTransaction",
            "A dev node signs it: from is the account named, not one recovered",
            captured!("send_transaction.req.json"),
        ),
        named(
            "tx-by-hash",
            call(
                "Transactions",
                "getTransactionByHash",
                "The same transaction as the node holds it, fees and all",
                "eth_getTransactionByHash",
                format!(r#"["{SENT}"]"#),
            ),
        ),
        named(
            "tx-by-hash-legacy",
            call(
                "Transactions",
                "getTransactionByHash (legacy)",
                "The older kind: gasPrice, and no fee cap",
                "eth_getTransactionByHash",
                format!(r#"["{LEGACY}"]"#),
            ),
        ),
        named(
            "tx-count",
            call(
                "Transactions",
                "getTransactionCount",
                "The nonce an account is up to",
                "eth_getTransactionCount",
                format!(r#"["{WHO}","pending"]"#),
            ),
        ),
        // What a transaction did: its logs, read as the events they are.
        sample(
            "get-logs",
            "Logs and receipts",
            "eth_getLogs",
            "Three Transfer logs, each with its topics and its amount",
            captured!("get_logs.req.json"),
        ),
        named(
            "receipt",
            call(
                "Logs and receipts",
                "receipt (succeeded)",
                "status 0x1, what it cost, and the two logs it made",
                "eth_getTransactionReceipt",
                format!(r#"["{SENT}"]"#),
            ),
        ),
        named(
            "receipt-failed",
            call(
                "Logs and receipts",
                "receipt (failed)",
                "Mined and did not go through: status 0x0, gas spent, no logs",
                "eth_getTransactionReceipt",
                format!(r#"["{FAILED}"]"#),
            ),
        ),
        named(
            "new-filter",
            call(
                "Logs and receipts",
                "eth_newFilter",
                "The same topics, asked for as they come",
                "eth_newFilter",
                format!(
                    r#"[{{"fromBlock":"{BLOCK}","address":"{TOKEN}","topics":["{}"]}}]"#,
                    topic("Transfer(address,address,uint256)")
                ),
            ),
        ),
        // The chain itself: where it is up to, and what it charges.
        named(
            "block-number",
            call(
                "Blocks and fees",
                "eth_blockNumber",
                "One word of hex, which is a block number",
                "eth_blockNumber",
                "[]".to_owned(),
            ),
        ),
        named(
            "block-by-number",
            call(
                "Blocks and fees",
                "getBlockByNumber",
                "A block, its transactions as hashes",
                "eth_getBlockByNumber",
                format!(r#"["{BLOCK}",false]"#),
            ),
        ),
        named(
            "fee-history",
            call(
                "Blocks and fees",
                "eth_feeHistory",
                "What four blocks charged, and what the next may",
                "eth_feeHistory",
                format!(r#"["0x4","{BLOCK}",[25,50,75]]"#),
            ),
        ),
        named(
            "gas-price",
            call(
                "Blocks and fees",
                "eth_gasPrice",
                "Wei, which the window says in gwei",
                "eth_gasPrice",
                "[]".to_owned(),
            ),
        ),
        named(
            "get-balance",
            call(
                "Blocks and fees",
                "eth_getBalance",
                "Wei again, which the window says in ether",
                "eth_getBalance",
                format!(r#"["{WHO}","latest"]"#),
            ),
        ),
        named(
            "get-code",
            call(
                "Blocks and fees",
                "eth_getCode",
                "The bytes of a contract, cut short on the line",
                "eth_getCode",
                format!(r#"["{TOKEN}","latest"]"#),
            ),
        ),
        named(
            "get-storage-at",
            call(
                "Blocks and fees",
                "eth_getStorageAt",
                "One slot of a contract's own memory",
                "eth_getStorageAt",
                format!(r#"["{TOKEN}","0x0","latest"]"#),
            ),
        ),
        // Several at once, and the ways a node says no.
        sample(
            "batch",
            "Batches and trouble",
            "A batch of three",
            "chainId, blockNumber and net_version in one request",
            captured!("batch.req.json"),
        ),
        named(
            "unknown-method",
            call(
                "Batches and trouble",
                "-32601",
                "The method does not exist: the plain JSON-RPC error",
                "eth_notAMethod",
                "[]".to_owned(),
            ),
        ),
        named(
            "insufficient-funds",
            call(
                "Batches and trouble",
                "-32000",
                "A node's own error: not enough for gas and value",
                "eth_estimateGas",
                format!(r#"[{{"from":"{TOKEN}","to":"{WHO}","value":"0xde0b6b3a7640000"}}]"#),
            ),
        ),
        Case {
            id: "rate-limited",
            group: "Batches and trouble",
            name: "429",
            what: "Turned away by the gateway, with the error in JSON-RPC",
            url: "/rpc?limit=1".to_owned(),
            body: r#"{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}"#.to_owned(),
        },
        Case {
            id: "slow",
            group: "Batches and trouble",
            name: "A slow node",
            what: "The same call, a second and a half later",
            url: "/rpc?ms=1500".to_owned(),
            body: r#"{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}"#.to_owned(),
        },
        // JSON-RPC that is not a chain at all, and a chain that is not
        // Ethereum: both are read as what they are.
        sample(
            "mcp",
            "Not Ethereum",
            "MCP tools/call",
            "JSON-RPC over HTTP, and nothing to do with a chain",
            captured!("mcp.req.json"),
        ),
        named(
            "solana-balance",
            call(
                "Not Ethereum",
                "Solana getBalance",
                "Lamports, and a commitment rather than a block",
                "getBalance",
                r#"["9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",{"commitment":"finalized"}]"#
                    .to_owned(),
            ),
        ),
        named(
            "solana-tx",
            call(
                "Not Ethereum",
                "Solana getTransaction",
                "A signature, asked for as JSON",
                "getTransaction",
                r#"["5h7xqBtVVsP9mEoYqNm3MW9rUYGSs9c1gkVNbWy5xDdAyjkCaA3wLbhDEA9NhcVy6vJgVrVbE1MxbvYcPrnJ3bEm",{"encoding":"json","maxSupportedTransactionVersion":0}]"#.to_owned(),
            ),
        ),
    ]
}

/// What a socket can be subscribed to, as `eth_subscribe` names it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feed {
    /// A block as it is mined.
    Heads,
    /// The logs of a contract, as they are written.
    Logs,
    /// The hash of a transaction that has not been mined yet.
    Pending,
}

impl Feed {
    /// What `eth_subscribe` was asked for, if it is one this node has.
    pub fn asked(call: &jsonrpc::Call) -> Option<Self> {
        let what = call.params.as_ref()?.get(0)?.as_str()?;
        Some(match what {
            "newHeads" => Self::Heads,
            "logs" => Self::Logs,
            "newPendingTransactions" => Self::Pending,
            _ => return None,
        })
    }

    /// The handle it is given, which `eth_unsubscribe` is called with. One
    /// for each feed, so that a window shows which is which.
    pub fn handle(self) -> &'static str {
        match self {
            Self::Heads => "0xcd0c3e8af590364c09d0fa6a1210faf5",
            Self::Logs => "0x9ce59a13059e417087c02d3236a0b1cc",
            Self::Pending => "0x5a3f1e8b2c7d4a6f9b0e3c5d7a1f4b8e",
        }
    }

    /// The handle, as the id of a subscription a client holds.
    pub fn of_handle(handle: &str) -> Option<Self> {
        [Self::Heads, Self::Logs, Self::Pending]
            .into_iter()
            .find(|feed| feed.handle().eq_ignore_ascii_case(handle))
    }

    /// The `n`th thing to come down this feed, as the node sends it: a
    /// notification, which has no id and wants no reply.
    pub fn notice(self, n: u64) -> String {
        let result = match self {
            Self::Heads => head(n),
            Self::Logs => log(n),
            Self::Pending => format!("\"{}\"", pending_hash(n)),
        };
        format!(
            concat!(
                r#"{{"jsonrpc":"2.0","method":"eth_subscription","#,
                r#""params":{{"subscription":"{}","result":{}}}}}"#
            ),
            self.handle(),
            result
        )
    }
}

/// A block as it is mined, counting on from the one the samples were taken
/// at so that a window sees the number move.
fn head(n: u64) -> String {
    let number = 0x192_bbde + n;
    format!(
        concat!(
            r#"{{"number":"{number:#x}","hash":"{hash}","#,
            r#""parentHash":"{parent}","timestamp":"{time:#x}","#,
            r#""gasLimit":"0x1c9c380","gasUsed":"0x{used:x}","#,
            r#""baseFeePerGas":"0x{fee:x}","miner":"{TOKEN}"}}"#
        ),
        number = number,
        hash = hash_like(0x6865_6164, n),
        parent = hash_like(0x6865_6164, n.wrapping_sub(1)),
        time = 0x6a4f_1b20 + n * 12,
        used = 0x1a2_b3c4 + n * 977,
        fee = 0x14ab_fc31 + n * 131,
        TOKEN = TOKEN
    )
}

/// One of the logs that were captured, over and over as the feed goes on.
fn log(n: u64) -> String {
    static LOGS: OnceLock<Vec<String>> = OnceLock::new();
    let logs = LOGS.get_or_init(|| {
        let captured: serde_json::Value =
            serde_json::from_str(captured!("get_logs.res.json")).expect("the sample is JSON");
        captured["result"]
            .as_array()
            .expect("an array of logs")
            .iter()
            .map(ToString::to_string)
            .collect()
    });
    logs[(n as usize) % logs.len()].clone()
}

/// A transaction that has been heard of but not mined.
fn pending_hash(n: u64) -> String {
    hash_like(0x7065_6e64, n)
}

/// Thirty-two bytes that look like a hash and are the same for the same
/// `what` and `n`: a hash of a few digits with the rest zeros fools nobody,
/// least of all a window showing it beside a real one.
fn hash_like(what: u64, n: u64) -> String {
    let mut seed = what ^ n.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut out = String::from("0x");
    for _ in 0..4 {
        // A word at a time, each stirred from the last.
        seed ^= seed >> 30;
        seed = seed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        seed ^= seed >> 27;
        seed = seed.wrapping_mul(0x94d0_49bb_1331_11eb);
        seed ^= seed >> 31;
        let _ = write!(out, "{seed:016x}");
    }
    out
}

/// What the node answers, with `fail` making every call revert.
pub fn answer(body: &[u8], fail: bool) -> String {
    let calls = jsonrpc::calls_in(body);
    if calls.is_empty() {
        return r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"invalid request"}}"#
            .to_owned();
    }
    let replies: Vec<String> = calls.iter().map(|call| one(call, fail)).collect();
    if body.trim_ascii_start().starts_with(b"[") {
        format!("[{}]", replies.join(","))
    } else {
        replies.join("")
    }
}

/// A reply to that call, with its id: for the answers a socket works out
/// for itself, such as what it has just been subscribed to.
pub fn answered(call: &jsonrpc::Call, result: &str) -> String {
    let id = call
        .id
        .as_ref()
        .map_or_else(|| "null".to_owned(), ToString::to_string);
    self::result(&id, result)
}

/// What a node says when it is asked for a feed it does not keep.
pub fn not_known(call: &jsonrpc::Call) -> String {
    let id = call
        .id
        .as_ref()
        .map_or_else(|| "null".to_owned(), ToString::to_string);
    format!(
        concat!(
            r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":-32601,"#,
            r#""message":"this node does not keep that feed"}}}}"#
        ),
        id = id
    )
}

/// What a node says when a gateway turns the request away.
pub fn too_many() -> &'static str {
    concat!(
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32005,"#,
        r#""message":"your app has exceeded its compute units per second capacity"}}"#
    )
}

fn one(call: &jsonrpc::Call, fail: bool) -> String {
    let id = call
        .id
        .as_ref()
        .map_or_else(|| "null".to_owned(), ToString::to_string);
    if fail {
        return reverted(&id, REASON);
    }
    let params = call.params.as_ref();
    let at = |n: usize| params.and_then(|p| p.get(n));
    let text_at = |n: usize| at(n).and_then(|v| v.as_str()).unwrap_or_default();
    let field = |name: &str| {
        at(0)
            .and_then(|o| o.get(name))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    match call.method.as_str() {
        "eth_call" => {
            let data = field("data");
            if starts(&data, "balanceOf(address)") {
                return with_id(captured!("call_balanceof.res.json"), &id);
            }
            if starts(&data, "transfer(address,uint256)") {
                return with_id(captured!("revert.res.json"), &id);
            }
            if starts(&data, "withdraw(uint256)") {
                return reverted(&id, &panic(0x11));
            }
            if starts(&data, "approve(address,uint256)") {
                // A custom error of a contract of its own, which is four
                // bytes and its arguments until an ABI names it.
                let custom = format!(
                    "0x{}{}{}",
                    hex(&web3::selector_of(
                        "ERC20InsufficientBalance(address,uint256,uint256)"
                    )),
                    address_word(WHO),
                    number_word(0)
                );
                return reverted(&id, &custom);
            }
            if starts(&data, "symbol()") {
                return result(&id, &format!("\"{}\"", abi_string("USDC")));
            }
            if starts(&data, "aggregate3((address,bool,bytes)[])") {
                return result(&id, &format!("\"{}\"", aggregate3_result()));
            }
            result(&id, "\"0x\"")
        }
        "eth_estimateGas" => {
            // A node answers a call it cannot pay for with an error of its
            // own, not with a revert.
            if field("value").is_empty() {
                result(&id, "\"0xb0d8\"")
            } else {
                format!(
                    concat!(
                        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32000,"#,
                        r#""message":"insufficient funds for gas * price + value: "#,
                        r#"address {} have 0 want 1000000000000000000"}}}}"#
                    ),
                    id, TOKEN
                )
            }
        }
        "eth_sendRawTransaction" | "eth_sendTransaction" | "eth_signTransaction" => {
            result(&id, &format!("\"{SENT}\""))
        }
        "eth_getTransactionByHash" => match text_at(0) {
            LEGACY => with_id(captured!("tx_legacy.res.json"), &id),
            SENT => with_id(captured!("tx.res.json"), &id),
            _ => result(&id, "null"),
        },
        "eth_getTransactionReceipt" => match text_at(0) {
            FAILED => with_id(captured!("receipt_failed.res.json"), &id),
            SENT => with_id(captured!("receipt.res.json"), &id),
            _ => result(&id, "null"),
        },
        "eth_getLogs" => with_id(captured!("get_logs.res.json"), &id),
        "eth_newFilter" => result(&id, "\"0x1b1a9a4f1c57c7a2b0a53e5d0f3a4c8d\""),
        "eth_getFilterChanges" => with_id(captured!("get_logs.res.json"), &id),
        "eth_chainId" => result(&id, "\"0x1\""),
        "net_version" => result(&id, "\"1\""),
        "web3_clientVersion" => result(&id, "\"tracewing-playground/v0.1.0\""),
        "eth_blockNumber" => result(&id, &format!("\"{BLOCK}\"")),
        "eth_gasPrice" => result(&id, "\"0x14abfc31\""),
        "eth_maxPriorityFeePerGas" => result(&id, "\"0x5f5e100\""),
        "eth_blobBaseFee" => result(&id, "\"0x1\""),
        "eth_feeHistory" => result(&id, FEE_HISTORY),
        "eth_getBalance" => result(&id, "\"0x506a9d72ec36d56a\""),
        "eth_getTransactionCount" => result(&id, "\"0x4d2\""),
        "eth_getCode" => result(&id, &format!("\"{CODE}\"")),
        "eth_getStorageAt" => result(
            &id,
            "\"0x0000000000000000000000005b24a0f0cbab79d9e7cf05a5cf5c28af0dc20ccc\"",
        ),
        "eth_getBlockByNumber" | "eth_getBlockByHash" => result(&id, &block()),
        "eth_getBlockReceipts" => {
            let receipt = with_id(captured!("receipt.res.json"), &id);
            let inside = receipt
                .find("\"result\":")
                .map_or("[]".to_owned(), |at| format!("[{}", &receipt[at + 9..]));
            format!(
                r#"{{"jsonrpc":"2.0","id":{id},"result":{}]}}"#,
                inside.trim_end_matches('}')
            )
        }
        "eth_subscribe" => result(&id, "\"0xcd0c3e8af590364c09d0fa6a1210faf5\""),
        "eth_unsubscribe" => result(&id, "true"),
        // Solana asks in its own words, and answers with JSON rather than
        // hex, which the window says as what it is.
        "getBalance" => result(&id, r#"{"context":{"slot":308291043},"value":1234567890}"#),
        "getLatestBlockhash" => result(
            &id,
            concat!(
                r#"{"context":{"slot":308291043},"value":{"#,
                r#""blockhash":"8vZ5oG2b1r4QJ3qZ8cZ1Jk5Ld8QhYtMwBq9XvK3nR2pA","#,
                r#""lastValidBlockHeight":286471939}}"#
            ),
        ),
        "getTransaction" => result(&id, SOLANA_TX),
        // MCP over HTTP: JSON-RPC that is not a chain at all.
        "tools/call" => result(&id, MCP_RESULT),
        "initialize" => result(
            &id,
            r#"{"protocolVersion":"2025-06-18","serverInfo":{"name":"playground","version":"0.1.0"}}"#,
        ),
        other => format!(
            concat!(
                r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32601,"#,
                r#""message":"the method {} does not exist/is not available"}}}}"#
            ),
            id, other
        ),
    }
}

/// Whether a call's `data` is that function's.
fn starts(data: &str, signature: &str) -> bool {
    let four = format!("0x{}", hex(&web3::selector_of(signature)));
    data.len() >= 10 && data[..10].eq_ignore_ascii_case(&four)
}

fn result(id: &str, result: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#)
}

/// A call that did not go through: a 200 with the data it reverted with.
fn reverted(id: &str, data: &str) -> String {
    let said = if data == REASON {
        "execution reverted: ERC20: transfer amount exceeds balance"
    } else {
        "execution reverted"
    };
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":3,"message":"{said}","data":"{data}"}}}}"#
    )
}

/// The id of the request put into a captured answer, so that a client which
/// numbers its calls its own way still has its reply matched.
fn with_id(captured: &str, id: &str) -> String {
    let Some(at) = captured.find("\"id\":") else {
        return captured.trim().to_owned();
    };
    let from = at + 5;
    let had = captured[from..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .count();
    format!(
        "{}{id}{}",
        &captured[..from],
        captured[from + had..].trim_end()
    )
}

/// `0x`, four bytes of the function, then its arguments, as calldata.
fn data(signature: &str, words: &[String]) -> String {
    format!("0x{}{}", hex(&web3::selector_of(signature)), words.concat())
}

/// What an event's first topic is.
fn topic(signature: &str) -> String {
    format!("0x{}", hex(&web3::topic_of(signature)))
}

fn address_word(address: &str) -> String {
    format!("{:0>64}", address.trim_start_matches("0x").to_lowercase())
}

fn number_word(number: u128) -> String {
    format!("{number:064x}")
}

fn max_word() -> String {
    "f".repeat(64)
}

/// A string as a call answers with one: where it starts, how long it is,
/// then its bytes, each in a word of its own.
fn abi_string(text: &str) -> String {
    let mut out = format!("0x{}{:064x}", number_word(32), text.len());
    let mut bytes = text.as_bytes().to_vec();
    bytes.resize(bytes.len().div_ceil(32) * 32, 0);
    out.push_str(&hex(&bytes));
    out
}

/// What `aggregate3` answers: four calls, each with whether it went through
/// and what it said.
fn aggregate3_result() -> String {
    let word = |n: u128| number_word(n);
    let mut out = String::from("0x");
    out.push_str(&word(32)); // where the array starts
    out.push_str(&word(4)); // how many
    for at in 0..4u128 {
        // where each tuple starts, after the four offsets
        out.push_str(&word(128 + at * 128));
    }
    for at in 0..4u128 {
        out.push_str(&word(1)); // success
        out.push_str(&word(64)); // where its bytes start
        out.push_str(&word(32)); // how many bytes
        out.push_str(&word(39_192_124 + at));
    }
    out
}

fn panic(code: u128) -> String {
    format!(
        "0x{}{}",
        hex(&web3::selector_of("Panic(uint256)")),
        number_word(code)
    )
}

fn block() -> String {
    format!(
        concat!(
            r#"{{"number":"{BLOCK}","hash":"0x9d4b1f5a0c7e3b2d8a6f4c1e9b7d5a3f2c8e6b4d1a9f7c5e3b1d9a7f5c3e1b9d","#,
            r#""parentHash":"0x4f2e1c9a7b5d3f1e9c7a5b3d1f9e7c5a3b1d9f7e5c3a1b9d7f5e3c1a9b7d5f3e","#,
            r#""timestamp":"0x6a4f1b20","gasLimit":"0x1c9c380","gasUsed":"0x1a2b3c4","#,
            r#""baseFeePerGas":"0x14abfc31","miner":"{TOKEN}","size":"0x1f4c","#,
            r#""transactions":["{SENT}","{LEGACY}","{FAILED}"]}}"#
        ),
        BLOCK = BLOCK,
        SENT = SENT,
        LEGACY = LEGACY,
        FAILED = FAILED,
        TOKEN = TOKEN
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The data an `Error(string)` reverts with, as the samples have it.
const REASON: &str = concat!(
    "0x08c379a000000000000000000000000000000000000000000000000000000000",
    "0000002000000000000000000000000000000000000000000000000000000000",
    "0000002645524332303a207472616e7366657220616d6f756e74206578636565",
    "64732062616c616e63650000000000000000000000000000000000000000000000000000"
);

const FEE_HISTORY: &str = concat!(
    r#"{"oldestBlock":"0x192bbda","#,
    r#""baseFeePerGas":["0x1424ef52","0x13c8d8a1","0x14abfc31","0x1510a2b4","0x1489e3c7"],"#,
    r#""gasUsedRatio":[0.4821,0.5133,0.6042,0.4417],"#,
    r#""reward":[["0x5f5e100","0xbebc200","0x17d78400"],["0x5f5e100","0xbebc200","0x1dcd6500"],"#,
    r#"["0x3b9aca00","0xbebc200","0x165a0bc0"],["0x5f5e100","0xaba9500","0x12a05f200"]]}"#
);

/// A contract's code, long enough to be cut on a line and opened whole.
const CODE: &str = concat!(
    "0x608060405234801561001057600080fd5b50600436106101005760003560e01c",
    "8063a9059cbb11610097578063dd62ed3e11610066578063dd62ed3e1461027a",
    "578063e78cea9214610290578063f2fde38b146102a657600080fd5b8063a905",
    "9cbb1461021a57806395d89b411461023a578063a457c2d71461024f57600080",
    "fd5b806323b872dd116100d357806323b872dd146101a0578063313ce567146101",
    "c057806339509351146101dc57806370a08231146101fc57600080fd5b806306fd",
    "de0314610105578063095ea7b31461013057806318160ddd14610160575b600080fd"
);

const SOLANA_TX: &str = concat!(
    r#"{"slot":308291043,"blockTime":1767225600,"#,
    r#""meta":{"err":null,"fee":5000,"preBalances":[1234567890,2039280],"#,
    r#""postBalances":[1234562890,2039280],"logMessages":["#,
    r#""Program 11111111111111111111111111111111 invoke [1]","#,
    r#""Program 11111111111111111111111111111111 success"]},"#,
    r#""transaction":{"message":{"accountKeys":["#,
    r#""9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM","#,
    r#""11111111111111111111111111111111"],"recentBlockhash":"#,
    r#""8vZ5oG2b1r4QJ3qZ8cZ1Jk5Ld8QhYtMwBq9XvK3nR2pA"}}}"#
);

const MCP_RESULT: &str = concat!(
    r#"{"content":[{"type":"text","text":"5 flows: GET /api/orders 200, "#,
    r#"POST /api/orders 201, GET /health 200, GET /assets/app.js 304, "#,
    r#"POST /rpc 200"}],"isError":false}"#
);

#[cfg(test)]
mod tests {
    use super::*;

    fn said(body: &str) -> String {
        answer(body.as_bytes(), false)
    }

    #[test]
    fn every_case_is_answered_as_itself() {
        for case in cases() {
            let reply = said(&case.body);
            // The trouble group is there for the errors; everything else is
            // answered by a node that knows the call.
            if case.group != "Batches and trouble" {
                assert!(
                    !reply.contains("-32600") && !reply.contains("does not exist"),
                    "{}: {reply}",
                    case.id
                );
            }
            // A reply carries the id it was asked with, so that a client can
            // match them up.
            let calls = jsonrpc::calls_in(case.body.as_bytes());
            assert!(!calls.is_empty(), "{}: not JSON-RPC", case.id);
            for call in &calls {
                if let Some(id) = &call.id {
                    assert!(
                        reply.contains(&format!("\"id\":{id}")),
                        "{}: {reply}",
                        case.id
                    );
                }
            }
        }
    }

    #[test]
    fn the_cases_that_were_captured_are_answered_with_what_came_back() {
        let balance = said(case("balance-of").unwrap().body.as_str());
        assert!(
            balance.contains("0x000000000000000000000000000000000000000000000000000000000256063c")
        );
        let logs = said(&case("get-logs").unwrap().body);
        assert_eq!(logs.matches("\"topics\"").count(), 3, "{logs}");
        let receipt = said(&case("receipt").unwrap().body);
        assert!(receipt.contains(r#""status":"0x1""#), "{receipt}");
        let failed = said(&case("receipt-failed").unwrap().body);
        assert!(failed.contains(r#""status":"0x0""#), "{failed}");
        let legacy = said(&case("tx-by-hash-legacy").unwrap().body);
        assert!(legacy.contains(r#""type":"0x0""#), "{legacy}");
    }

    #[test]
    fn a_revert_comes_with_what_it_reverted_with() {
        let reason = said(&case("revert-reason").unwrap().body);
        assert!(
            reason.contains("transfer amount exceeds balance"),
            "{reason}"
        );
        let data = reason
            .split(r#""data":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        let bytes = web3::bytes_of(data).unwrap();
        assert!(matches!(
            web3::Revert::of(&bytes),
            Some(web3::Revert::Reason(_))
        ));

        let panicked = said(&case("revert-panic").unwrap().body);
        let data = panicked
            .split(r#""data":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        let bytes = web3::bytes_of(data).unwrap();
        assert_eq!(web3::Revert::of(&bytes), Some(web3::Revert::Panic(0x11)));

        let custom = said(&case("revert-custom").unwrap().body);
        let data = custom
            .split(r#""data":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        let bytes = web3::bytes_of(data).unwrap();
        assert!(matches!(
            web3::Revert::of(&bytes),
            Some(web3::Revert::Custom(_))
        ));
    }

    #[test]
    fn a_batch_is_answered_call_for_call() {
        let batch = said(&case("batch").unwrap().body);
        assert!(batch.starts_with('['), "{batch}");
        assert_eq!(batch.matches("\"result\"").count(), 3, "{batch}");
    }

    #[test]
    fn what_a_call_answers_reads_as_what_it_asked_for() {
        // A string answer is a string once it is decoded, not a word of hex.
        let symbol = said(&case("symbol").unwrap().body);
        let data = symbol
            .split(r#""result":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        let bytes = web3::bytes_of(data).unwrap();
        let args = web3::returned("(string)", &bytes).unwrap();
        assert_eq!(args[0].text, "USDC", "{symbol}");
    }

    #[test]
    fn what_a_multicall_answers_reads_as_its_tuples() {
        let reply = said(&case("multicall").unwrap().body);
        let data = reply
            .split(r#""result":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        let bytes = web3::bytes_of(data).unwrap();
        let args = web3::returned("((bool,bytes)[])", &bytes)
            .unwrap_or_else(|| panic!("four tuples: {data}"));
        assert!(args[0].text.contains("true"), "{}", args[0].text);
    }

    #[test]
    fn what_comes_down_a_feed_is_shaped_like_a_chain() {
        let looks_like_a_hash = |hash: &str| {
            hash.len() == 66
                && hash.starts_with("0x")
                && hash[2..].chars().all(|c| c.is_ascii_hexdigit())
        };
        let read = |text: &str| {
            serde_json::from_str::<serde_json::Value>(text)
                .unwrap_or_else(|trouble| panic!("{trouble}: {text}"))
        };

        let first = read(&Feed::Heads.notice(0));
        assert_eq!(first["method"], "eth_subscription");
        assert!(first["id"].is_null(), "a notification has no id");
        assert_eq!(first["params"]["subscription"], Feed::Heads.handle());
        let head = &first["params"]["result"];
        for name in ["hash", "parentHash"] {
            let hash = head[name].as_str().unwrap_or_default();
            assert!(looks_like_a_hash(hash), "{name} is {hash}");
        }
        // A block further on each time, and its parent is the one before.
        let next = read(&Feed::Heads.notice(1));
        assert_eq!(head["hash"], next["params"]["result"]["parentHash"]);
        assert_ne!(head["number"], next["params"]["result"]["number"]);

        // The logs are the ones that were captured, topics and all.
        let logged = read(&Feed::Logs.notice(0));
        let log = &logged["params"]["result"];
        assert!(
            log["topics"].as_array().is_some_and(|t| t.len() == 3),
            "{log}"
        );
        assert_ne!(Feed::Logs.notice(0), Feed::Logs.notice(1));

        // And a pending transaction is a hash on its own.
        let pending = read(&Feed::Pending.notice(0));
        let hash = pending["params"]["result"].as_str().unwrap_or_default();
        assert!(looks_like_a_hash(hash), "{hash}");
    }

    #[test]
    fn fail_makes_every_call_revert() {
        let reply = answer(case("balance-of").unwrap().body.as_bytes(), true);
        assert!(reply.contains("\"code\":3"), "{reply}");
    }

    #[test]
    fn what_is_not_json_rpc_is_turned_away() {
        assert!(answer(b"", false).contains("-32600"));
        assert!(answer(b"{\"hello\":1}", false).contains("-32600"));
    }
}
