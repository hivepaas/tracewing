//! The port macOS's network extension hands its flows to.
//!
//! The extension (`crates/netext`) takes the flows of the apps it was told of,
//! apps that were never told a proxy exists. It connects here for each one and
//! says, in [`tracewing_handoff`]'s few lines, where the flow was going and
//! which app made it, signed with a key tracewing gave it; this checks the
//! signature, answers with a proof of its own, and hands the rest of the
//! connection over as a [`Took::Flow`](crate::source::Took). From there it is
//! [`crate::handover`]'s, as every taken flow is: TLS through a tunnel, plain
//! HTTP as it stands, so a `POST` is read and not tunnelled blind.
//!
//! **The program is named, not looked for.** Looking a connection up by its
//! socket's owner, as [`crate::apps`] does for a client that asked, would find
//! the extension: the socket is its. The hello says the app's process, and
//! the program is read from that process here — its file, and the name of its
//! bundle — or, where it has gone already, named by the signing id the
//! extension saw.
//!
//! It is a port of its own rather than a header on a `CONNECT` to the proxy
//! port, because a `CONNECT` is a tunnel, and inside a tunnel the proxy core
//! reads only TLS and a plain `GET`.

use crate::source::{Flow, Found, Source, Stop, Target, stopped};
use anyhow::{Context, Result, anyhow, bail};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracewing_handoff::{HEAD_MAX, Hello, Key, Refused};
use tracewing_store::app::App;

/// How long the extension has to say its lines: a few hundred bytes, written
/// at once by the one program that means it.
const HELLO_WITHIN: Duration = Duration::from_secs(10);

/// The extension's port.
pub struct ExtensionPort {
    listener: std::net::TcpListener,
    key: Key,
}

impl ExtensionPort {
    /// `listener` is bound on the loopback before the proxy's runtime starts,
    /// as the app binds its ports; `key` is the one the extension was given.
    pub fn new(listener: std::net::TcpListener, key: Key) -> Self {
        Self { listener, key }
    }
}

impl Source for ExtensionPort {
    fn name(&self) -> String {
        match self.listener.local_addr() {
            Ok(at) => format!("the network extension's port at {at}"),
            Err(_) => "the network extension's port".to_owned(),
        }
    }

    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        let at = self.listener.local_addr()?;
        // Forgetting this blocks a whole worker thread on an accept.
        self.listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(self.listener)?;
        tokio::spawn(take_them(listener, self.key, [proxy, at], found, stop));
        Ok(())
    }
}

async fn take_them(
    listener: TcpListener,
    key: Key,
    ours: [SocketAddr; 2],
    found: Found,
    stop: Stop,
) {
    loop {
        tokio::select! {
            taken = listener.accept() => {
                let (stream, from) = match taken {
                    Ok(taken) => taken,
                    Err(e) => {
                        tracing::warn!("taking a connection from the network extension: {e}");
                        continue;
                    }
                };
                let (key, found) = (key.clone(), found.clone());
                tokio::spawn(async move {
                    if let Err(e) = take_one(stream, from, &key, ours, &found).await {
                        // A flow turned down is a thing to see in the log;
                        // it is the extension, or something posing as it.
                        tracing::warn!("a hand-off from {from}: {e:#}");
                    }
                });
            }
            _ = stopped(stop.clone()) => break,
        }
    }
}

async fn take_one(
    mut stream: TcpStream,
    from: SocketAddr,
    key: &Key,
    ours: [SocketAddr; 2],
    found: &Found,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let Some(head) = tokio::time::timeout(HELLO_WITHIN, read_head(&mut stream))
        .await
        .map_err(|_| anyhow!("no hello within {HELLO_WITHIN:?}"))??
    else {
        // Opened and closed with nothing said: something seeing whether
        // the port is open, which is no hand-off and nothing to warn of.
        return Ok(());
    };
    let (hello, nonce) = match Hello::read(&head, key) {
        Ok(read) => read,
        Err(why) => {
            let _ = stream.write_all(&tracewing_handoff::refusal(&why)).await;
            return Err(why.into());
        }
    };
    let to = target(&hello);
    if let Target::Addr(addr) = &to
        && ours.contains(addr)
    {
        let why = Refused::Bad("to");
        let _ = stream.write_all(&tracewing_handoff::refusal(&why)).await;
        bail!("{} was going to tracewing itself", hello.app);
    }
    stream
        .write_all(&tracewing_handoff::answer(key, nonce))
        .await
        .context("answering")?;
    let who = program(&hello);
    found
        .flow(Flow {
            stream: Box::new(stream),
            // Where the app's own socket is the extension does not say; this
            // is the extension's end of the hand-off.
            client: from,
            to,
            who: Some(who),
        })
        .await;
    Ok(())
}

/// The lines up to and with the blank one, and not a byte further: what
/// follows is the app's, and goes to the proxy whole. None when the
/// connection closes before a byte of them.
async fn read_head(stream: &mut TcpStream) -> Result<Option<Vec<u8>>> {
    let mut head = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= HEAD_MAX {
            bail!("more than {HEAD_MAX} bytes and no end to the hello");
        }
        if stream.read(&mut byte).await? == 0 {
            if head.is_empty() {
                return Ok(None);
            }
            bail!("closed before the hello ended");
        }
        head.push(byte[0]);
    }
    Ok(Some(head))
}

fn target(hello: &Hello) -> Target {
    match hello.host.parse::<IpAddr>() {
        Ok(ip) => Target::Addr(SocketAddr::new(ip, hello.port)),
        Err(_) => Target::Named {
            host: hello.host.clone(),
            port: hello.port,
        },
    }
}

/// The program, from the process the hello names: its file where it can be
/// read, else the signing id the extension saw.
fn program(hello: &Hello) -> App {
    match hello.pid {
        Some(pid) => {
            let found = crate::apps::describe(pid);
            match found.path {
                Some(_) => found,
                None => App {
                    pid,
                    name: hello.app.clone(),
                    path: None,
                },
            }
        }
        None => App {
            pid: 0,
            name: hello.app.clone(),
            path: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(host: &str, pid: Option<u32>) -> Hello {
        Hello {
            host: host.into(),
            port: 443,
            app: "com.apple.curl".into(),
            pid,
        }
    }

    #[test]
    fn a_name_is_named_and_an_address_is_one() {
        assert_eq!(
            target(&hello("example.com", None)),
            Target::Named {
                host: "example.com".into(),
                port: 443
            }
        );
        assert_eq!(
            target(&hello("2606:4700::1", None)),
            Target::Addr("[2606:4700::1]:443".parse().unwrap())
        );
    }

    #[test]
    fn the_program_is_the_process_or_its_signing_id() {
        let me = program(&hello("example.com", Some(std::process::id())));
        assert_eq!(me.pid, std::process::id());
        assert!(me.path.is_some(), "this test's own file is found");
        let gone = program(&hello("example.com", Some(u32::MAX - 1)));
        assert_eq!((gone.name.as_str(), gone.path), ("com.apple.curl", None));
        let unknown = program(&hello("example.com", None));
        assert_eq!((unknown.pid, unknown.name.as_str()), (0, "com.apple.curl"));
    }
}
