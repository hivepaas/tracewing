//! What a chain writes, read: an address and whether its capitals check out,
//! an event by its topic, a call by its four bytes.
//!
//! The tables and the keccak live here rather than in the store, so that a
//! value can be read without a database behind it. `tracewing_store::web3`
//! takes them from here, so nothing that used them had to change.

use crate::{Confidence, Input, Reading, Shown};
use sha3::{Digest, Keccak256};
use std::{collections::HashMap, sync::OnceLock};
use tracewing_i18n::tr;

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
pub const FUNCTIONS: &[(&str, &str)] = &[
    // ERC-20, with what each answers: without that, a number a call came
    // back with is only a word of hex.
    ("transfer(address,uint256)", "(bool)"),
    ("transferFrom(address,address,uint256)", "(bool)"),
    ("approve(address,uint256)", "(bool)"),
    ("balanceOf(address)", "(uint256)"),
    ("allowance(address,address)", "(uint256)"),
    ("totalSupply()", "(uint256)"),
    ("decimals()", "(uint8)"),
    ("symbol()", "(string)"),
    ("name()", "(string)"),
    (
        "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)",
        "()",
    ),
    // ERC-721 and ERC-1155.
    ("ownerOf(uint256)", "(address)"),
    ("safeTransferFrom(address,address,uint256)", "()"),
    ("safeTransferFrom(address,address,uint256,bytes)", "()"),
    ("setApprovalForAll(address,bool)", "()"),
    ("isApprovedForAll(address,address)", "(bool)"),
    ("tokenURI(uint256)", "(string)"),
    ("uri(uint256)", "(string)"),
    ("balanceOfBatch(address[],uint256[])", "(uint256[])"),
    // Wrapped ether.
    ("deposit()", "()"),
    ("withdraw(uint256)", "()"),
    // What a wallet asks a node for, through a contract.
    ("multicall(bytes[])", "(bytes[])"),
    ("aggregate3((address,bool,bytes)[])", "((bool,bytes)[])"),
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
    Some(known(data)?.0)
}

/// The types that call answers with, as `(uint256)`, for reading what came
/// back. `()` for one that answers with nothing.
pub fn known_returns(data: &[u8]) -> Option<&'static str> {
    Some(known(data)?.1)
}

fn known(data: &[u8]) -> Option<(&'static str, &'static str)> {
    static TABLE: OnceLock<HashMap<[u8; 4], (&'static str, &'static str)>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        FUNCTIONS
            .iter()
            .map(|(signature, returns)| (selector_of(signature), (*signature, *returns)))
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

/// How a chain writes nothing at all: the address a token is minted from and
/// burned to.
const NOBODY: &str = "0x0000000000000000000000000000000000000000";
/// What a contract writes where it means the chain's own coin rather than a
/// token, since that has no address of its own.
const NATIVE: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

/// Twenty bytes of hex: an address, and whether the capitals in it check out.
///
/// A mistyped address is the costliest value in this whole crate, and EIP-55
/// is the only thing standing between a reader and one: change a digit and
/// the capitals no longer fit. Nobody can see that by eye, which is the
/// point of saying it here.
pub fn address(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    if digits.len() != 40 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let bytes = hex_bytes(digits)?;
    let checksummed = address_text(&bytes);
    let mixed = digits.chars().any(|c| c.is_ascii_uppercase())
        && digits.chars().any(|c| c.is_ascii_lowercase());
    let held = checksummed[2..] == *digits;
    let lower = digits.to_ascii_lowercase();
    let mut fields = vec![(tr!("As EIP-55 writes it"), checksummed.clone())];
    fields.push((
        tr!("Checksum"),
        if !mixed {
            // All one case carries no checksum at all: there is nothing to
            // check, which is not the same as checking out.
            tr!("none in it, so nothing to check")
        } else if held {
            tr!("holds")
        } else {
            tr!("does not hold: a digit is wrong")
        },
    ));
    if lower == NOBODY[2..] {
        fields.push((tr!("Known as"), tr!("nobody: minted from, or burned to")));
    } else if lower == NATIVE[2..] {
        fields.push((tr!("Known as"), tr!("the chain's own coin, not a token")));
    }
    // A field that says what it holds leaves nothing to guess; capitals that
    // check out are an address saying so itself.
    let confidence = if input.field_says(SAYS_ADDRESS) || (mixed && held) {
        Confidence::Certain
    } else if mixed && !held {
        // Mixed case that does not check out is either a typo or not an
        // address at all, and either way worth saying out loud.
        Confidence::Likely
    } else {
        Confidence::Likely
    };
    Some(Reading::new(
        "address",
        tr!("An address"),
        confidence,
        Shown::Fields(fields),
    ))
}

/// The names a field goes by when it holds an address.
const SAYS_ADDRESS: &[&str] = &[
    "address",
    "from",
    "to",
    "owner",
    "spender",
    "token",
    "contract",
    "recipient",
    "sender",
    "account",
    "wallet",
    "operator",
    "beneficiary",
];

/// A log's first topic: the event it stands for, where that is one every
/// chain has. `Transfer(address,address,uint256)` rather than
/// `0xddf252ad…`, which is the same thing and says nothing.
pub fn topic(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    let digits = text.strip_prefix("0x")?;
    if digits.len() != 64 {
        return None;
    }
    let bytes = hex_bytes(digits)?;
    let event = known_event(&bytes)?;
    Some(Reading::new(
        "topic",
        tr!("An event's topic"),
        Confidence::Certain,
        Shown::Fields(vec![
            (tr!("Event"), event.to_owned()),
            (tr!("Name"), name_of(event).to_owned()),
        ]),
    ))
}

/// The four bytes a call starts with: the function they name, where it is
/// one every chain has.
pub fn selector(input: &Input) -> Option<Reading> {
    let text = input.text.trim().trim_matches('"');
    let digits = text.strip_prefix("0x")?;
    // The four bytes alone, or the whole calldata they start.
    if digits.len() < 8
        || !digits.len().is_multiple_of(2)
        || !digits.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let bytes = hex_bytes(&digits[..8])?;
    let function = known_function(&bytes)?;
    let mut fields = vec![
        (tr!("Calls"), function.to_owned()),
        (tr!("Name"), name_of(function).to_owned()),
    ];
    if let Some(returns) = known_returns(&bytes)
        && returns != "()"
    {
        fields.push((tr!("Answers with"), returns.to_owned()));
    }
    Some(Reading::new(
        "selector",
        tr!("A call, by its four bytes"),
        Confidence::Certain,
        Shown::Fields(fields),
    ))
}

fn hex_bytes(digits: &str) -> Option<Vec<u8>> {
    digits
        .len()
        .is_multiple_of(2)
        .then(|| {
            (0..digits.len() / 2)
                .map(|at| u8::from_str_radix(&digits[at * 2..at * 2 + 2], 16).ok())
                .collect::<Option<Vec<u8>>>()
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{by_id, read, read_under};

    /// The address of USDC on mainnet, as a node writes it.
    const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

    fn fields_of(found: &[Reading], id: &str) -> Vec<(String, String)> {
        match by_id(found, id).map(|reading| &reading.shown) {
            Some(Shown::Fields(fields)) => fields.clone(),
            other => panic!("no {id}: {other:?}"),
        }
    }

    #[test]
    fn an_address_says_whether_its_capitals_check_out() {
        let found = fields_of(&read(USDC), "address");
        assert_eq!(found[0].1, USDC, "written as EIP-55 writes it");
        assert_eq!(found[1].1, "holds");

        // One digit changed: the capitals no longer fit, which is the whole
        // point of them and the thing a reader cannot see by eye.
        let typo = USDC.replace("c6218b36", "c6218b37");
        let found = fields_of(&read(&typo), "address");
        assert_eq!(found[1].1, "does not hold: a digit is wrong");

        // All one case carries no checksum at all, which is not the same as
        // one that checks out.
        let flat = USDC.to_ascii_lowercase();
        let found = fields_of(&read(&flat), "address");
        assert_eq!(found[0].1, USDC, "and says how it would be written");
        assert_eq!(found[1].1, "none in it, so nothing to check");
    }

    #[test]
    fn the_addresses_that_stand_for_nobody_are_named() {
        let found = fields_of(&read(NOBODY), "address");
        assert!(
            found.iter().any(|(_, said)| said.contains("burned")),
            "{found:?}"
        );
        let found = fields_of(&read(NATIVE), "address");
        assert!(
            found.iter().any(|(_, said)| said.contains("own coin")),
            "{found:?}"
        );
    }

    /// Twenty bytes of hex are the size of a RIPEMD-160, which is how an
    /// address came to be read as one. Behind an `0x` it is an address.
    #[test]
    fn an_address_is_not_offered_as_a_hash() {
        let found = read(USDC);
        assert!(by_id(&found, "hash").is_none(), "{found:?}");
        // A hash written as a hash is still a hash.
        let bare = "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
        assert!(by_id(&read(bare), "hash").is_some());
    }

    #[test]
    fn a_topic_is_the_event_it_stands_for() {
        let transfer = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        let found = fields_of(&read(transfer), "topic");
        assert_eq!(found[0].1, "Transfer(address,address,uint256)");
        assert_eq!(found[1].1, "Transfer");
        // One nobody has a name for says nothing rather than guessing.
        let unknown = "0x1111111111111111111111111111111111111111111111111111111111111111";
        assert!(by_id(&read(unknown), "topic").is_none());
    }

    #[test]
    fn four_bytes_name_the_call_they_start() {
        // `transfer(address,uint256)` and its arguments.
        let data = concat!(
            "0xa9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045",
            "00000000000000000000000000000000000000000000000000000000000f4240"
        );
        let found = fields_of(&read_under("data", data), "selector");
        assert_eq!(found[0].1, "transfer(address,uint256)");
        assert_eq!(found[1].1, "transfer");
        assert_eq!(found[2].1, "(bool)", "and what it answers with");
        // The four bytes alone are enough.
        assert!(by_id(&read("0x70a08231"), "selector").is_some());
        assert!(by_id(&read("0xdeadbeef"), "selector").is_none());
    }
}
