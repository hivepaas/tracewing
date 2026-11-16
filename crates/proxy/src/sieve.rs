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

use crate::{
    apps::Apps,
    source::{Duplex, Flow, Found, Target},
};
use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

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

/// The programs somebody chose, as the window keeps them.
///
/// Read for every connection and changed while the mode is on, because
/// somebody adding an app must not have to turn the switch off and on again.
///
/// **Empty means nothing is kept**, and that is worth being plain about
/// rather than treating as a mode that does nothing: with no program chosen,
/// every connection on the machine is passed straight through, so the mode is
/// on and watching nobody. It is also the state a first run is in, which is
/// why the switch is not something to offer before there is a list.
#[derive(Clone, Default)]
pub struct Chosen(Arc<RwLock<Vec<String>>>);

impl Chosen {
    pub fn of(programs: Vec<String>) -> Self {
        Self(Arc::new(RwLock::new(programs)))
    }

    /// What the window has now, in place of what it had.
    pub fn these(&self, programs: Vec<String>) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = programs;
    }

    pub fn none(&self) -> bool {
        self.0.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }

    fn holds(&self, program: &str) -> bool {
        one_of(
            program,
            &self.0.read().unwrap_or_else(|e| e.into_inner()).clone(),
        )
    }
}

/// How long a connection waits for the operating system to say which program
/// made it.
///
/// Paid once per connection, and by **every** connection on the machine while
/// the mode is on: a program nobody chose waits too, because until the answer
/// comes there is nothing to say it is nobody's. It is one look at a table of
/// the kernel's, so this is a bug's timeout rather than a network's.
const NAMED_WITHIN: Duration = Duration::from_millis(500);

/// The sieve a mode that captures by route needs: keep the programs somebody
/// chose, and pass everything else on.
///
/// `out` is the way out for what is passed on, injected because on Windows it
/// has to be a socket pinned to the interface the machine's own traffic
/// leaves by -- one following the ordinary routes would go back into
/// tracewing's own adapter.
///
/// A connection whose program cannot be named is **passed on**. Keeping it
/// would mean offering tracewing's certificate to a program on the strength
/// of a guess, which breaks one that pins its own for no gain; passing it on
/// costs nothing but the sight of it.
pub fn by_programs(apps: Apps, chosen: Chosen, out: StraightTo) -> Sieve {
    Sieve::new(
        Box::new(move |client, to| {
            let apps = apps.clone();
            let chosen = chosen.clone();
            Box::pin(async move {
                // Nothing chosen keeps nothing, and asks the operating system
                // nothing either: the answer is the same and this is the one
                // state every machine starts in.
                if chosen.none() {
                    return false;
                }
                // `None` is an address that is not this machine's, which is
                // no program of ours to find.
                let Some(answer) = apps.took(client, to) else {
                    return false;
                };
                let deadline = Instant::now() + NAMED_WITHIN;
                // `wait_until` waits on a condition variable, and nothing on
                // the runtime may: one of these would hold a worker thread
                // for as long as the look-up takes.
                let named = tokio::task::spawn_blocking(move || answer.wait_until(deadline)).await;
                match named.ok().flatten().and_then(|app| app.path) {
                    Some(path) => chosen.holds(&path),
                    // Either the look-up found nobody, or it found a process
                    // whose file could not be read -- one that ended while we
                    // asked, or another user's.
                    None => false,
                }
            })
        }),
        out,
    )
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

    /// Nowhere to go: `by_programs` never asks for a way out in these
    /// tests, and a sieve that did would say so rather than pass silently.
    fn nowhere() -> StraightTo {
        Box::new(|to| {
            Box::pin(async move { Err(io::Error::other(format!("no way out to {to} in a test"))) })
        })
    }

    /// The whole decision against a real program, a real connection and a
    /// real look-up: this test's own process is the program behind the
    /// connection, so choosing the folder its binary is in must keep it, and
    /// choosing somewhere else must not.
    #[tokio::test]
    async fn the_program_behind_a_connection_decides_whether_it_is_kept() {
        let far = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let to = far.local_addr().unwrap();
        let client = std::net::TcpStream::connect(to).unwrap();
        let _accepted = far.accept().unwrap();
        let from = client.local_addr().unwrap();

        // No port of tracewing's is given: this connection was taken on its
        // way, and all four addresses are what name it. See `apps::Look`.
        let apps = Apps::spawn(Vec::new()).unwrap();
        // These two agree because they are the same file read the same way,
        // not because they happened to on this machine: `current_exe` reads
        // `/proc/self/exe` and `apps::os::path_of` reads `/proc/<pid>/exe`,
        // and both resolve the link. A test that leaned on them merely
        // looking alike would be a fact about one checkout's paths.
        let exe = std::env::current_exe().unwrap();
        // A folder rule in the convention `program_rule` writes: every
        // separator inside it comes from `display()`, and the one at the end
        // is the only thing written by hand.
        //
        // On Windows that hand-written `/` is **not** the separator a path
        // there uses, and this is still right -- but only because `plain`
        // turns `/` into `\` before anything is compared, so the trailing
        // one is normalised along with the rest. It works because the
        // comparison normalises, not because the string is right.
        //
        // That is a claim with a guard rather than a hope:
        // `either_separator_is_the_same_separator_on_windows` asserts the
        // normalisation directly, so it goes red before this test does if
        // the normalisation is ever dropped. Without it this test would
        // simply stop keeping anything on Windows and say nothing about why.
        let folder = format!("{}/", exe.parent().unwrap().display());

        let kept = by_programs(apps.clone(), Chosen::of(vec![folder.clone()]), nowhere());
        assert!(
            (kept.keeps)(from, to).await,
            "this program is in the folder that was chosen"
        );

        let others = by_programs(
            apps.clone(),
            Chosen::of(vec!["/nowhere/in/particular/".to_owned()]),
            nowhere(),
        );
        assert!(
            !(others.keeps)(from, to).await,
            "and is not in one that was not"
        );

        // Nothing chosen keeps nothing, which is the state a first run is in.
        let empty = by_programs(apps, Chosen::default(), nowhere());
        assert!(
            !(empty.keeps)(from, to).await,
            "nothing chosen, nothing kept"
        );
    }

    /// A connection whose near end is not this machine's has no program here
    /// to find, and is passed on rather than kept on a guess.
    #[tokio::test]
    async fn a_connection_from_somewhere_else_is_not_kept() {
        let apps = Apps::spawn(Vec::new()).unwrap();
        let sieve = by_programs(apps, Chosen::of(vec!["/".to_owned()]), nowhere());
        assert!(
            !(sieve.keeps)(
                "198.51.100.9:51000".parse().unwrap(),
                "203.0.113.9:443".parse().unwrap()
            )
            .await,
            "not an address of ours, so not a program of ours"
        );
    }

    /// An address of this machine's with no connection behind it: the
    /// look-up is made and answers nobody, which is a different arm from an
    /// address that is not ours at all. Found by ablation -- turning the
    /// final `None` into `true` left all ten other tests passing, because
    /// every one of them stopped at the earlier guard.
    #[tokio::test]
    async fn a_connection_that_is_not_there_is_not_kept() {
        let apps = Apps::spawn(Vec::new()).unwrap();
        let sieve = by_programs(apps, Chosen::of(vec!["/".to_owned()]), nowhere());
        assert!(
            !(sieve.keeps)(
                // Loopback, so it is an address of ours and the look-up
                // happens; port 1, so there is nothing holding it.
                "127.0.0.1:1".parse().unwrap(),
                "127.0.0.1:2".parse().unwrap()
            )
            .await,
            "the look-up answered nobody, and nobody is not kept"
        );
    }

    /// The list is read for every connection, so the window changing it
    /// reaches the next one without the switch being turned off and on.
    #[tokio::test]
    async fn changing_the_list_changes_the_next_answer() {
        let far = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let to = far.local_addr().unwrap();
        let client = std::net::TcpStream::connect(to).unwrap();
        let _accepted = far.accept().unwrap();
        let from = client.local_addr().unwrap();

        let apps = Apps::spawn(Vec::new()).unwrap();
        let chosen = Chosen::default();
        let sieve = by_programs(apps, chosen.clone(), nowhere());
        assert!(!(sieve.keeps)(from, to).await, "nothing chosen yet");

        let exe = std::env::current_exe().unwrap();
        // A folder rule, written as the test above writes one and right for
        // the same reason: `plain` normalises the trailing slash with the
        // rest before comparing.
        chosen.these(vec![format!("{}/", exe.parent().unwrap().display())]);
        assert!(
            (sieve.keeps)(from, to).await,
            "and the same sieve answers the other way once the window has said so"
        );
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
