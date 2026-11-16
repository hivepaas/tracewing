//! tracewing's end of its network extension, on macOS: installing it, and
//! saving, starting and stopping the transparent proxy it runs.
//!
//! The extension (`crates/netext`) takes the flows of the apps its
//! configuration names and hands them to the port the engine opens for it
//! (`tracewing_proxy::extension`). This is what puts that configuration where
//! macOS reads it: a `NETransparentProxyManager`, saved with the port, a key
//! made for this run alone, the apps, and tracewing's other ports. A new key
//! each run means nothing secret is kept between runs, and what an earlier run
//! saved is worthless once it is over.
//!
//! **Not from the main thread.** Each call here waits for macOS, and macOS
//! answers most of them on the main queue, so a call made from the main
//! thread would wait for itself. The window calls them from its background
//! executor; the command line runs its work on another thread and lends the
//! main thread to macOS with [`serve_main_thread_while`].
//!
//! **A run that ends badly leaves the proxy on, and that is harmless.** The
//! extension hands a flow over only to something on the port that proves it
//! holds the key, so with tracewing gone every flow goes out as it would have.
//! The next run saves its own configuration over it, and stopping or
//! [`uninstall`] turns it off.

#[cfg(target_os = "macos")]
#[path = "extension/macos.rs"]
mod os;
#[path = "extension/program.rs"]
mod program;

pub use program::{Program, code_within, identifier_in, program_at};

use anyhow::Result;
pub use tracewing_handoff::Key;
pub use tracewing_netext::flow::{Config, EXTENSION_ID};

/// Where the extension is, as macOS says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Installed {
    /// Installed and allowed.
    Yes,
    /// Installed, and waiting for the person to allow it in System Settings.
    WaitingForApproval,
    No,
    /// Not known: only the app the extension is in may ask, and this
    /// program is not in one (a build of your own, run from `target/`).
    Unknown,
}

/// What the proxy configuration is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Running {
    /// None saved.
    None,
    Off,
    Starting,
    On,
    Stopping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub installed: Installed,
    pub running: Running,
}

/// Where to allow it, said whenever macOS waits for that.
pub const WHERE_TO_ALLOW: &str =
    "System Settings › General › Login Items & Extensions › Network Extensions";

/// Installs the extension, or makes sure the one installed is this app's own
/// version. Returns once macOS has, which can be after the person allows it:
/// `waiting` is called when macOS starts waiting for that.
pub fn install(waiting: &dyn Fn()) -> Result<()> {
    imp::install(waiting)
}

/// Saves `config` and starts the proxy with it, stopping the one running
/// first. Returns once it runs.
pub fn start(config: &Config) -> Result<()> {
    imp::start(config)
}

/// Stops the proxy, if it runs.
pub fn stop() -> Result<()> {
    imp::stop()
}

/// Stops the proxy this program started, as it exits, from any thread, the
/// main one included: the word to stop is sent and not waited for, since an
/// exiting window cannot serve the main queue where the answer would come.
/// Should the word be lost, the proxy left running is harmless (above).
pub fn stop_on_exit() {
    imp::stop_on_exit()
}

pub fn status() -> Result<Status> {
    imp::status()
}

/// Whether this program runs from inside an app bundle, which is what macOS
/// asks of whatever installs, or asks about, its extension.
pub fn in_app_bundle() -> bool {
    std::env::current_exe().is_ok_and(|exe| {
        exe.parent()
            .is_some_and(|dir| dir.ends_with("Contents/MacOS"))
            && exe
                .ancestors()
                .any(|a| a.extension().is_some_and(|e| e == "app"))
    })
}

/// Removes the configuration, then the extension.
pub fn uninstall(waiting: &dyn Fn()) -> Result<()> {
    imp::uninstall(waiting)
}

/// Runs `work` on a thread of its own while this one, the main thread, serves
/// macOS's main queue, where the answers to the calls above come. For the
/// command line, whose main thread would otherwise sit waiting.
pub fn serve_main_thread_while<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    imp::serve_main_thread_while(work)
}

#[cfg(target_os = "macos")]
use os as imp;

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;
    use anyhow::bail;

    const NOT_HERE: &str = "capturing an app with no proxy settings is macOS's alone, for now";

    pub fn install(_: &dyn Fn()) -> Result<()> {
        bail!(NOT_HERE)
    }
    pub fn start(_: &Config) -> Result<()> {
        bail!(NOT_HERE)
    }
    pub fn stop() -> Result<()> {
        Ok(())
    }
    pub fn stop_on_exit() {}
    pub fn status() -> Result<Status> {
        Ok(Status {
            installed: Installed::No,
            running: Running::None,
        })
    }
    pub fn uninstall(_: &dyn Fn()) -> Result<()> {
        Ok(())
    }
    pub fn serve_main_thread_while<T: Send>(work: impl FnOnce() -> T + Send) -> T {
        work()
    }
}

/// What an `OSSystemExtensionErrorDomain` code means for the person, where
/// there is more to say than macOS's own words.
pub fn install_problem(code: isize) -> Option<&'static str> {
    Some(match code {
        // MissingEntitlement, CodeSignatureInvalid, ValidationFailed.
        2 | 8 | 9 => {
            "this tracewing is not signed for its network extension: only the release, \
             signed with the Developer ID and notarized, is"
        }
        3 => "tracewing.app has to be in /Applications to install its network extension",
        4 => {
            "this tracewing has no network extension in it: a build of your own leaves it \
             out unless it is signed with the profiles that allow it"
        }
        10 => "macOS's policy on this computer forbids it, as a company's management can",
        11 => "the request was cancelled",
        12 => "another request to install it came after this one",
        13 => "it has to be allowed by someone who may change this computer's settings",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_install_problem_says_what_to_do() {
        assert!(install_problem(3).unwrap().contains("/Applications"));
        assert!(install_problem(8).unwrap().contains("signed"));
        assert!(install_problem(4).unwrap().contains("no network extension"));
        assert_eq!(install_problem(1), None, "Unknown: macOS's own words");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn elsewhere_it_says_so() {
        let config = Config {
            port: 1,
            key: Key::from_bytes([0; 32]),
            apps: vec!["curl".into()],
            own: vec![],
        };
        assert!(start(&config).unwrap_err().to_string().contains("macOS"));
        assert_eq!(serve_main_thread_while(|| 7), 7);
    }
}
