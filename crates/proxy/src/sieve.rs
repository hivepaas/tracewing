//! Which taken connections the proxy sees, and where the rest go.
//!
//! A source that captures by **route** captures everything the machine sends:
//! the operating system routed it to us, and no route can say "this program".
//! So a mode meant to watch the programs somebody chose holds every other
//! program's traffic as well, and what it does with that traffic is the whole
//! of whether the mode is honest.
//!
//! It is **spliced**: the bytes go to where they were going and are never
//! read. Nothing of that connection is decrypted, nothing of it is written
//! down, and the program at the far end is never offered a certificate of
//! ours, so one that pins its own keeps working. Leaving such a flow out of
//! the window instead would leave all three of those untrue, which is why
//! this is a sieve and not a filter on the display.
//!
//! What a sieve cannot do is keep the packets out of tracewing. They are ours
//! either way, so every cost that is about the stack rather than about the
//! data — the connection cap, the window, UDP being refused — is unchanged by
//! it. Those are written up in
//! [capture-modes.md](../../../docs/design/capture-modes.md).

use crate::source::{Duplex, Flow, Found, Target};
use std::{future::Future, io, net::SocketAddr, pin::Pin};

/// Whether a connection is kept, and the way out for one that is not.
///
/// Two closures rather than a trait, because the only thing that implements
/// it in earnest needs the window's list of programs and an operating
/// system's way of pinning a socket, and a test needs neither. A fake is two
/// lines here instead of a type.
pub struct Sieve {
    keeps: Keeps,
    straight_to: StraightTo,
}

/// Whether the program behind a connection from `client` to `to` is one
/// somebody chose.
pub type Keeps = Box<dyn Fn(SocketAddr, SocketAddr) -> Asking + Send + Sync>;

/// A way to `to` that does not come back through the adapter.
pub type StraightTo = Box<dyn Fn(SocketAddr) -> Leaving + Send + Sync>;

/// The answer comes later: which program holds a socket is looked for by a
/// thread of its own, and nothing here may block the stack's loop while it
/// does.
pub type Asking = Pin<Box<dyn Future<Output = bool> + Send>>;

pub type Leaving = Pin<Box<dyn Future<Output = io::Result<Box<dyn Duplex>>> + Send>>;

impl Sieve {
    pub fn new(keeps: Keeps, straight_to: StraightTo) -> Self {
        Self { keeps, straight_to }
    }
}

/// One taken connection, kept or passed on.
///
/// Deciding comes **before** a byte of it reaches the proxy, and that is the
/// point: a flow handed over and then found to be somebody else's would
/// already have been offered tracewing's certificate, and a program that pins
/// would already have failed. So this waits for the answer, and the wait is
/// paid once per connection.
pub async fn sift(sieve: &Sieve, flow: Flow, found: Found) {
    let Target::Addr(to) = flow.to else {
        // A source of packets hands over an address; nothing here can name a
        // host. If that ever changes, keeping the flow is the safe arm: it is
        // seen rather than passed on unseen.
        found.flow(flow).await;
        return;
    };
    if (sieve.keeps)(flow.client, to).await {
        found.flow(flow).await;
        return;
    }
    let mut ours = flow.stream;
    match (sieve.straight_to)(to).await {
        Ok(mut out) => {
            // Whatever ends it ends it: this is a relay, and both ends closing
            // is the ordinary way out.
            if let Err(e) = tokio::io::copy_bidirectional(&mut ours, &mut out).await {
                tracing::debug!("passing a connection to {to} through ended: {e}");
            }
        }
        // The connection dies, and that is the lesser of the two wrongs. The
        // other arm would be to capture it after all — which would break a
        // program that pins, write its bytes down, and do both to a program
        // nobody asked us to look at.
        Err(e) => tracing::warn!("a connection to {to} that nobody chose cannot be passed on: {e}"),
    }
}

/// Whether `program` is one of `chosen`.
///
/// A `chosen` entry is a program's file, or a folder and everything under it,
/// which is how a program with no name of its own is picked — what `go build`
/// makes is `a.out` every time, so the only thing that tells one from another
/// is where it is. A folder is written with a separator at the end.
pub fn one_of(program: &str, chosen: &[String]) -> bool {
    one_of_as(program, chosen, HERE)
}

/// How this platform compares two paths.
///
/// A constant rather than `cfg!` at each use, so that both answers are
/// reachable from a test on either platform. The Windows rules cannot be run
/// on Linux otherwise, and they are the ones with something to get wrong.
const HERE: Paths = if cfg!(windows) {
    Paths::AsWindowsDoes
} else {
    Paths::AsWritten
};

/// What counts as the same path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Paths {
    /// Byte for byte, as Linux and macOS filesystems mean it.
    AsWritten,
    /// Ignoring case, with `/` and `\` the same separator, and without the
    /// `\\?\` that `canonicalize` puts in front and a running program's own
    /// path does not have.
    AsWindowsDoes,
}

pub fn one_of_as(program: &str, chosen: &[String], how: Paths) -> bool {
    let program = plain(program, how);
    chosen.iter().any(|one| {
        let one = plain(one, how);
        match one.strip_suffix(separator(how)) {
            // A folder, and everything under it. The separator goes back on,
            // so that `…/bin` does not take in `…/binaries/x`.
            Some(folder) => {
                program.starts_with(&format!("{folder}{}", separator(how)))
                    // And the folder itself, if somebody chose a program and
                    // it was written with a separator at the end.
                    || program == folder
            }
            None => program == one,
        }
    })
}

fn separator(how: Paths) -> char {
    match how {
        Paths::AsWritten => '/',
        Paths::AsWindowsDoes => '\\',
    }
}

/// `path` as it will be compared.
fn plain(path: &str, how: Paths) -> String {
    match how {
        Paths::AsWritten => path.to_owned(),
        Paths::AsWindowsDoes => {
            let path = path.replace('/', "\\");
            // `std::fs::canonicalize` answers `\\?\C:\…`, and
            // `QueryFullProcessImageNameW` answers `C:\…`. One list is built
            // from the first and every program arrives as the second, so
            // without this nothing would ever match and the mode would keep
            // nothing at all.
            let path = match path.strip_prefix("\\\\?\\UNC\\") {
                Some(share) => format!("\\\\{share}"),
                None => path
                    .strip_prefix("\\\\?\\")
                    .map(str::to_owned)
                    .unwrap_or(path),
            };
            path.to_lowercase()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chosen(of: &[&str]) -> Vec<String> {
        of.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_program_is_chosen_by_its_file() {
        let list = chosen(&["/usr/bin/curl", "/opt/thing/thing"]);
        assert!(one_of_as("/usr/bin/curl", &list, Paths::AsWritten));
        assert!(!one_of_as("/usr/bin/curlx", &list, Paths::AsWritten));
        assert!(!one_of_as("/usr/bin/wget", &list, Paths::AsWritten));
        // Not a folder: nothing under it is chosen by it.
        assert!(!one_of_as("/usr/bin/curl/inner", &list, Paths::AsWritten));
    }

    #[test]
    fn a_folder_takes_in_everything_under_it() {
        let list = chosen(&["/home/me/build/"]);
        assert!(one_of_as("/home/me/build/a.out", &list, Paths::AsWritten));
        assert!(one_of_as(
            "/home/me/build/deep/down/a.out",
            &list,
            Paths::AsWritten
        ));
        // The folder itself, written without its separator.
        assert!(one_of_as("/home/me/build", &list, Paths::AsWritten));
        // And the trap: a sibling whose name starts with the folder's.
        assert!(!one_of_as(
            "/home/me/buildings/a.out",
            &list,
            Paths::AsWritten
        ));
    }

    /// Linux tells `Curl` from `curl`, and Windows does not. Both arms are
    /// asserted here, on whichever platform this runs.
    #[test]
    fn case_matters_where_the_filesystem_says_it_does() {
        let list = chosen(&["C:\\Program Files\\curl\\curl.exe"]);
        assert!(one_of_as(
            "c:\\program files\\CURL\\Curl.EXE",
            &list,
            Paths::AsWindowsDoes
        ));
        let unix = chosen(&["/usr/bin/curl"]);
        assert!(!one_of_as("/usr/bin/CURL", &unix, Paths::AsWritten));
    }

    /// The one that would have kept nothing at all: a list built by
    /// `canonicalize` against programs that arrive without the prefix.
    #[test]
    fn the_verbatim_prefix_is_not_part_of_the_path() {
        let list = chosen(&["\\\\?\\C:\\Users\\me\\build\\"]);
        assert!(one_of_as(
            "C:\\Users\\me\\build\\a.exe",
            &list,
            Paths::AsWindowsDoes
        ));
        // And the other way round, since either side may carry it.
        let plainly = chosen(&["C:\\Users\\me\\build\\"]);
        assert!(one_of_as(
            "\\\\?\\C:\\Users\\me\\build\\a.exe",
            &plainly,
            Paths::AsWindowsDoes
        ));
        // A UNC share keeps its two leading separators rather than losing
        // them with the prefix.
        let share = chosen(&["\\\\?\\UNC\\server\\tools\\"]);
        assert!(one_of_as(
            "\\\\server\\tools\\a.exe",
            &share,
            Paths::AsWindowsDoes
        ));
    }

    /// Windows writes a separator both ways round and means the same path.
    #[test]
    fn either_separator_is_the_same_separator_on_windows() {
        let list = chosen(&["C:/Users/me/build/"]);
        assert!(one_of_as(
            "C:\\Users\\me\\build\\a.exe",
            &list,
            Paths::AsWindowsDoes
        ));
    }

    #[test]
    fn nothing_chosen_keeps_nothing() {
        assert!(!one_of_as("/usr/bin/curl", &[], Paths::AsWritten));
    }

    /// This platform's answer is one of the two, and the constant is what the
    /// production path uses: a test that only ever called `one_of_as` would
    /// not notice `HERE` being wrong.
    #[test]
    fn this_platform_compares_paths_one_of_the_two_ways() {
        assert_eq!(
            HERE,
            if cfg!(windows) {
                Paths::AsWindowsDoes
            } else {
                Paths::AsWritten
            }
        );
        let here = if cfg!(windows) {
            "C:\\bin\\curl.exe"
        } else {
            "/usr/bin/curl"
        };
        assert!(one_of(here, &chosen(&[here])));
    }
}
