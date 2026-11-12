//! What a Solana call holds: addresses, which are base58 and not hex;
//! lamports, which are what SOL is counted in; and a transaction, which
//! `sendTransaction` carries as one base64 string of a few hundred bytes.
//!
//! On top of [`crate::jsonrpc`], as [`crate::web3`] is. Solana's methods are
//! camelCase where Bitcoin's are lower case and Ethereum's start `eth_`, so
//! the three do not have to be told apart by anything else.
//!
//! Nothing here asks the network.

/// The alphabet Bitcoin chose and Solana kept: no `0`, `O`, `I` or `l`, so
/// that a written-down address cannot be read two ways.
const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// How many lamports make one SOL.
pub const DECIMALS: usize = 9;

/// Bytes as base58, which is how Solana writes an address, a signature and a
/// blockhash alike.
pub fn base58(bytes: &[u8]) -> String {
    let zeros = bytes.iter().take_while(|byte| **byte == 0).count();
    let mut digits: Vec<u8> = Vec::with_capacity(bytes.len() * 2);
    for byte in &bytes[zeros..] {
        let mut carry = usize::from(*byte);
        for digit in digits.iter_mut() {
            carry += usize::from(*digit) << 8;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut out = String::with_capacity(zeros + digits.len());
    // A leading nothing is a `1`, one for each, which is why an address of
    // the system program is a row of them.
    out.extend(std::iter::repeat_n('1', zeros));
    for digit in digits.iter().rev() {
        out.push(char::from(ALPHABET[usize::from(*digit)]));
    }
    out
}

/// The bytes a base58 string holds. `None` when it is not base58.
pub fn from_base58(text: &str) -> Option<Vec<u8>> {
    let zeros = text.chars().take_while(|c| *c == '1').count();
    let mut bytes: Vec<u8> = Vec::with_capacity(text.len());
    for c in text.chars().skip(zeros) {
        let mut carry = ALPHABET.iter().position(|a| char::from(*a) == c)?;
        for byte in bytes.iter_mut() {
            carry += usize::from(*byte) * 58;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    let mut out = vec![0u8; zeros];
    out.extend(bytes.iter().rev());
    Some(out)
}

/// `1.5 SOL`, from the lamports as they came.
pub fn sol(lamports: u64) -> String {
    format!(
        "{} SOL",
        crate::web3::scaled(&lamports.to_string(), DECIMALS)
    )
}

/// The programs nearly every transaction touches, by the address each lives
/// at. Checked against the chain: each one is there and runnable.
pub const PROGRAMS: &[(&str, &str)] = &[
    ("11111111111111111111111111111111", "System"),
    ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "Token"),
    ("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", "Token-2022"),
    (
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        "Associated Token Account",
    ),
    (
        "ComputeBudget111111111111111111111111111111",
        "Compute Budget",
    ),
    ("Vote111111111111111111111111111111111111111", "Vote"),
    ("Stake11111111111111111111111111111111111111", "Stake"),
];

/// What the program at that address is called, if it is one of [`PROGRAMS`].
pub fn program_name(address: &str) -> Option<&'static str> {
    PROGRAMS
        .iter()
        .find(|(at, _)| *at == address)
        .map(|(_, name)| *name)
}

/// A count, as Solana writes one in front of a list: seven bits a byte, with
/// the top bit saying another follows.
fn shortvec(input: &[u8]) -> Option<(usize, &[u8])> {
    let mut value = 0usize;
    for (at, byte) in input.iter().enumerate().take(3) {
        value |= usize::from(byte & 0x7f) << (at * 7);
        if byte & 0x80 == 0 {
            return Some((value, input.get(at + 1..)?));
        }
    }
    None
}

/// `count` items of `size` bytes, and what is left after them.
fn chunks(input: &[u8], count: usize, size: usize) -> Option<(Vec<&[u8]>, &[u8])> {
    let (taken, rest) = input.split_at_checked(count.checked_mul(size)?)?;
    Some((taken.chunks(size).collect(), rest))
}

/// One thing a transaction asks a program to do.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Instruction {
    /// Where the program lives, as base58.
    pub program: String,
    /// How many of the transaction's accounts it is given.
    pub accounts: usize,
    /// What it is told, which only that program knows how to read.
    pub data: Vec<u8>,
}

impl Instruction {
    /// What the program is called, where it is one everyone uses.
    pub fn program_name(&self) -> Option<&'static str> {
        program_name(&self.program)
    }
}

/// A signed transaction, as `sendTransaction` carries it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transaction {
    /// `None` for the kind with no version in front of its message, which is
    /// every transaction before 2022 and many since.
    pub version: Option<u8>,
    /// Base58, the first of which names the transaction everywhere else.
    pub signatures: Vec<String>,
    /// How many of the accounts had to sign.
    pub signers: usize,
    /// Every account the transaction touches, as base58.
    pub accounts: Vec<String>,
    pub blockhash: String,
    pub instructions: Vec<Instruction>,
    /// How many tables of further accounts it points at (v0 and after).
    pub lookups: usize,
}

impl Transaction {
    /// What a signed transaction holds. `None` when the bytes are not one.
    pub fn decode(raw: &[u8]) -> Option<Self> {
        let (count, rest) = shortvec(raw)?;
        let (signatures, rest) = chunks(rest, count, 64)?;
        let mut tx = Self {
            signatures: signatures.iter().map(|bytes| base58(bytes)).collect(),
            ..Self::default()
        };
        // A version in front has its top bit set, which a count of signers
        // never does.
        let (&first, rest) = rest.split_first()?;
        let rest = match first & 0x80 {
            0 => {
                tx.signers = usize::from(first);
                rest
            }
            _ => {
                tx.version = Some(first & 0x7f);
                let (&signers, rest) = rest.split_first()?;
                tx.signers = usize::from(signers);
                rest
            }
        };
        // The two counts of accounts that may only be read, which are not
        // worth saying on their own.
        let rest = rest.get(2..)?;
        let (count, rest) = shortvec(rest)?;
        let (accounts, rest) = chunks(rest, count, 32)?;
        tx.accounts = accounts.iter().map(|bytes| base58(bytes)).collect();
        let (blockhash, rest) = rest.split_at_checked(32)?;
        tx.blockhash = base58(blockhash);
        let (count, mut rest) = shortvec(rest)?;
        for _ in 0..count {
            let (&program, after) = rest.split_first()?;
            let (accounts, after) = shortvec(after)?;
            let after = after.get(accounts..)?;
            let (len, after) = shortvec(after)?;
            let (data, after) = after.split_at_checked(len)?;
            tx.instructions.push(Instruction {
                program: tx.accounts.get(usize::from(program))?.clone(),
                accounts,
                data: data.to_vec(),
            });
            rest = after;
        }
        if tx.version.is_some() {
            let (lookups, _) = shortvec(rest)?;
            tx.lookups = lookups;
        }
        Some(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde_json::Value;

    macro_rules! sample {
        ($name:literal) => {
            include_str!(concat!("../testdata/solana/", $name))
        };
    }

    fn at(sample: &str, path: &[&str]) -> Value {
        let mut at: Value = serde_json::from_str(sample).expect("the sample is JSON");
        for step in path {
            at = match step.parse::<usize>() {
                Ok(index) if at.is_array() => at[index].clone(),
                _ => at[*step].clone(),
            };
        }
        at
    }

    #[test]
    fn base58_goes_both_ways() {
        // The system program is thirty-two nothings, so it writes as a row
        // of ones.
        assert_eq!(base58(&[0u8; 32]), "1".repeat(32));
        assert_eq!(from_base58(&"1".repeat(32)), Some(vec![0u8; 32]));
        // And a real address, out of a real transaction.
        let address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        let bytes = from_base58(address).expect("base58");
        assert_eq!(bytes.len(), 32, "an address is thirty-two bytes");
        assert_eq!(base58(&bytes), address);
        assert_eq!(from_base58("not base58: 0OIl"), None);
        assert_eq!(base58(&[]), "");
    }

    #[test]
    fn lamports_read_as_sol() {
        assert_eq!(sol(1_000_000_000), "1 SOL");
        assert_eq!(sol(1_867_492_609_312), "1867.492609312 SOL");
        assert_eq!(sol(5_000), "0.000005 SOL");
        assert_eq!(sol(0), "0 SOL");
    }

    /// The transaction a client sends and the same one as the node reads it
    /// were captured together, so every field can be checked against it.
    #[test]
    fn a_transaction_reads_as_the_node_reads_it() {
        let raw = at(sample!("send_transaction.req.json"), &["params", "0"]);
        let bytes = STANDARD.decode(raw.as_str().unwrap()).expect("base64");
        let tx = Transaction::decode(&bytes).expect("a transaction");
        let node = at(sample!("transaction.res.json"), &["result"]);
        let message = &node["transaction"]["message"];

        assert_eq!(tx.version.map(u64::from), node["version"].as_u64());
        assert_eq!(
            tx.signatures,
            node["transaction"]["signatures"]
                .as_array()
                .unwrap()
                .iter()
                .map(|one| one.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            tx.signers as u64,
            message["header"]["numRequiredSignatures"].as_u64().unwrap()
        );
        assert_eq!(
            tx.accounts,
            message["accountKeys"]
                .as_array()
                .unwrap()
                .iter()
                .map(|one| one.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(tx.blockhash, message["recentBlockhash"].as_str().unwrap());

        let said = message["instructions"].as_array().unwrap();
        assert_eq!(tx.instructions.len(), said.len());
        for (read, said) in tx.instructions.iter().zip(said) {
            let program = said["programIdIndex"].as_u64().unwrap() as usize;
            assert_eq!(read.program, tx.accounts[program]);
            assert_eq!(read.accounts, said["accounts"].as_array().unwrap().len());
            // The node writes an instruction's data as base58.
            assert_eq!(base58(&read.data), said["data"].as_str().unwrap());
        }
        // One of them is the system program, which everyone knows by name.
        assert!(
            tx.instructions
                .iter()
                .any(|one| one.program_name() == Some("System")),
            "{:?}",
            tx.instructions
        );
    }

    /// A node calls some transactions version 1, and their bytes are not laid
    /// out as a legacy or a version 0 one is: the signature is at the end
    /// rather than the front. Reading one by these rules would give nonsense,
    /// so it is declined, and this keeps that deliberate.
    #[test]
    fn a_kind_this_does_not_know_is_declined_rather_than_guessed_at() {
        let raw = at(sample!("send_transaction_v1.req.json"), &["params", "0"]);
        let bytes = STANDARD.decode(raw.as_str().unwrap()).expect("base64");
        assert_eq!(Transaction::decode(&bytes), None);
    }

    #[test]
    fn bytes_that_are_not_a_transaction_are_not_read() {
        assert_eq!(Transaction::decode(b""), None);
        assert_eq!(Transaction::decode(&[1]), None, "a signature is promised");
        // A count that says more than is there.
        assert_eq!(Transaction::decode(&[9, 0, 0]), None);
    }
}
