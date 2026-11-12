//! What a Bitcoin call holds: amounts in satoshis, which are not what anyone
//! writes; and a transaction, which `sendrawtransaction` carries as one hex
//! string holding every input, output and signature.
//!
//! On top of [`crate::jsonrpc`], as [`crate::web3`] and [`crate::solana`]
//! are. Bitcoin Core's methods are lower case where Solana's are camelCase
//! and Ethereum's start `eth_`, so the three tell themselves apart.
//!
//! Nothing here asks the network.

use sha2::{Digest, Sha256};

/// How many satoshis make one bitcoin.
pub const DECIMALS: usize = 8;

/// The bytes of a hex string as Bitcoin's RPC writes it: no `0x` in front of
/// it, which is what tells it from Ethereum's and why
/// [`crate::web3::bytes_of`] will not read it.
pub fn bytes_of(hex: &str) -> Option<Vec<u8>> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len() / 2)
        .map(|at| u8::from_str_radix(hex.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect()
}

/// `0.00661128 BTC`, from the satoshis as they came.
pub fn btc(satoshis: u64) -> String {
    format!(
        "{} BTC",
        crate::web3::scaled(&satoshis.to_string(), DECIMALS)
    )
}

/// Twice over, as Bitcoin hashes everything.
fn sha256d(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(Sha256::digest(bytes)).into()
}

/// Hex, the way Bitcoin writes a hash: the bytes backwards.
fn reversed_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes.iter().rev() {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// An address as the oldest kind is written: the bytes, a byte in front
/// saying what kind, and four bytes of their own hash at the end so that a
/// mistyped one does not read.
fn base58check(prefix: u8, payload: &[u8]) -> String {
    let mut bytes = vec![prefix];
    bytes.extend(payload);
    bytes.extend(&sha256d(&bytes)[..4]);
    // The same alphabet Solana uses, which Bitcoin chose first.
    crate::solana::base58(&bytes)
}

const BECH32: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
/// What a bech32 checksum comes to, and what bech32m's comes to instead:
/// taproot changed it so that an address of one kind cannot be read as the
/// other.
const BECH32_SUM: u32 = 1;
const BECH32M_SUM: u32 = 0x2bc830a3;

fn polymod(values: &[u8]) -> u32 {
    const GENERATOR: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut checksum = 1u32;
    for value in values {
        let top = checksum >> 25;
        checksum = ((checksum & 0x1ffffff) << 5) ^ u32::from(*value);
        for (at, bit) in GENERATOR.iter().enumerate() {
            if (top >> at) & 1 == 1 {
                checksum ^= bit;
            }
        }
    }
    checksum
}

/// Eight bits a byte into five bits a character.
fn to_five(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() * 8 / 5 + 1);
    let (mut acc, mut bits) = (0u32, 0u32);
    for byte in bytes {
        acc = (acc << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 31) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 31) as u8);
    }
    out
}

/// A witness address: `bc1q…` for the first version, `bc1p…` for taproot,
/// which checksums by the other constant.
fn bech32(version: u8, program: &[u8]) -> String {
    let mut values = vec![version];
    values.extend(to_five(program));
    // `bc` as the rules expand it, before and after the one separator.
    let mut checked = vec![3u8, 3, 0, 2, 3];
    checked.extend(&values);
    checked.extend([0, 0, 0, 0, 0, 0]);
    let sum = polymod(&checked)
        ^ match version {
            0 => BECH32_SUM,
            _ => BECH32M_SUM,
        };
    let mut out = String::from("bc1");
    for value in values {
        out.push(char::from(BECH32[usize::from(value)]));
    }
    for at in (0..6).rev() {
        out.push(char::from(BECH32[((sum >> (5 * at)) & 31) as usize]));
    }
    out
}

/// What an output pays to, by the shape of the script that locks it. The
/// names are the ones a node and the explorers use.
fn locked_by(script: &[u8]) -> (&'static str, Option<String>) {
    match script {
        // OP_DUP OP_HASH160 <20> OP_EQUALVERIFY OP_CHECKSIG
        [0x76, 0xa9, 0x14, hash @ .., 0x88, 0xac] if hash.len() == 20 => {
            ("p2pkh", Some(base58check(0x00, hash)))
        }
        // OP_HASH160 <20> OP_EQUAL
        [0xa9, 0x14, hash @ .., 0x87] if hash.len() == 20 => {
            ("p2sh", Some(base58check(0x05, hash)))
        }
        [0x00, 0x14, program @ ..] if program.len() == 20 => {
            ("v0_p2wpkh", Some(bech32(0, program)))
        }
        [0x00, 0x20, program @ ..] if program.len() == 32 => ("v0_p2wsh", Some(bech32(0, program))),
        // OP_1 <32>
        [0x51, 0x20, program @ ..] if program.len() == 32 => ("v1_p2tr", Some(bech32(1, program))),
        [0x6a, ..] => ("op_return", None),
        _ => ("unknown", None),
    }
}

/// A number as Bitcoin writes one in front of a list: one byte, or a marker
/// and two, four or eight more.
fn varint(input: &[u8]) -> Option<(u64, &[u8])> {
    let (&first, rest) = input.split_first()?;
    let len = match first {
        0xfd => 2,
        0xfe => 4,
        0xff => 8,
        _ => return Some((u64::from(first), rest)),
    };
    let (bytes, rest) = rest.split_at_checked(len)?;
    let mut value = 0u64;
    for byte in bytes.iter().rev() {
        value = (value << 8) | u64::from(*byte);
    }
    Some((value, rest))
}

fn u32_at(input: &[u8]) -> Option<(u32, &[u8])> {
    let (bytes, rest) = input.split_at_checked(4)?;
    Some((u32::from_le_bytes(bytes.try_into().ok()?), rest))
}

/// Where a coin being spent came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// The transaction it was paid by, written as Bitcoin writes a hash.
    pub txid: String,
    /// Which of that transaction's outputs.
    pub vout: u32,
    pub sequence: u32,
}

/// Where a coin is going.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub satoshis: u64,
    /// `p2pkh`, `v0_p2wpkh`, `v1_p2tr`, `op_return`, and so on.
    pub kind: &'static str,
    /// `None` for a script that pays nobody, as `op_return` does.
    pub address: Option<String>,
}

/// A signed transaction, as `sendrawtransaction` carries it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transaction {
    /// What it is called everywhere else, worked out from its own bytes.
    pub txid: String,
    pub version: u32,
    /// Carrying its signatures apart from the rest, as everything does now.
    pub segwit: bool,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub locktime: u32,
}

impl Transaction {
    /// What a raw transaction holds. `None` when the bytes are not one.
    pub fn decode(raw: &[u8]) -> Option<Self> {
        let (version, rest) = u32_at(raw)?;
        let mut tx = Self {
            version,
            ..Self::default()
        };
        // A nothing where the inputs should start, then a flag: the mark of
        // a transaction keeping its signatures apart.
        let rest = match rest {
            [0x00, 0x01, after @ ..] => {
                tx.segwit = true;
                after
            }
            _ => rest,
        };
        // Where what the txid is worked out over begins: the counts, the
        // inputs and the outputs, with the mark, the flag and the signatures
        // left out.
        let named_from = rest;
        let (count, mut rest) = varint(rest)?;
        for _ in 0..count {
            let (txid, after) = rest.split_at_checked(32)?;
            let (vout, after) = u32_at(after)?;
            let (len, after) = varint(after)?;
            let after = after.get(usize::try_from(len).ok()?..)?;
            let (sequence, after) = u32_at(after)?;
            tx.inputs.push(Input {
                txid: reversed_hex(txid),
                vout,
                sequence,
            });
            rest = after;
        }
        let (count, mut after_inputs) = varint(rest)?;
        for _ in 0..count {
            let (value, after) = after_inputs.split_at_checked(8)?;
            let satoshis = u64::from_le_bytes(value.try_into().ok()?);
            let (len, after) = varint(after)?;
            let (script, after) = after.split_at_checked(usize::try_from(len).ok()?)?;
            let (kind, address) = locked_by(script);
            tx.outputs.push(Output {
                satoshis,
                kind,
                address,
            });
            after_inputs = after;
        }
        let named_len = named_from.len() - after_inputs.len();
        // The signatures, which are not what the transaction is named by.
        let rest = match tx.segwit {
            false => after_inputs,
            true => {
                let mut rest = after_inputs;
                for _ in 0..tx.inputs.len() {
                    let (items, mut after) = varint(rest)?;
                    for _ in 0..items {
                        let (len, next) = varint(after)?;
                        after = next.get(usize::try_from(len).ok()?..)?;
                    }
                    rest = after;
                }
                rest
            }
        };
        let (locktime, rest) = u32_at(rest)?;
        if !rest.is_empty() {
            return None;
        }
        tx.locktime = locktime;
        // Named by its own bytes, with the signatures left out.
        let mut named = Vec::with_capacity(raw.len());
        named.extend(&raw[..4]);
        named.extend(named_from.get(..named_len)?);
        named.extend(&locktime.to_le_bytes());
        tx.txid = reversed_hex(&sha256d(&named));
        Some(tx)
    }

    /// What it pays out in all.
    pub fn paid(&self) -> u64 {
        self.outputs.iter().map(|out| out.satoshis).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    macro_rules! sample {
        ($name:literal) => {
            include_str!(concat!("../testdata/bitcoin/", $name))
        };
    }

    fn hex_bytes(hex: &str) -> Vec<u8> {
        (0..hex.len() / 2)
            .map(|at| u8::from_str_radix(&hex[at * 2..at * 2 + 2], 16).expect("hex"))
            .collect()
    }

    /// The raw hex a client sends, and the same transaction as the explorer
    /// reads it: every field is checked against the second.
    fn both(raw: &str, parsed: &str) -> (Transaction, Value) {
        let asked: Value = serde_json::from_str(raw).expect("JSON");
        let hex = asked["params"][0].as_str().expect("the hex");
        let tx = Transaction::decode(&hex_bytes(hex)).expect("a transaction");
        (tx, serde_json::from_str(parsed).expect("JSON"))
    }

    fn check(raw: &str, parsed: &str) -> Transaction {
        let (tx, said) = both(raw, parsed);
        // Worked out from the bytes, not taken from anywhere.
        assert_eq!(tx.txid, said["txid"].as_str().unwrap(), "the txid");
        assert_eq!(u64::from(tx.version), said["version"].as_u64().unwrap());
        assert_eq!(u64::from(tx.locktime), said["locktime"].as_u64().unwrap());
        let ins = said["vin"].as_array().unwrap();
        assert_eq!(tx.inputs.len(), ins.len());
        for (read, said) in tx.inputs.iter().zip(ins) {
            assert_eq!(read.txid, said["txid"].as_str().unwrap());
            assert_eq!(u64::from(read.vout), said["vout"].as_u64().unwrap());
        }
        let outs = said["vout"].as_array().unwrap();
        assert_eq!(tx.outputs.len(), outs.len());
        for (read, said) in tx.outputs.iter().zip(outs) {
            assert_eq!(read.satoshis, said["value"].as_u64().unwrap());
            assert_eq!(read.kind, said["scriptpubkey_type"].as_str().unwrap());
            assert_eq!(
                read.address.as_deref(),
                said["scriptpubkey_address"].as_str(),
                "the address it pays"
            );
        }
        tx
    }

    #[test]
    fn satoshis_read_as_bitcoin() {
        assert_eq!(btc(100_000_000), "1 BTC");
        assert_eq!(btc(661_128), "0.00661128 BTC");
        assert_eq!(btc(1), "0.00000001 BTC");
        assert_eq!(btc(0), "0 BTC");
    }

    /// Two bech32 outputs, which is what most of a block looks like now.
    #[test]
    fn a_transaction_reads_as_the_chain_reads_it() {
        let tx = check(sample!("segwit.req.json"), sample!("segwit.tx.json"));
        let (_, said) = both(sample!("segwit.req.json"), sample!("segwit.tx.json"));
        assert!(tx.segwit, "its signatures are kept apart");
        assert_eq!(tx.outputs.len(), 2);
        assert!(
            tx.outputs.iter().all(|out| out
                .address
                .as_deref()
                .is_some_and(|a| a.starts_with("bc1q"))),
            "{:?}",
            tx.outputs
        );
        // Against the chain's own numbers rather than any worked out here.
        let total: u64 = said["vout"]
            .as_array()
            .unwrap()
            .iter()
            .map(|out| out["value"].as_u64().unwrap())
            .sum();
        assert_eq!(tx.paid(), total);
    }

    /// Every way an output can be paid to, each against a real one: base58
    /// for the two older kinds, bech32 for witness, and bech32m for taproot,
    /// which checksums by a different constant and would come out wrong if
    /// that were missed.
    #[test]
    fn every_kind_of_address_is_written_the_way_the_chain_writes_it() {
        let legacy = check(
            sample!("legacy_and_script.req.json"),
            sample!("legacy_and_script.tx.json"),
        );
        assert_eq!(legacy.outputs[0].kind, "p2pkh");
        assert!(
            legacy.outputs[0]
                .address
                .as_deref()
                .unwrap()
                .starts_with('1')
        );
        assert_eq!(legacy.outputs[1].kind, "v0_p2wsh");

        let script = check(sample!("p2sh.req.json"), sample!("p2sh.tx.json"));
        assert_eq!(script.outputs[0].kind, "p2sh");
        assert!(
            script.outputs[0]
                .address
                .as_deref()
                .unwrap()
                .starts_with('3')
        );

        let taproot = check(sample!("taproot.req.json"), sample!("taproot.tx.json"));
        let paid = taproot
            .outputs
            .iter()
            .find(|out| out.kind == "v1_p2tr")
            .expect("a taproot output");
        assert!(
            paid.address.as_deref().unwrap().starts_with("bc1p"),
            "{paid:?}"
        );
    }

    #[test]
    fn bytes_that_are_not_a_transaction_are_not_read() {
        assert_eq!(Transaction::decode(b""), None);
        assert_eq!(
            Transaction::decode(&[1, 0, 0, 0]),
            None,
            "no inputs promised"
        );
        // A trailing byte that belongs to nothing.
        let raw: Value = serde_json::from_str(sample!("segwit.req.json")).unwrap();
        let mut bytes = hex_bytes(raw["params"][0].as_str().unwrap());
        bytes.push(0);
        assert_eq!(Transaction::decode(&bytes), None);
    }
}
