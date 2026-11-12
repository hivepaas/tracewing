//! What an Ethereum call holds under the hex: the function a `data` field
//! names, the transaction `eth_sendRawTransaction` sends as hundreds of bytes
//! of RLP, the reason a call did not go through, and the numbers, which come
//! as hex and mean wei until something says otherwise.
//!
//! On top of [`crate::jsonrpc`], which says that a call is JSON-RPC and which
//! method it names. This says what the hex inside it means.
//!
//! Nothing here asks the network. A function is named because its text is in
//! [`FUNCTIONS`], and the four bytes are worked out from that text here: a
//! signature directory is never asked, which would send what the user's
//! wallet is doing to a stranger.

use alloy_dyn_abi::{DynSolType, DynSolValue};
use sha3::{Digest, Keccak256};
use std::{collections::HashMap, sync::OnceLock};

/// The bytes a `DATA` field holds: `0x`, then two hex digits a byte.
pub fn bytes_of(hex: &str) -> Option<Vec<u8>> {
    let digits = hex.strip_prefix("0x").or_else(|| hex.strip_prefix("0X"))?;
    if digits.len() % 2 != 0 {
        return None;
    }
    digits
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// The bytes a `QUANTITY` holds, of any size. It comes with no leading
/// zeros, so its digits may be an odd number where a `DATA` field's never
/// are: that is why this is not [`bytes_of`]. For one past a machine word,
/// as a balance can be, this and [`digits`] read it where [`quantity`]
/// cannot.
pub fn quantity_bytes(hex: &str) -> Option<Vec<u8>> {
    let digits = hex.strip_prefix("0x").or_else(|| hex.strip_prefix("0X"))?;
    match digits.len() % 2 {
        0 => bytes_of(hex),
        _ => bytes_of(&format!("0x0{digits}")),
    }
}

/// A `QUANTITY`, which comes with no leading zeros and so may have an odd
/// number of digits. `None` when it is not one, or holds more than a machine
/// word: every quantity a chain sends fits, where a `uint256` a call returns
/// need not, and [`digits`] is for those.
pub fn quantity(hex: &str) -> Option<u128> {
    let digits = hex.strip_prefix("0x").or_else(|| hex.strip_prefix("0X"))?;
    if digits.is_empty() || digits.len() > 32 {
        return None;
    }
    u128::from_str_radix(digits, 16).ok()
}

/// The digits of a big-endian number of any length. A `uint256` does not fit
/// a machine word: an allowance of no limit has 78 digits.
pub fn digits(bytes: &[u8]) -> String {
    let mut left = bytes.to_vec();
    let mut out = Vec::new();
    while left.iter().any(|byte| *byte != 0) {
        let mut carry = 0u16;
        for byte in &mut left {
            let now = (carry << 8) | u16::from(*byte);
            *byte = (now / 10) as u8;
            carry = now % 10;
        }
        out.push(b'0' + carry as u8);
    }
    if out.is_empty() {
        return "0".to_owned();
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// `1.5` from `1500000000000000000` and 18: the point put in the digits, with
/// the noughts after it left off. Ether has 18 decimals, USDC six.
pub fn scaled(digits: &str, decimals: usize) -> String {
    if decimals == 0 {
        return digits.to_owned();
    }
    let padded = format!("{digits:0>width$}", width = decimals + 1);
    let (whole, part) = padded.split_at(padded.len() - decimals);
    let part = part.trim_end_matches('0');
    match part.is_empty() {
        true => whole.to_owned(),
        false => format!("{whole}.{part}"),
    }
}

/// `1.5 ETH`, from the wei as they came.
pub fn ether(wei: &[u8]) -> String {
    format!("{} ETH", scaled(&digits(wei), 18))
}

/// `0.17 gwei`, which is how a fee is read.
pub fn gwei(wei: u128) -> String {
    format!("{} gwei", scaled(&wei.to_string(), 9))
}

/// An address as EIP-55 writes it: the hex, with the capitals that check it.
/// One digit wrong and the capitals no longer fit, which is what they are for.
pub fn address_text(bytes: &[u8]) -> String {
    let lower = hex_text(bytes)[2..].to_owned();
    let hash = Keccak256::digest(lower.as_bytes());
    let mut out = String::with_capacity(2 + lower.len());
    out.push_str("0x");
    for (i, digit) in lower.chars().enumerate() {
        // The nibble of the hash that stands for this digit.
        let nibble = match i % 2 {
            0 => hash[i / 2] >> 4,
            _ => hash[i / 2] & 0xf,
        };
        match nibble >= 8 {
            true => out.extend(digit.to_uppercase()),
            false => out.push(digit),
        }
    }
    out
}

/// `0x70a08231`, as a `data` field starts.
pub fn hex_text(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(2 + bytes.len() * 2);
    out.push_str("0x");
    for byte in bytes {
        // Written straight in: calldata runs to hundreds of bytes, and can
        // run to hundreds of thousands.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The four bytes a `data` field starts with: the first four of the
/// keccak-256 of the signature.
pub fn selector_of(signature: &str) -> [u8; 4] {
    let hash = Keccak256::digest(signature.as_bytes());
    [hash[0], hash[1], hash[2], hash[3]]
}

/// An event's first topic: the whole keccak-256 of its signature.
pub fn topic_of(signature: &str) -> [u8; 32] {
    Keccak256::digest(signature.as_bytes()).into()
}

/// The calls that nearly every chain has, as their text. The four bytes are
/// worked out from it, so what is written here can be read and checked; four
/// bytes written by hand could be wrong and nobody would see it.
pub const FUNCTIONS: &[&str] = &[
    // ERC-20.
    "transfer(address,uint256)",
    "transferFrom(address,address,uint256)",
    "approve(address,uint256)",
    "balanceOf(address)",
    "allowance(address,address)",
    "totalSupply()",
    "decimals()",
    "symbol()",
    "name()",
    "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)",
    // ERC-721 and ERC-1155.
    "ownerOf(uint256)",
    "safeTransferFrom(address,address,uint256)",
    "safeTransferFrom(address,address,uint256,bytes)",
    "setApprovalForAll(address,bool)",
    "isApprovedForAll(address,address)",
    "tokenURI(uint256)",
    "uri(uint256)",
    // Wrapped ether.
    "deposit()",
    "withdraw(uint256)",
    // What a wallet asks a node for, through a contract.
    "multicall(bytes[])",
    "aggregate3((address,bool,bytes)[])",
];

/// The events that nearly every chain has, as their text.
pub const EVENTS: &[&str] = &[
    "Transfer(address,address,uint256)",
    "Approval(address,address,uint256)",
    "ApprovalForAll(address,address,bool)",
    "TransferSingle(address,address,address,uint256,uint256)",
    "TransferBatch(address,address,address,uint256[],uint256[])",
    "Deposit(address,uint256)",
    "Withdrawal(address,uint256)",
    "OwnershipTransferred(address,address)",
];

/// The function those four bytes name, if it is one of [`FUNCTIONS`]. The
/// `data` of a call may be passed whole: only its first four bytes are read.
pub fn known_function(data: &[u8]) -> Option<&'static str> {
    static TABLE: OnceLock<HashMap<[u8; 4], &'static str>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        FUNCTIONS
            .iter()
            .map(|signature| (selector_of(signature), *signature))
            .collect()
    });
    let four: [u8; 4] = data.get(..4)?.try_into().ok()?;
    table.get(&four).copied()
}

/// The event that first topic names, if it is one of [`EVENTS`].
pub fn known_event(topic: &[u8]) -> Option<&'static str> {
    static TABLE: OnceLock<HashMap<[u8; 32], &'static str>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        EVENTS
            .iter()
            .map(|signature| (topic_of(signature), *signature))
            .collect()
    });
    let whole: [u8; 32] = topic.get(..32)?.try_into().ok()?;
    table.get(&whole).copied()
}

/// The name a signature gives, without its types: `transfer`.
pub fn name_of(signature: &str) -> &str {
    signature.split('(').next().unwrap_or(signature)
}

/// Why a call did not go through, from the `data` a node sends with the error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Revert {
    /// What `require(…, "why")` and `revert("why")` say.
    Reason(String),
    /// What the compiler itself raises: an assert that did not hold, a sum
    /// that overflowed, a division by zero.
    Panic(u128),
    /// A custom error. Only its four bytes, until an ABI names it.
    Custom([u8; 4]),
}

impl Revert {
    /// What the `data` of a failed call holds. `None` when there is none, as
    /// a call that ran out of gas sends none.
    pub fn of(data: &[u8]) -> Option<Self> {
        let four: [u8; 4] = data.get(..4)?.try_into().ok()?;
        let words = &data[4..];
        // Whichever it is, it reverted: a payload that does not read leaves
        // the four bytes, which say more than nothing at all.
        if four == selector_of("Error(string)") {
            return Some(match reason_of(words) {
                Some(why) => Self::Reason(why),
                None => Self::Custom(four),
            });
        }
        if four == selector_of("Panic(uint256)") {
            return Some(match words.get(..32).and_then(word_u128) {
                Some(code) => Self::Panic(code),
                None => Self::Custom(four),
            });
        }
        Some(Self::Custom(four))
    }

    /// What it says, in a line.
    pub fn text(&self) -> String {
        match self {
            Self::Reason(why) => why.clone(),
            Self::Panic(code) => match panic_text(*code) {
                Some(what) => format!("{what} (panic {code:#x})"),
                None => format!("panic {code:#x}"),
            },
            Self::Custom(four) => hex_text(four),
        }
    }
}

/// The string an `Error(string)` holds: where it starts, how long it is,
/// then its bytes.
fn reason_of(words: &[u8]) -> Option<String> {
    let at = word_usize(words.get(..32)?)?;
    let len = word_usize(words.get(at..at + 32)?)?;
    let text = words.get(at + 32..at + 32 + len)?;
    Some(String::from_utf8_lossy(text).into_owned())
}

/// What a panic code means, as Solidity's own list has it.
pub fn panic_text(code: u128) -> Option<&'static str> {
    Some(match code {
        0x01 => "an assert did not hold",
        0x11 => "a sum or a product overflowed",
        0x12 => "a division by zero",
        0x21 => "a number that is not one of the enum's",
        0x22 => "a string or array that was stored wrong",
        0x31 => "pop from an empty array",
        0x32 => "an index past the end of an array",
        0x41 => "out of memory",
        0x51 => "a call to a function with no body",
        _ => return None,
    })
}

/// One argument of a call, as it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    /// The type the signature gave it, as `address` or `uint256`.
    pub kind: String,
    pub text: String,
}

/// How deep a value is laid out before what is left is counted instead: a
/// call that nests deeper than this is one the Body part reads better.
const DEPTH_MAX: usize = 3;
/// How many of a list's items are laid out before the rest are counted.
const ITEMS_SHOWN: usize = 8;

/// What the words after the four bytes hold, by the types the signature
/// names. `None` when the signature does not read, or the words are not laid
/// out as it says.
///
/// Calldata inside the call — what a multicall carries — is named by the
/// calls every chain shares. [`arguments_named`] takes an ABI's names too.
pub fn arguments(signature: &str, data: &[u8]) -> Option<Vec<Argument>> {
    arguments_named(signature, data, &|data| {
        known_function(data).map(str::to_owned)
    })
}

/// The same, where `name` says what a `bytes` of calldata calls: an ABI the
/// user chose knows more names than [`FUNCTIONS`] does, and this is how they
/// reach in here without this module knowing about them.
pub fn arguments_named(
    signature: &str,
    data: &[u8],
    name: &dyn Fn(&[u8]) -> Option<String>,
) -> Option<Vec<Argument>> {
    let open = signature.find('(')?;
    let kind: DynSolType = signature[open..].parse().ok()?;
    let DynSolType::Tuple(types) = &kind else {
        return None;
    };
    if types.is_empty() {
        return Some(Vec::new());
    }
    let DynSolValue::Tuple(values) = kind.abi_decode_params(data.get(4..)?).ok()? else {
        return None;
    };
    Some(
        types
            .iter()
            .zip(values.iter())
            .map(|(kind, value)| Argument {
                kind: kind.sol_type_name().into_owned(),
                text: value_text(value, 0, name),
            })
            .collect(),
    )
}

/// One value as it reads: an address with the capitals that check it, a
/// number as digits however long, a list or a tuple with its parts inside.
fn value_text(value: &DynSolValue, depth: usize, name: &dyn Fn(&[u8]) -> Option<String>) -> String {
    match value {
        // Alloy writes an address with EIP-55's capitals, as this does.
        DynSolValue::Address(address) => address.to_string(),
        DynSolValue::Bool(yes) => yes.to_string(),
        DynSolValue::Uint(number, _) => number.to_string(),
        DynSolValue::Int(number, _) => number.to_string(),
        DynSolValue::FixedBytes(bytes, size) => hex_text(&bytes[..*size.min(&32)]),
        DynSolValue::Bytes(bytes) => calldata_text(bytes, name),
        DynSolValue::String(text) => text.clone(),
        DynSolValue::Array(items) | DynSolValue::FixedArray(items) => {
            list_text(items, depth, name, '[', ']')
        }
        DynSolValue::Tuple(items) => list_text(items, depth, name, '(', ')'),
        other => other.sol_type_name().unwrap_or_default().into_owned(),
    }
}

/// A `bytes` is often calldata of its own: a multicall carries one a call.
/// Said by the name it calls where that is known, since the hex of it says
/// nothing.
fn calldata_text(bytes: &[u8], name: &dyn Fn(&[u8]) -> Option<String>) -> String {
    let count = bytes.len();
    match name(bytes) {
        Some(signature) => format!("{signature}, {count} bytes"),
        None if count >= 4 => format!("{}…, {count} bytes", hex_text(&bytes[..4])),
        None => format!("{count} bytes"),
    }
}

fn list_text(
    items: &[DynSolValue],
    depth: usize,
    name: &dyn Fn(&[u8]) -> Option<String>,
    open: char,
    close: char,
) -> String {
    if depth >= DEPTH_MAX {
        return items_text(items.len());
    }
    let shown: Vec<String> = items
        .iter()
        .take(ITEMS_SHOWN)
        .map(|item| value_text(item, depth + 1, name))
        .collect();
    let more = items.len().saturating_sub(ITEMS_SHOWN);
    let rest = match more {
        0 => String::new(),
        more => format!(", … {more} more"),
    };
    format!("{open}{}{rest}{close}", shown.join(", "))
}

fn items_text(count: usize) -> String {
    match count {
        1 => "1 item".to_owned(),
        n => format!("{n} items"),
    }
}

/// A word as a length or a place in the data, which must fit a machine word:
/// one that does not is not either of those.
fn word_usize(word: &[u8]) -> Option<usize> {
    let value = word_u128(word)?;
    usize::try_from(value).ok()
}

fn word_u128(word: &[u8]) -> Option<u128> {
    if word.len() != 32 || word[..16].iter().any(|byte| *byte != 0) {
        return None;
    }
    Some(u128::from_be_bytes(word[16..].try_into().ok()?))
}

/// An item of RLP: bytes, or a list of items.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Item<'a> {
    Bytes(&'a [u8]),
    List(Vec<Item<'a>>),
}

impl<'a> Item<'a> {
    /// The item at the front of `input`, and what is left after it.
    fn read(input: &'a [u8]) -> Option<(Self, &'a [u8])> {
        let (&first, rest) = input.split_first()?;
        match first {
            // A byte under 0x80 is its own encoding.
            0x00..=0x7f => Some((Self::Bytes(&input[..1]), rest)),
            0x80..=0xb7 => {
                let (bytes, rest) = rest.split_at_checked(usize::from(first - 0x80))?;
                Some((Self::Bytes(bytes), rest))
            }
            0xb8..=0xbf => {
                let (len, rest) = long_length(rest, usize::from(first - 0xb7))?;
                let (bytes, rest) = rest.split_at_checked(len)?;
                Some((Self::Bytes(bytes), rest))
            }
            0xc0..=0xf7 => {
                let (payload, rest) = rest.split_at_checked(usize::from(first - 0xc0))?;
                Some((Self::List(items(payload)?), rest))
            }
            0xf8..=0xff => {
                let (len, rest) = long_length(rest, usize::from(first - 0xf7))?;
                let (payload, rest) = rest.split_at_checked(len)?;
                Some((Self::List(items(payload)?), rest))
            }
        }
    }

    fn bytes(&self) -> Option<&'a [u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::List(_) => None,
        }
    }

    fn list(&self) -> Option<&[Item<'a>]> {
        match self {
            Self::List(items) => Some(items),
            Self::Bytes(_) => None,
        }
    }

    /// A number, which RLP sends big-endian with no leading zeros.
    fn number(&self) -> Option<u128> {
        let bytes = self.bytes()?;
        if bytes.len() > 16 {
            return None;
        }
        let mut value = 0u128;
        for byte in bytes {
            value = (value << 8) | u128::from(*byte);
        }
        Some(value)
    }
}

fn long_length(input: &[u8], of: usize) -> Option<(usize, &[u8])> {
    let (bytes, rest) = input.split_at_checked(of)?;
    let mut len = 0usize;
    for byte in bytes {
        len = len.checked_mul(256)?.checked_add(usize::from(*byte))?;
    }
    Some((len, rest))
}

fn items(mut payload: &[u8]) -> Option<Vec<Item<'_>>> {
    let mut out = Vec::new();
    while !payload.is_empty() {
        let (item, rest) = Item::read(payload)?;
        out.push(item);
        payload = rest;
    }
    Some(out)
}

/// A list's payload: what is inside its header.
fn list_payload(input: &[u8]) -> Option<&[u8]> {
    let (&first, rest) = input.split_first()?;
    match first {
        0xc0..=0xf7 => rest.get(..usize::from(first - 0xc0)),
        0xf8..=0xff => {
            let (len, rest) = long_length(rest, usize::from(first - 0xf7))?;
            rest.get(..len)
        }
        _ => None,
    }
}

/// Where the `n`th item of a payload begins, counted from its start. The
/// items of a transaction are laid out one after another, so this says where
/// the signed part of one ends without encoding anything again.
fn field_at(payload: &[u8], n: usize) -> Option<usize> {
    let mut rest = payload;
    for _ in 0..n {
        let (_, next) = Item::read(rest)?;
        rest = next;
    }
    Some(payload.len() - rest.len())
}

/// The header of a list that long.
fn list_header(len: usize) -> Vec<u8> {
    if len <= 55 {
        return vec![0xc0 + len as u8];
    }
    let be = len.to_be_bytes();
    let from = be.iter().position(|byte| *byte != 0).unwrap_or(0);
    let mut out = vec![0xf7 + (be.len() - from) as u8];
    out.extend(&be[from..]);
    out
}

/// A number, as RLP sends one: big-endian with no leading zeros, and a
/// nothing of its own for zero.
fn rlp_number(value: u128) -> Vec<u8> {
    if value == 0 {
        return vec![0x80];
    }
    let be = value.to_be_bytes();
    let from = be.iter().position(|byte| *byte != 0).unwrap_or(0);
    let body = &be[from..];
    if body.len() == 1 && body[0] < 0x80 {
        return body.to_vec();
    }
    let mut out = vec![0x80 + body.len() as u8];
    out.extend(body);
    out
}

/// A 32-byte word from bytes RLP stripped the leading zeros off.
fn padded(bytes: &[u8]) -> Option<[u8; 32]> {
    if bytes.len() > 32 {
        return None;
    }
    let mut out = [0u8; 32];
    out[32 - bytes.len()..].copy_from_slice(bytes);
    Some(out)
}

/// Who signed a transaction, from the hash of what was signed and the
/// signature over it: the public key is recovered from the two, and the
/// address is the end of its hash.
fn sender(hash: &[u8], r: &[u8], s: &[u8], parity: u8) -> Option<String> {
    use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
    let signature = Signature::from_scalars(padded(r)?, padded(s)?).ok()?;
    let id = RecoveryId::from_byte(parity)?;
    let key = VerifyingKey::recover_from_prehash(hash, &signature, id).ok()?;
    let point = key.to_encoded_point(false);
    let hashed = Keccak256::digest(point.as_bytes().get(1..)?);
    Some(address_text(hashed.get(12..)?))
}

/// A signed transaction, as `eth_sendRawTransaction` carries it: RLP, with a
/// byte in front since EIP-2718 saying which kind it is.
///
/// Who sent it is not here. That is worked out from the signature, which
/// takes the elliptic curve, not these rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transaction {
    /// `0` for the kind with no byte in front, then `1` (EIP-2930),
    /// `2` (EIP-1559), `3` (blobs), `4` (EIP-7702).
    pub kind: u8,
    /// Who signed it, worked out from the signature: a transaction does not
    /// carry its sender, so this is the only way anyone knows. `None` when
    /// the signature does not recover, as a made-up one does not.
    pub from: Option<String>,
    pub chain_id: Option<u128>,
    pub nonce: u128,
    /// `None` when it makes a contract instead of calling one.
    pub to: Option<String>,
    /// Wei, as they came: in general they do not fit a machine word.
    pub value: Vec<u8>,
    pub gas: u128,
    /// What the older kinds pay for gas.
    pub gas_price: Option<u128>,
    /// What EIP-1559 and after pay: at most this much...
    pub max_fee: Option<u128>,
    /// ...and this much of it to whoever includes it.
    pub max_priority: Option<u128>,
    pub data: Vec<u8>,
    /// How many places it says beforehand that it will touch (EIP-2930).
    pub access_list: usize,
    /// How many blobs it carries (EIP-4844).
    pub blobs: usize,
}

/// Who signed it: the last three fields are the signature, and what was
/// signed is everything before them, put back the way the signer had it.
/// `None` when the signature does not recover, which a made-up one will not.
fn sender_of(kind: u8, rlp: &[u8], fields: &[Item<'_>], chain_id: Option<u128>) -> Option<String> {
    let at = fields.len().checked_sub(3)?;
    let (r, s) = (fields.get(at + 1)?.bytes()?, fields.get(at + 2)?.bytes()?);
    let v = fields.get(at)?.number()?;
    let parity = match kind {
        // EIP-155 folded the chain into `v`; before it, `v` was 27 or 28.
        0 if v >= 35 => ((v - 35) % 2) as u8,
        0 if v == 27 || v == 28 => (v - 27) as u8,
        0 => return None,
        _ if v <= 1 => v as u8,
        _ => return None,
    };
    let payload = list_payload(rlp)?;
    let signed = payload.get(..field_at(payload, at)?)?;
    let mut body = signed.to_vec();
    if kind == 0 {
        // What EIP-155 adds to what is signed, and nothing before it.
        if let Some(chain) = chain_id {
            body.extend(rlp_number(chain));
            body.extend([0x80, 0x80]);
        }
    }
    let mut message = Vec::with_capacity(body.len() + 11);
    if kind != 0 {
        message.push(kind);
    }
    message.extend(list_header(body.len()));
    message.extend(&body);
    sender(&Keccak256::digest(&message), r, s, parity)
}

impl Transaction {
    /// What a raw transaction holds. `None` when it is not one: the kind in
    /// front is unknown, the RLP does not read, or the fields are not those
    /// of that kind.
    pub fn decode(raw: &[u8]) -> Option<Self> {
        let (kind, rlp) = match *raw.first()? {
            // A list straight away: the kind from before EIP-2718.
            0xc0..=0xff => (0u8, raw),
            kind @ 1..=4 => (kind, raw.get(1..)?),
            _ => return None,
        };
        let (item, rest) = Item::read(rlp)?;
        if !rest.is_empty() {
            return None;
        }
        let fields = item.list()?;
        let number = |at: usize| fields.get(at).and_then(Item::number);
        let bytes = |at: usize| fields.get(at).and_then(Item::bytes);
        let count = |at: usize| fields.get(at).and_then(Item::list).map(<[Item]>::len);
        let mut tx = Self {
            kind,
            ..Self::default()
        };
        // The fields of each kind, in the order its own rules give them.
        let (to_at, value_at, data_at) = match kind {
            0 if fields.len() == 9 => {
                tx.nonce = number(0)?;
                tx.gas_price = Some(number(1)?);
                tx.gas = number(2)?;
                // EIP-155 keeps the chain in `v`: 35 or 36 above twice it.
                tx.chain_id = number(6).filter(|v| *v >= 35).map(|v| (v - 35) / 2);
                (3, 4, 5)
            }
            1 if fields.len() == 11 => {
                tx.chain_id = Some(number(0)?);
                tx.nonce = number(1)?;
                tx.gas_price = Some(number(2)?);
                tx.gas = number(3)?;
                tx.access_list = count(7)?;
                (4, 5, 6)
            }
            2 if fields.len() == 12 => {
                tx.chain_id = Some(number(0)?);
                tx.nonce = number(1)?;
                tx.max_priority = Some(number(2)?);
                tx.max_fee = Some(number(3)?);
                tx.gas = number(4)?;
                tx.access_list = count(8)?;
                (5, 6, 7)
            }
            3 if fields.len() == 14 => {
                tx.chain_id = Some(number(0)?);
                tx.nonce = number(1)?;
                tx.max_priority = Some(number(2)?);
                tx.max_fee = Some(number(3)?);
                tx.gas = number(4)?;
                tx.access_list = count(8)?;
                tx.blobs = count(10)?;
                (5, 6, 7)
            }
            4 if fields.len() == 13 => {
                tx.chain_id = Some(number(0)?);
                tx.nonce = number(1)?;
                tx.max_priority = Some(number(2)?);
                tx.max_fee = Some(number(3)?);
                tx.gas = number(4)?;
                tx.access_list = count(8)?;
                (5, 6, 7)
            }
            _ => return None,
        };
        let to = bytes(to_at)?;
        tx.to = (!to.is_empty()).then(|| address_text(to));
        tx.value = bytes(value_at)?.to_vec();
        tx.data = bytes(data_at)?.to_vec();
        tx.from = sender_of(kind, rlp, fields, tx.chain_id);
        Some(tx)
    }

    /// The function its `data` calls, when the four bytes name one of
    /// [`FUNCTIONS`].
    pub fn function(&self) -> Option<&'static str> {
        known_function(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    macro_rules! sample {
        ($name:literal) => {
            include_str!(concat!("../testdata/jsonrpc/", $name))
        };
    }

    /// A string out of a sample, by the names and numbers that lead to it.
    fn field(sample: &str, path: &[&str]) -> String {
        let mut at: Value = serde_json::from_str(sample).expect("the sample is JSON");
        for step in path {
            at = match step.parse::<usize>() {
                Ok(index) if at.is_array() => at[index].clone(),
                _ => at[*step].clone(),
            };
        }
        at.as_str()
            .unwrap_or_else(|| panic!("no string at {path:?}, found {at}"))
            .to_owned()
    }

    #[test]
    fn a_number_of_any_length_has_its_digits() {
        assert_eq!(digits(&[]), "0");
        assert_eq!(digits(&[0, 0]), "0");
        assert_eq!(digits(&[1, 0]), "256");
        // An allowance of no limit, which is what a wallet asks for: 78
        // digits, where a machine word holds 39.
        let all = digits(&[0xff; 32]);
        assert_eq!(all.len(), 78);
        assert!(
            all.starts_with("115792089237316195423570985008687907853269984665640564039457"),
            "{all}"
        );
        assert_eq!(scaled("1500000000000000000", 18), "1.5");
        assert_eq!(scaled("39192124", 6), "39.192124");
        assert_eq!(
            scaled("1000000", 6),
            "1",
            "the noughts after it are left off"
        );
        assert_eq!(scaled("0", 18), "0");
        assert_eq!(scaled("7", 18), "0.000000000000000007");
        assert_eq!(
            ether(&[0x50, 0x6a, 0x9d, 0x72, 0xec, 0x36, 0xd5, 0x6a]),
            "5.794616987480413546 ETH"
        );
        assert_eq!(gwei(170_239_778), "0.170239778 gwei");
        assert_eq!(quantity("0x192bbde"), Some(26_393_566));
        assert_eq!(quantity("0x0"), Some(0));
        assert_eq!(quantity("192bbde"), None, "a quantity comes with its 0x");
        assert_eq!(
            quantity(&format!("0x{}", "f".repeat(64))),
            None,
            "a uint256"
        );
        // A quantity's digits may be an odd number; a DATA field's may not.
        assert_eq!(quantity_bytes("0x0"), Some(vec![0]));
        assert_eq!(
            quantity_bytes("0x506a9d72ec36d56a").map(|b| b.len()),
            Some(8)
        );
        assert_eq!(bytes_of("0x0"), None);
        assert_eq!(
            digits(&quantity_bytes(&format!("0x{}", "f".repeat(64))).unwrap()).len(),
            78
        );
    }

    /// EIP-55: the capitals check the digits. Both of these are addresses
    /// anyone can look up, written as everyone writes them.
    #[test]
    fn an_address_is_written_with_the_capitals_that_check_it() {
        let usdc = bytes_of("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
        assert_eq!(
            address_text(&usdc),
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        );
        let vitalik = bytes_of("0xd8da6bf26964af9d7eed9e03e53415d37aa96045").unwrap();
        assert_eq!(
            address_text(&vitalik),
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
        );
    }

    /// The four bytes and the topics are worked out from the signature's
    /// text, never written by hand. These two come from traffic that was
    /// really captured, so they say the working out is right.
    #[test]
    fn the_four_bytes_are_worked_out_from_the_text() {
        let data = field(sample!("call_balanceof.req.json"), &["params", "0", "data"]);
        let calldata = bytes_of(&data).unwrap();
        assert_eq!(selector_of("balanceOf(address)"), calldata[..4]);
        assert_eq!(known_function(&calldata), Some("balanceOf(address)"));
        assert_eq!(name_of("balanceOf(address)"), "balanceOf");
        let transfer = field(sample!("revert.req.json"), &["params", "0", "data"]);
        assert_eq!(
            known_function(&bytes_of(&transfer).unwrap()),
            Some("transfer(address,uint256)")
        );
        // The first topic of a real ERC-20 Transfer, out of a receipt.
        let topic = field(
            sample!("receipt.res.json"),
            &["result", "logs", "0", "topics", "0"],
        );
        assert_eq!(
            bytes_of(&topic).unwrap(),
            topic_of("Transfer(address,address,uint256)"),
        );
        assert_eq!(
            known_event(&bytes_of(&topic).unwrap()),
            Some("Transfer(address,address,uint256)")
        );
        assert_eq!(known_function(b"\x00\x01\x02\x03"), None);
    }

    /// The raw transaction and the same transaction as the node reads it were
    /// captured together, so every field can be checked against it.
    #[test]
    fn a_raw_transaction_reads_as_the_node_reads_it() {
        let raw = field(sample!("send_raw.req.json"), &["params", "0"]);
        let tx = Transaction::decode(&bytes_of(&raw).unwrap()).expect("a transaction");
        let node = |name: &str| field(sample!("tx.res.json"), &["result", name]);
        let number = |name: &str| quantity(&node(name)).expect("a quantity");
        assert_eq!(u128::from(tx.kind), number("type"));
        assert_eq!(tx.kind, 2, "EIP-1559");
        assert_eq!(tx.chain_id, Some(number("chainId")));
        assert_eq!(tx.nonce, number("nonce"));
        assert_eq!(tx.gas, number("gas"));
        assert_eq!(tx.max_fee, Some(number("maxFeePerGas")));
        assert_eq!(tx.max_priority, Some(number("maxPriorityFeePerGas")));
        assert_eq!(tx.gas_price, None, "that kind does not have one");
        assert_eq!(
            tx.to.as_deref(),
            Some(address_text(&bytes_of(&node("to")).unwrap()).as_str())
        );
        assert_eq!(
            digits(&tx.value),
            digits(&quantity_bytes(&node("value")).unwrap())
        );
        assert_eq!(hex_text(&tx.data), node("input"));
        assert_eq!(tx.data.len(), 772);
        assert_eq!(tx.access_list, 0);
        assert_eq!(tx.blobs, 0);
        // A transaction does not carry who sent it: this is worked out from
        // the signature, and the node says the same.
        assert_eq!(
            tx.from.as_deref(),
            Some(address_text(&bytes_of(&node("from")).unwrap()).as_str())
        );
        // What it calls is not one every chain has: an ABI would name it.
        assert_eq!(tx.function(), None);
        assert_eq!(hex_text(&tx.data[..4]), "0x09c5eabe");
    }

    /// The kind from before EIP-2718 signs something else: the chain goes in
    /// what is signed, not in a byte in front. Also real, also checked
    /// against the node.
    #[test]
    fn a_transaction_of_the_older_kind_reads_too() {
        let raw = field(sample!("send_raw_legacy.req.json"), &["params", "0"]);
        let tx = Transaction::decode(&bytes_of(&raw).unwrap()).expect("a transaction");
        let node = |name: &str| field(sample!("tx_legacy.res.json"), &["result", name]);
        let number = |name: &str| quantity(&node(name)).expect("a quantity");
        assert_eq!(tx.kind, 0);
        assert_eq!(tx.nonce, number("nonce"));
        assert_eq!(tx.gas_price, Some(number("gasPrice")));
        assert_eq!(tx.max_fee, None, "that kind has no such thing");
        // `v` is 35 or 36 above twice the chain, which is where it is kept.
        assert_eq!(tx.chain_id, Some(1));
        assert!(number("v") >= 35, "EIP-155, so the chain is signed over");
        assert_eq!(
            tx.from.as_deref(),
            Some(address_text(&bytes_of(&node("from")).unwrap()).as_str()),
            "recovered over what EIP-155 says is signed"
        );
    }

    #[test]
    fn a_transaction_that_is_not_one_is_not_read() {
        assert_eq!(Transaction::decode(b""), None);
        assert_eq!(Transaction::decode(&[0x05, 0xc0]), None, "an unknown kind");
        assert_eq!(Transaction::decode(&[0x02, 0xc0]), None, "no fields");
        assert_eq!(Transaction::decode(b"\xc3\x01\x02\x03"), None, "too few");
    }

    #[test]
    fn a_reverted_call_says_why_in_its_data() {
        let data = field(sample!("revert.res.json"), &["error", "data"]);
        let revert = Revert::of(&bytes_of(&data).unwrap()).expect("a reason");
        assert_eq!(
            revert,
            Revert::Reason("ERC20: transfer amount exceeds balance".to_owned())
        );
        assert_eq!(revert.text(), "ERC20: transfer amount exceeds balance");
        // What the compiler itself raises.
        let mut panic = selector_of("Panic(uint256)").to_vec();
        panic.extend(std::iter::repeat_n(0, 31));
        panic.push(0x11);
        let overflowed = Revert::of(&panic).expect("a panic");
        assert_eq!(overflowed, Revert::Panic(0x11));
        assert_eq!(
            overflowed.text(),
            "a sum or a product overflowed (panic 0x11)"
        );
        // A custom error: four bytes, and nothing yet to name them.
        let custom = Revert::of(b"\x12\x34\x56\x78").expect("four bytes are enough");
        assert_eq!(custom, Revert::Custom([0x12, 0x34, 0x56, 0x78]));
        assert_eq!(custom.text(), "0x12345678");
        assert_eq!(Revert::of(b"\x01"), None, "a call out of gas sends none");
        // The selector of a reason, with nothing readable after it: it still
        // reverted, and saying so beats saying nothing.
        let cut = selector_of("Error(string)").to_vec();
        assert_eq!(
            Revert::of(&cut),
            Some(Revert::Custom(selector_of("Error(string)")))
        );
    }

    #[test]
    fn the_arguments_of_a_call_are_read_by_its_types() {
        let data = field(sample!("call_balanceof.req.json"), &["params", "0", "data"]);
        let args = arguments("balanceOf(address)", &bytes_of(&data).unwrap()).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].kind, "address");
        assert_eq!(args[0].text, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        // The call that reverted asked to send more than there is: every bit
        // set, which is 78 digits.
        let transfer = field(sample!("revert.req.json"), &["params", "0", "data"]);
        let args = arguments("transfer(address,uint256)", &bytes_of(&transfer).unwrap()).unwrap();
        assert_eq!(args[0].text, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        assert_eq!(args[1].kind, "uint256");
        assert_eq!(args[1].text.len(), 78);
        assert_eq!(
            arguments("totalSupply()", &selector_of("totalSupply()")),
            Some(Vec::new())
        );
        assert_eq!(arguments("notasignature", b""), None);
    }

    /// A real `aggregate3`, captured from mainnet: an array of tuples, each
    /// holding calldata of its own. This is what the hand-written decoder
    /// could not read at all.
    #[test]
    fn a_multicall_reads_down_to_the_calls_inside_it() {
        let data = field(sample!("multicall.req.json"), &["params", "0", "data"]);
        let calldata = bytes_of(&data).unwrap();
        assert_eq!(calldata.len(), 4772);
        let signature = known_function(&calldata).expect("a call every chain shares");
        assert_eq!(signature, "aggregate3((address,bool,bytes)[])");
        let args = arguments(signature, &calldata).expect("it reads");
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].kind, "(address,bool,bytes)[]");
        // Each call inside: where it goes, whether it may fail, and what it
        // calls there.
        // One call: where it goes, that it may fail, and the calldata it
        // carries there, which is most of the 4,772 bytes.
        assert_eq!(
            args[0].text,
            "[(0xE37A2663Cc9c119BA41572C40fAb0bbb23a5402f, true, 0x0a883a99…, 4516 bytes)]"
        );
        // An ABI of the user's reaches in through the closure.
        let named = arguments_named(signature, &calldata, &|data| {
            (data.len() >= 4).then(|| format!("seen({})", hex_text(&data[..4])))
        })
        .unwrap();
        assert!(named[0].text.contains("seen(0x"), "{}", named[0].text);
    }

    #[test]
    fn the_types_of_an_argument_each_read_their_own_way() {
        let word = |hex: &str| {
            let mut out = selector_of("f()").to_vec();
            out.extend(bytes_of(&format!("0x{hex:0>64}")).unwrap());
            out
        };
        let one = |kind: &str, hex: &str| {
            arguments(&format!("f({kind})"), &word(hex)).unwrap()[0]
                .text
                .clone()
        };
        assert_eq!(one("bool", "01"), "true");
        assert_eq!(one("bool", "00"), "false");
        assert_eq!(one("uint8", "ff"), "255");
        assert_eq!(one("bytes32", "ab"), format!("0x{:0>64}", "ab"));
        // Two's complement: -1 is every bit set.
        assert_eq!(one("int256", &"f".repeat(64)), "-1");
        assert_eq!(one("int256", "02"), "2");
        // A list reads as its items, which the words after the count hold.
        let mut array = selector_of("f(uint256[])").to_vec();
        array.extend(bytes_of(&format!("0x{:0>64x}", 32)).unwrap());
        array.extend(bytes_of(&format!("0x{:0>64x}", 3)).unwrap());
        for item in [7u64, 8, 9] {
            array.extend(bytes_of(&format!("0x{item:0>64x}")).unwrap());
        }
        let args = arguments("f(uint256[])", &array).unwrap();
        assert_eq!(args[0].text, "[7, 8, 9]");
        // A list that says it holds more than it does does not read at all,
        // where the hand-written one took the count on trust.
        array.truncate(array.len() - 32);
        assert_eq!(arguments("f(uint256[])", &array), None);
        assert_eq!(arguments("f(nosuchtype)", &word("01")), None);
    }
}
