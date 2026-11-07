//! The connection a flow went over, as the proxy wrote it down: the server's
//! address, TLS, and the certificates it showed, read for the inspector.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use sha2::{Digest, Sha256};
use x509_parser::{extensions::GeneralName, prelude::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub remote: String,
    pub tls: Option<Tls>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tls {
    pub version: String,
    pub cipher: String,
    pub alpn: Option<String>,
    /// The server's own first.
    pub chain: Vec<Certificate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    /// Its common name, or the whole name if it has none.
    pub subject: String,
    pub issuer: String,
    /// Seconds since 1970.
    pub not_before: i64,
    pub not_after: i64,
    pub names: Vec<String>,
    pub serial: String,
    /// Uppercase hex, colon between bytes.
    pub sha256: String,
    pub is_ca: bool,
}

impl Connection {
    pub fn parse(json: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(json).ok()?;
        let tls = value.get("tls").map(|tls| {
            let text = |key: &str| tls.get(key).and_then(Value::as_str).map(str::to_owned);
            Tls {
                version: text("version").unwrap_or_default(),
                cipher: text("cipher").unwrap_or_default(),
                alpn: text("alpn"),
                chain: tls
                    .get("certificates")
                    .and_then(Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(Value::as_str)
                            .filter_map(|b64| STANDARD.decode(b64).ok())
                            .filter_map(|der| Certificate::parse(&der))
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        });
        Some(Self {
            remote: value.get("remote")?.as_str()?.to_owned(),
            tls,
        })
    }
}

impl Certificate {
    pub fn parse(der: &[u8]) -> Option<Self> {
        let (_, cert) = X509Certificate::from_der(der).ok()?;
        let name = |name: &X509Name| {
            name.iter_common_name()
                .next()
                .and_then(|cn| cn.as_str().ok())
                .map_or_else(|| name.to_string(), str::to_owned)
        };
        let names = cert
            .subject_alternative_name()
            .ok()
            .flatten()
            .map(|san| {
                san.value
                    .general_names
                    .iter()
                    .filter_map(|n| match n {
                        GeneralName::DNSName(dns) => Some((*dns).to_owned()),
                        GeneralName::IPAddress(ip) => ip_text(ip),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let digest = Sha256::digest(der);
        Some(Self {
            subject: name(cert.subject()),
            issuer: name(cert.issuer()),
            not_before: cert.validity().not_before.timestamp(),
            not_after: cert.validity().not_after.timestamp(),
            names,
            serial: cert.raw_serial_as_string().to_uppercase(),
            sha256: digest
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":"),
            is_ca: cert.is_ca(),
        })
    }

    /// Whether `now` (seconds since 1970) is outside when it may be used.
    pub fn out_of_date(&self, now: i64) -> bool {
        now < self.not_before || now > self.not_after
    }
}

fn ip_text(bytes: &[u8]) -> Option<String> {
    match bytes.len() {
        4 => Some(std::net::Ipv4Addr::from(<[u8; 4]>::try_from(bytes).ok()?).to_string()),
        16 => Some(std::net::Ipv6Addr::from(<[u8; 16]>::try_from(bytes).ok()?).to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsucker::rcgen;

    fn certificate() -> Vec<u8> {
        let mut params =
            rcgen::CertificateParams::new(vec!["api.test".to_owned(), "127.0.0.1".to_owned()])
                .unwrap();
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "api.test");
        params.not_before = rcgen::date_time_ymd(2026, 1, 1);
        params.not_after = rcgen::date_time_ymd(2027, 1, 1);
        let key = rcgen::KeyPair::generate().unwrap();
        params.self_signed(&key).unwrap().der().to_vec()
    }

    #[test]
    fn what_the_proxy_wrote_reads_back() {
        let der = certificate();
        let json = serde_json::json!({
            "remote": "127.0.0.1:443",
            "tls": {
                "version": "TLS 1.3",
                "cipher": "TLS13_AES_128_GCM_SHA256",
                "alpn": "h2",
                "certificates": [STANDARD.encode(&der)],
            }
        });
        let connection = Connection::parse(json.to_string().as_bytes()).unwrap();
        assert_eq!(connection.remote, "127.0.0.1:443");
        let tls = connection.tls.unwrap();
        assert_eq!(tls.alpn.as_deref(), Some("h2"));
        let cert = &tls.chain[0];
        assert_eq!(cert.subject, "api.test");
        assert_eq!(cert.issuer, "api.test");
        assert_eq!(cert.names, ["api.test", "127.0.0.1"]);
        assert_eq!(cert.not_before, 1_767_225_600);
        assert!(!cert.out_of_date(1_780_000_000));
        assert!(cert.out_of_date(1_800_000_000));
        assert_eq!(cert.sha256.len(), 32 * 3 - 1);
    }

    #[test]
    fn plain_http_has_no_tls() {
        let connection = Connection::parse(br#"{"remote":"10.0.0.1:80"}"#).unwrap();
        assert_eq!(connection.tls, None);
        assert_eq!(Connection::parse(b"{}"), None);
    }
}
