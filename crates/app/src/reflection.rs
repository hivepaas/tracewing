//! A server's `.proto` files, fetched from the server by gRPC reflection and
//! kept as a descriptor set that the Protobuf schemas read like any other.
//!
//! `https://` goes through tracewing's proxy, so the calls show in Traffic
//! and the proxy's trust applies. `http://` is gRPC with no TLS, as servers
//! on this computer often are, and goes to the server straight.

use anyhow::{Context as _, Result, bail};
use prost::Message as _;
use std::{
    collections::BTreeSet,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use tracewing_i18n::tr;
use tracewing_store::reflection::{Answer, Ask, PATHS, Request, Response, frames, unframe};

const WAIT_MAX: Duration = Duration::from_secs(30);

/// How many times files that others import are asked for, at most.
const ROUNDS_MAX: usize = 8;

/// What was fetched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fetched {
    pub services: Vec<String>,
    pub files: usize,
    /// The descriptor set written.
    pub path: PathBuf,
}

/// `https://api.example.com`, with `https://` when no scheme is given, and
/// nothing after the host.
pub fn base_of(address: &str) -> Result<String> {
    let address = address.trim().trim_end_matches('/');
    if address.is_empty() {
        bail!(tr!("Write the server's address first"));
    }
    let with_scheme = if address.contains("://") {
        address.to_owned()
    } else {
        format!("https://{address}")
    };
    let url = reqwest::Url::parse(&with_scheme).map_err(|e| anyhow::anyhow!("{e}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        bail!(tr!(
            "\"{address}\" is not a server's address",
            address = address
        ));
    }
    // The host as written, with its port: the URL drops a default one.
    let host = with_scheme[url.scheme().len() + 3..]
        .split('/')
        .next()
        .unwrap_or_default();
    Ok(format!("{}://{host}", url.scheme()))
}

/// The file the descriptor set for `base` is kept in, under `dir`.
pub fn file_for(dir: &Path, base: &str) -> PathBuf {
    let name: String = base
        .split_once("://")
        .map_or(base, |(_, rest)| rest)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("{name}.pb"))
}

/// Asks the server at `address` for its services and the files that
/// describe them, and writes them under `dir`.
pub fn fetch(address: &str, proxy: SocketAddr, ca_pem: &Path, dir: &Path) -> Result<Fetched> {
    let base = base_of(address)?;
    let client = if base.starts_with("https://") {
        let ca = std::fs::read(ca_pem).with_context(|| format!("read {}", ca_pem.display()))?;
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(format!("http://{proxy}"))?)
            .tls_certs_merge([reqwest::Certificate::from_pem(&ca)?])
            .timeout(WAIT_MAX)
            .build()?
    } else {
        reqwest::Client::builder()
            .no_proxy()
            .http2_prior_knowledge()
            .timeout(WAIT_MAX)
            .build()?
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (services, set) = runtime.block_on(ask_all(&client, &base))?;
    std::fs::create_dir_all(dir)?;
    let path = file_for(dir, &base);
    let part = path.with_extension("pb.part");
    std::fs::write(&part, set.encode_to_vec())?;
    std::fs::rename(&part, &path)?;
    Ok(Fetched {
        services,
        files: set.file.len(),
        path,
    })
}

async fn ask_all(
    client: &reqwest::Client,
    base: &str,
) -> Result<(Vec<String>, prost_types::FileDescriptorSet)> {
    let ask = |what: Ask| Request {
        host: String::new(),
        ask: Some(what),
    };
    // The newer version first; a server with only the older says it has no
    // such call.
    let mut last = None;
    let mut found = None;
    for path in PATHS {
        match call(client, base, path, &[ask(Ask::ListServices(String::new()))]).await {
            Ok(answers) => {
                found = Some((path, answers));
                break;
            }
            Err(e) => last = Some(e),
        }
    }
    let Some((path, answers)) = found else {
        return Err(last.expect("one was tried"));
    };
    let services: Vec<String> = answers
        .into_iter()
        .filter_map(|answer| match answer.answer {
            Some(Answer::Services(list)) => Some(list.service),
            _ => None,
        })
        .flatten()
        .map(|service| service.name)
        .filter(|name| !name.starts_with("grpc.reflection."))
        .collect();
    if services.is_empty() {
        bail!(tr!("The server has no services to describe"));
    }

    let mut files: Vec<prost_types::FileDescriptorProto> = Vec::new();
    let mut asks: Vec<Request> = services
        .iter()
        .map(|name| ask(Ask::FileContainingSymbol(name.clone())))
        .collect();
    let mut asked: BTreeSet<String> = BTreeSet::new();
    for _ in 0..ROUNDS_MAX {
        if asks.is_empty() {
            break;
        }
        for answer in call(client, base, path, &asks).await? {
            if let Some(Answer::Files(got)) = answer.answer {
                for bytes in got.file_descriptor_proto {
                    let file = prost_types::FileDescriptorProto::decode(&bytes[..])
                        .context("the server sent a file that cannot be read")?;
                    if !files.iter().any(|f| f.name() == file.name()) {
                        files.push(file);
                    }
                }
            }
        }
        // Files imported that did not come along, asked for by name. Google's
        // own are known already.
        let have: BTreeSet<&str> = files.iter().map(|f| f.name()).collect();
        let missing: Vec<String> = files
            .iter()
            .flat_map(|f| f.dependency.iter())
            .filter(|d| !have.contains(d.as_str()) && !d.starts_with("google/protobuf/"))
            .filter(|d| !asked.contains(*d))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        asked.extend(missing.iter().cloned());
        asks = missing
            .into_iter()
            .map(|name| ask(Ask::FileByFilename(name)))
            .collect();
    }
    if files.is_empty() {
        bail!(tr!("The server sent no files for its services"));
    }
    Ok((services, prost_types::FileDescriptorSet { file: files }))
}

/// One call: every request sent at once, every answer read.
async fn call(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    requests: &[Request],
) -> Result<Vec<Response>> {
    let response = client
        .post(format!("{base}{path}"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(frames(requests))
        .send()
        .await?;
    let status = response.status();
    let grpc = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let grpc_status = grpc("grpc-status").and_then(|s| s.parse::<u16>().ok());
    let grpc_message = grpc("grpc-message").unwrap_or_default();
    if !status.is_success() {
        bail!(tr!(
            "The server answered {status}: it may not offer gRPC reflection",
            status = status.to_string()
        ));
    }
    if let Some(code) = grpc_status.filter(|code| *code != 0) {
        bail!(tr!(
            "The server answered {code}: it may not offer gRPC reflection. {message}",
            code = crate::grpc::code_text(code),
            message = grpc_message
        ));
    }
    let body = response.bytes().await?;
    let answers: Vec<Response> =
        unframe(&body).context("the server's answer cannot be read as gRPC")?;
    if answers.is_empty() {
        bail!(tr!(
            "The server answered nothing: it may not offer gRPC reflection"
        ));
    }
    for answer in &answers {
        if let Some(Answer::Error(error)) = &answer.answer
            && requests.len() == 1
        {
            bail!("{}", error.error_message);
        }
    }
    Ok(answers)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server with no TLS on this computer, asked straight over HTTP/2;
    /// it has only the older version of reflection.
    #[test]
    fn a_plain_server_is_asked_for_its_files() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        runtime.spawn(tracewing_dev::benchkit::serve_plain_origin(listener));
        let dir = std::env::temp_dir().join(format!("tracewing-reflection-{}", std::process::id()));
        let unused: SocketAddr = "127.0.0.1:9".parse().unwrap();
        let fetched = fetch(
            &format!("http://localhost:{port}"),
            unused,
            Path::new("/no/ca.pem"),
            &dir,
        )
        .unwrap();
        assert_eq!(fetched.services, ["shop.Orders"]);
        assert_eq!(fetched.files, 1);
        assert_eq!(fetched.path, dir.join(format!("localhost_{port}.pb")));
        let schema = tracewing_store::schema::Schema::load(&[fetched.path]);
        assert!(schema.method("/shop.Orders/Get").is_some());

        let with_path = fetch(
            &format!("http://localhost:{port}/x"),
            unused,
            Path::new("/no/ca.pem"),
            &dir,
        );
        assert!(with_path.is_ok(), "only the host counts");
        let nothing = fetch("http://127.0.0.1:9", unused, Path::new("/no/ca.pem"), &dir);
        assert!(nothing.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn addresses_are_read() {
        assert_eq!(base_of("api.shop.vn").unwrap(), "https://api.shop.vn");
        assert_eq!(
            base_of(" http://localhost:50051/ ").unwrap(),
            "http://localhost:50051"
        );
        assert_eq!(
            base_of("https://api.shop.vn:8443/x/y").unwrap(),
            "https://api.shop.vn:8443"
        );
        assert!(base_of("").is_err());
        assert!(base_of("ftp://x").is_err());
        assert_eq!(
            file_for(Path::new("/d"), "http://localhost:50051"),
            Path::new("/d/localhost_50051.pb")
        );
    }
}
