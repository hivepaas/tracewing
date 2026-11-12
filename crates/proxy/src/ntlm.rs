//! NTLM, as a company's proxy asks for it in `Proxy-Authenticate: NTLM`: three
//! messages on one connection. tracewing says what it can do (negotiate), the
//! proxy answers with a challenge, and tracewing proves it knows the password
//! (authenticate), NTLMv2 only (MS-NLMP). The password never goes on the wire.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use md4::{Digest, Md4};

type HmacMd5 = Hmac<md5::Md5>;

const SIGNATURE: &[u8; 8] = b"NTLMSSP\0";
const NEGOTIATE_UNICODE: u32 = 0x0000_0001;
const NEGOTIATE_OEM: u32 = 0x0000_0002;
const REQUEST_TARGET: u32 = 0x0000_0004;
const NEGOTIATE_NTLM: u32 = 0x0000_0200;
const NEGOTIATE_ALWAYS_SIGN: u32 = 0x0000_8000;
const NEGOTIATE_EXTENDED_SESSIONSECURITY: u32 = 0x0008_0000;
const NEGOTIATE_TARGET_INFO: u32 = 0x0080_0000;
const NEGOTIATE_128: u32 = 0x2000_0000;
const NEGOTIATE_56: u32 = 0x8000_0000;
const FLAGS: u32 = NEGOTIATE_UNICODE
    | NEGOTIATE_OEM
    | REQUEST_TARGET
    | NEGOTIATE_NTLM
    | NEGOTIATE_ALWAYS_SIGN
    | NEGOTIATE_EXTENDED_SESSIONSECURITY
    | NEGOTIATE_TARGET_INFO
    | NEGOTIATE_128
    | NEGOTIATE_56;
/// The AV pair that carries the server's time.
const AV_TIMESTAMP: u16 = 7;
const AV_EOL: u16 = 0;
/// From 1601 to 1970, in 100 ns.
const FILETIME_1970: u64 = 116_444_736_000_000_000;

/// Who signs in: `DOMAIN\user`, `user@domain`, or a user alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub domain: String,
    pub user: String,
    pub password: String,
}

impl Account {
    pub fn new(name: &str, password: &str) -> Self {
        let (domain, user) = match (name.split_once('\\'), name.rsplit_once('@')) {
            (Some((domain, user)), _) => (domain, user),
            (None, Some((user, domain))) => (domain, user),
            (None, None) => ("", name),
        };
        Self {
            domain: domain.to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
        }
    }
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn hmac_md5(key: &[u8], parts: &[&[u8]]) -> [u8; 16] {
    let mut mac = HmacMd5::new_from_slice(key).expect("HMAC takes any key");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}

/// NTOWFv2: the key both sides derive from the password.
fn ntowf_v2(account: &Account) -> [u8; 16] {
    let nt_hash: [u8; 16] = Md4::digest(utf16(&account.password)).into();
    let who = format!("{}{}", account.user.to_uppercase(), account.domain);
    hmac_md5(&nt_hash, &[&utf16(&who)])
}

/// The first message, as `Proxy-Authorization`'s value.
pub fn negotiate() -> String {
    let mut message = Vec::with_capacity(32);
    message.extend_from_slice(SIGNATURE);
    message.extend_from_slice(&1u32.to_le_bytes());
    message.extend_from_slice(&FLAGS.to_le_bytes());
    // No domain and no workstation given.
    message.extend_from_slice(&[0; 16]);
    format!("NTLM {}", STANDARD.encode(message))
}

/// What the proxy's challenge says.
#[derive(Debug, PartialEq, Eq)]
pub struct Challenge {
    pub server_challenge: [u8; 8],
    pub target_info: Vec<u8>,
    pub flags: u32,
}

/// The challenge in a `Proxy-Authenticate: NTLM …` value.
pub fn read_challenge(header: &str) -> Result<Challenge, String> {
    let encoded = header
        .trim()
        .strip_prefix("NTLM ")
        .ok_or("the proxy sent no NTLM challenge")?;
    let message = STANDARD
        .decode(encoded.trim())
        .map_err(|_| "the proxy's NTLM challenge cannot be read")?;
    let field = |at: usize, len: usize| message.get(at..at + len);
    if field(0, 8) != Some(SIGNATURE) || field(8, 4) != Some(&2u32.to_le_bytes()) {
        return Err("the proxy's NTLM challenge is not one".to_owned());
    }
    let u16_at = |at| field(at, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |at| field(at, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let flags = u32_at(20).ok_or("the proxy's NTLM challenge is cut short")?;
    let server_challenge: [u8; 8] = field(24, 8)
        .ok_or("the proxy's NTLM challenge is cut short")?
        .try_into()
        .expect("eight bytes");
    let target_info = match (u16_at(40), u32_at(44)) {
        (Some(len), Some(at)) if len > 0 => field(at as usize, len as usize)
            .ok_or("the proxy's NTLM challenge is cut short")?
            .to_vec(),
        _ => Vec::new(),
    };
    Ok(Challenge {
        server_challenge,
        target_info,
        flags,
    })
}

/// The server's time in its target info, if it gave one.
fn timestamp_in(target_info: &[u8]) -> Option<[u8; 8]> {
    let mut at = 0;
    while at + 4 <= target_info.len() {
        let id = u16::from_le_bytes([target_info[at], target_info[at + 1]]);
        let len = u16::from_le_bytes([target_info[at + 2], target_info[at + 3]]) as usize;
        let value = target_info.get(at + 4..at + 4 + len)?;
        match id {
            AV_EOL => return None,
            AV_TIMESTAMP if len == 8 => return value.try_into().ok(),
            _ => at += 4 + len,
        }
    }
    None
}

/// NTLMv2's two responses, for this challenge, at `time` (a FILETIME), with
/// `client_challenge`.
fn responses(
    account: &Account,
    challenge: &Challenge,
    time: [u8; 8],
    client_challenge: [u8; 8],
) -> (Vec<u8>, Vec<u8>) {
    let key = ntowf_v2(account);
    let mut blob = Vec::with_capacity(32 + challenge.target_info.len());
    blob.extend_from_slice(&[1, 1, 0, 0, 0, 0, 0, 0]);
    blob.extend_from_slice(&time);
    blob.extend_from_slice(&client_challenge);
    blob.extend_from_slice(&[0; 4]);
    blob.extend_from_slice(&challenge.target_info);
    blob.extend_from_slice(&[0; 4]);
    let proof = hmac_md5(&key, &[&challenge.server_challenge, &blob]);
    let nt = [proof.as_slice(), &blob].concat();
    let lm = [
        hmac_md5(&key, &[&challenge.server_challenge, &client_challenge]).as_slice(),
        &client_challenge,
    ]
    .concat();
    (lm, nt)
}

/// The third message, as `Proxy-Authorization`'s value.
pub fn authenticate(account: &Account, challenge: &Challenge, workstation: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let filetime = FILETIME_1970 + (now.as_nanos() / 100) as u64;
    // The server's own time when it gives one: it checks against that.
    let time = timestamp_in(&challenge.target_info).unwrap_or(filetime.to_le_bytes());
    let mut client_challenge = [0u8; 8];
    let _ = crate::crypto_provider()
        .secure_random
        .fill(&mut client_challenge);
    authenticate_with(account, challenge, workstation, time, client_challenge)
}

fn authenticate_with(
    account: &Account,
    challenge: &Challenge,
    workstation: &str,
    time: [u8; 8],
    client_challenge: [u8; 8],
) -> String {
    let (lm, nt) = responses(account, challenge, time, client_challenge);
    let fields = [
        lm,
        nt,
        utf16(&account.domain),
        utf16(&account.user),
        utf16(workstation),
        Vec::new(),
    ];
    const HEADER: usize = 64;
    let mut message = Vec::new();
    message.extend_from_slice(SIGNATURE);
    message.extend_from_slice(&3u32.to_le_bytes());
    let mut at = HEADER;
    for field in &fields {
        let len = field.len() as u16;
        message.extend_from_slice(&len.to_le_bytes());
        message.extend_from_slice(&len.to_le_bytes());
        message.extend_from_slice(&(at as u32).to_le_bytes());
        at += field.len();
    }
    let flags = FLAGS & (challenge.flags | NEGOTIATE_UNICODE);
    message.extend_from_slice(&flags.to_le_bytes());
    debug_assert_eq!(message.len(), HEADER);
    for field in &fields {
        message.extend_from_slice(field);
    }
    format!("NTLM {}", STANDARD.encode(message))
}

/// What a proxy that asks for NTLM checks, for tests and the fake relay: the
/// user and domain an authenticate message names, and whether its NTLMv2
/// response proves `password`.
pub fn check(
    header: &str,
    challenge: &Challenge,
    password: &str,
) -> Result<(String, String), String> {
    let encoded = header.trim().strip_prefix("NTLM ").ok_or("not NTLM")?;
    let message = STANDARD.decode(encoded.trim()).map_err(|e| e.to_string())?;
    if message.get(0..8) != Some(SIGNATURE) || message.get(8..12) != Some(&3u32.to_le_bytes()) {
        return Err("not an authenticate message".to_owned());
    }
    let field = |n: usize| -> Option<&[u8]> {
        let at = 12 + n * 8;
        let len = u16::from_le_bytes(message.get(at..at + 2)?.try_into().ok()?) as usize;
        let offset = u32::from_le_bytes(message.get(at + 4..at + 8)?.try_into().ok()?) as usize;
        message.get(offset..offset + len)
    };
    let text = |bytes: &[u8]| {
        let units: Vec<u16> = bytes
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)]))
            .collect();
        String::from_utf16_lossy(&units)
    };
    let nt = field(1).ok_or("cut short")?;
    let domain = text(field(2).ok_or("cut short")?);
    let user = text(field(3).ok_or("cut short")?);
    if nt.len() < 16 + 28 {
        return Err("no NTLMv2 response".to_owned());
    }
    let account = Account {
        domain: domain.clone(),
        user: user.clone(),
        password: password.to_owned(),
    };
    let (proof, blob) = nt.split_at(16);
    let expected = hmac_md5(&ntowf_v2(&account), &[&challenge.server_challenge, blob]);
    if proof != expected {
        return Err("wrong password".to_owned());
    }
    Ok((domain, user))
}

/// A challenge message, as a proxy sends it, for tests and the fake relay.
pub fn challenge_header(server_challenge: [u8; 8], target_info: &[u8]) -> String {
    let mut message = Vec::new();
    message.extend_from_slice(SIGNATURE);
    message.extend_from_slice(&2u32.to_le_bytes());
    // No target name.
    message.extend_from_slice(&[0; 8]);
    message.extend_from_slice(&FLAGS.to_le_bytes());
    message.extend_from_slice(&server_challenge);
    message.extend_from_slice(&[0; 8]);
    let len = target_info.len() as u16;
    message.extend_from_slice(&len.to_le_bytes());
    message.extend_from_slice(&len.to_le_bytes());
    message.extend_from_slice(&48u32.to_le_bytes());
    message.extend_from_slice(target_info);
    format!("NTLM {}", STANDARD.encode(message))
}

/// Target info with a NetBIOS domain and computer name, as AV pairs.
pub fn target_info(domain: &str, computer: &str) -> Vec<u8> {
    let mut info = Vec::new();
    for (id, value) in [(2u16, domain), (1u16, computer)] {
        let value = utf16(value);
        info.extend_from_slice(&id.to_le_bytes());
        info.extend_from_slice(&(value.len() as u16).to_le_bytes());
        info.extend_from_slice(&value);
    }
    info.extend_from_slice(&[0; 4]);
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// MS-NLMP 4.2.4: User, Domain, Password; the server's challenge
    /// 0123456789abcdef, the client's aa…, time zero, target info naming
    /// Domain and Server.
    #[test]
    fn ntlm_v2_is_as_the_specification_works_it_out() {
        let account = Account::new("Domain\\User", "Password");
        assert_eq!(hex(&ntowf_v2(&account)), "0c868a403bfd7a93a3001ef22ef02e3f");
        let challenge = Challenge {
            server_challenge: [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
            target_info: target_info("Domain", "Server"),
            flags: FLAGS,
        };
        let (lm, nt) = responses(&account, &challenge, [0; 8], [0xaa; 8]);
        assert_eq!(hex(&nt[..16]), "68cd0ab851e51c96aabc927bebef6a1c");
        assert_eq!(hex(&lm), "86c35097ac9cec102554764a57cccc19aaaaaaaaaaaaaaaa");
    }

    #[test]
    fn a_name_is_read_with_its_domain() {
        assert_eq!(Account::new("CORP\\ann", "p").domain, "CORP");
        assert_eq!(Account::new("ann@corp.example", "p").domain, "corp.example");
        let alone = Account::new("ann", "p");
        assert_eq!((alone.domain.as_str(), alone.user.as_str()), ("", "ann"));
    }

    #[test]
    fn the_three_messages_go_round_and_prove_the_password() {
        assert!(negotiate().starts_with("NTLM TlRMTVNTUAABAAAA"));
        let sent = challenge_header([7; 8], &target_info("CORP", "PROXY"));
        let challenge = read_challenge(&sent).unwrap();
        assert_eq!(challenge.server_challenge, [7; 8]);
        assert_eq!(challenge.target_info, target_info("CORP", "PROXY"));
        let account = Account::new("CORP\\ann", "s3cret");
        let answer = authenticate(&account, &challenge, "LAPTOP");
        assert!(!answer.contains("s3cret"));
        assert_eq!(
            check(&answer, &challenge, "s3cret").unwrap(),
            ("CORP".to_owned(), "ann".to_owned())
        );
        assert!(check(&answer, &challenge, "wrong").is_err());
        assert!(read_challenge("Basic realm=x").is_err());
        assert!(read_challenge("NTLM AAAA").is_err());
    }
}
