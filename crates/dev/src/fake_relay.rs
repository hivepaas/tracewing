//! A company's proxy, for tests: HTTP (CONNECT, and plain requests passed
//! on whole) or SOCKS5, with a user name and password if given. It writes
//! down what it was asked to reach, a line each.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
};

/// What it was asked: `CONNECT host:port`, `GET http://…`, `SOCKS5 host:port`,
/// each with ` auth=yes`, ` auth=no` or ` auth=wrong`.
pub type Seen = Arc<Mutex<Vec<String>>>;

pub async fn serve_http(listener: TcpListener, user: Option<(String, String)>, seen: Seen) {
    let expected = user.map(|(n, p)| format!("Basic {}", STANDARD.encode(format!("{n}:{p}"))));
    while let Ok((client, _)) = listener.accept().await {
        let (expected, seen) = (expected.clone(), Arc::clone(&seen));
        tokio::spawn(async move {
            let _ = http_one(client, expected, seen).await;
        });
    }
}

async fn read_head(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        if stream.read(&mut byte).await? == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        head.push(byte[0]);
    }
    Ok(String::from_utf8_lossy(&head).into_owned())
}

async fn http_one(
    mut client: TcpStream,
    expected: Option<String>,
    seen: Seen,
) -> std::io::Result<()> {
    let head = read_head(&mut client).await?;
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or_default().to_owned();
    let mut auth = None;
    let mut rest = Vec::new();
    for line in lines.filter(|l| !l.is_empty()) {
        match line.split_once(':') {
            Some((name, value)) if name.eq_ignore_ascii_case("proxy-authorization") => {
                auth = Some(value.trim().to_owned());
            }
            Some((name, _)) if name.eq_ignore_ascii_case("proxy-connection") => {}
            _ => rest.push(line.to_owned()),
        }
    }
    let said = match (&expected, &auth) {
        (_, None) => "no",
        (Some(e), Some(a)) if e == a => "yes",
        (None, Some(_)) => "yes",
        _ => "wrong",
    };
    let mut parts = first.split_whitespace();
    let (method, target) = (
        parts.next().unwrap_or_default().to_owned(),
        parts.next().unwrap_or_default().to_owned(),
    );
    seen.lock()
        .unwrap()
        .push(format!("{method} {target} auth={said}"));
    if expected.is_some() && said != "yes" {
        client
            .write_all(
                b"HTTP/1.1 407 Proxy Authentication Required\r\n\
                  Proxy-Authenticate: Basic realm=\"corp\"\r\nContent-Length: 0\r\n\r\n",
            )
            .await?;
        return Ok(());
    }
    if method == "CONNECT" {
        let mut server = TcpStream::connect(target.as_str()).await?;
        client
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await?;
        let _ = copy_bidirectional(&mut client, &mut server).await;
        return Ok(());
    }
    // A request to pass on: its URL in full, to the server as a path.
    let after = target.strip_prefix("http://").unwrap_or(&target);
    let (address, path) = match after.find('/') {
        Some(at) => (&after[..at], &after[at..]),
        None => (after, "/"),
    };
    let address = if address.contains(':') {
        address.to_owned()
    } else {
        format!("{address}:80")
    };
    let mut server = TcpStream::connect(address.as_str()).await?;
    let version = parts.next().unwrap_or("HTTP/1.1");
    let mut out = format!("{method} {path} {version}\r\n");
    for line in rest {
        out.push_str(&line);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    server.write_all(out.as_bytes()).await?;
    let _ = copy_bidirectional(&mut client, &mut server).await;
    Ok(())
}

/// A company's proxy that takes NTLM alone, as a Windows domain's often
/// does: each 407 offers NTLM only, with a body, and keeps the connection,
/// as NTLM needs. What it saw is written down with ` auth=ntlm`, ` auth=no`
/// or ` auth=wrong`, and plain requests with their method.
pub async fn serve_http_ntlm(listener: TcpListener, user: (String, String), seen: Seen) {
    while let Ok((client, _)) = listener.accept().await {
        let (user, seen) = (user.clone(), Arc::clone(&seen));
        tokio::spawn(async move {
            let _ = ntlm_one(client, user, seen).await;
        });
    }
}

async fn ntlm_one(
    mut client: TcpStream,
    (name, password): (String, String),
    seen: Seen,
) -> std::io::Result<()> {
    use tracewing_proxy::ntlm;
    let account = ntlm::Account::new(&name, &password);
    let mut challenge = None;
    loop {
        let head = match read_head(&mut client).await {
            Ok(head) => head,
            Err(_) => return Ok(()),
        };
        let mut lines = head.split("\r\n");
        let first = lines.next().unwrap_or_default().to_owned();
        let auth = lines.find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("proxy-authorization")
                .then(|| value.trim().to_owned())
        });
        let mut parts = first.split_whitespace();
        let (method, target) = (
            parts.next().unwrap_or_default().to_owned(),
            parts.next().unwrap_or_default().to_owned(),
        );
        let refuse = |ask: String| {
            format!(
                "HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: {ask}\r\n\
                 Content-Type: text/plain\r\nContent-Length: 7\r\n\r\nsign in"
            )
        };
        match auth.as_deref() {
            Some(value) if value.starts_with("NTLM ") && challenge.is_none() => {
                // Negotiate: the challenge, on this connection.
                let header = ntlm::challenge_header([9; 8], &ntlm::target_info("CORP", "PROXY"));
                challenge = ntlm::read_challenge(&header).ok();
                client.write_all(refuse(header).as_bytes()).await?;
            }
            Some(value) if value.starts_with("NTLM ") => {
                let proved = challenge
                    .as_ref()
                    .map(|c| ntlm::check(value, c, &password))
                    .is_some_and(|who| {
                        who.is_ok_and(|(d, u)| d == account.domain && u == account.user)
                    });
                let said = if proved { "ntlm" } else { "wrong" };
                seen.lock()
                    .unwrap()
                    .push(format!("{method} {target} auth={said}"));
                if !proved {
                    client
                        .write_all(refuse("NTLM".to_owned()).as_bytes())
                        .await?;
                    return Ok(());
                }
                let mut server = TcpStream::connect(target.as_str()).await?;
                client
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await?;
                let _ = copy_bidirectional(&mut client, &mut server).await;
                return Ok(());
            }
            _ => {
                seen.lock()
                    .unwrap()
                    .push(format!("{method} {target} auth=no"));
                client
                    .write_all(refuse("NTLM".to_owned()).as_bytes())
                    .await?;
            }
        }
    }
}

pub async fn serve_socks5(listener: TcpListener, user: Option<(String, String)>, seen: Seen) {
    while let Ok((client, _)) = listener.accept().await {
        let (user, seen) = (user.clone(), Arc::clone(&seen));
        tokio::spawn(async move {
            let _ = socks_one(client, user, seen).await;
        });
    }
}

async fn socks_one(
    mut client: TcpStream,
    user: Option<(String, String)>,
    seen: Seen,
) -> std::io::Result<()> {
    let mut greeting = [0u8; 2];
    client.read_exact(&mut greeting).await?;
    let mut methods = vec![0u8; greeting[1] as usize];
    client.read_exact(&mut methods).await?;
    let mut said = "no";
    match &user {
        Some((name, password)) => {
            if !methods.contains(&2) {
                client.write_all(&[5, 0xff]).await?;
                return Ok(());
            }
            client.write_all(&[5, 2]).await?;
            let mut version_len = [0u8; 2];
            client.read_exact(&mut version_len).await?;
            let mut got_name = vec![0u8; version_len[1] as usize];
            client.read_exact(&mut got_name).await?;
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut got_password = vec![0u8; len[0] as usize];
            client.read_exact(&mut got_password).await?;
            let good = got_name == name.as_bytes() && got_password == password.as_bytes();
            client.write_all(&[1, if good { 0 } else { 1 }]).await?;
            if !good {
                seen.lock().unwrap().push("SOCKS5 auth=wrong".to_owned());
                return Ok(());
            }
            said = "yes";
        }
        None => client.write_all(&[5, 0]).await?,
    }
    let mut ask = [0u8; 4];
    client.read_exact(&mut ask).await?;
    let host = match ask[3] {
        1 => {
            let mut ip = [0u8; 4];
            client.read_exact(&mut ip).await?;
            std::net::Ipv4Addr::from(ip).to_string()
        }
        4 => {
            let mut ip = [0u8; 16];
            client.read_exact(&mut ip).await?;
            format!("[{}]", std::net::Ipv6Addr::from(ip))
        }
        _ => {
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut name = vec![0u8; len[0] as usize];
            client.read_exact(&mut name).await?;
            String::from_utf8_lossy(&name).into_owned()
        }
    };
    let mut port = [0u8; 2];
    client.read_exact(&mut port).await?;
    let target = format!("{host}:{}", u16::from_be_bytes(port));
    seen.lock()
        .unwrap()
        .push(format!("SOCKS5 {target} auth={said}"));
    let mut server = TcpStream::connect(target.as_str()).await?;
    client.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
    let _ = copy_bidirectional(&mut client, &mut server).await;
    Ok(())
}
