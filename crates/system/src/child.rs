//! What level a child process starts at.
//!
//! **Every platform has this question; what differs is the direction.** It is
//! tempting to call it a Windows concept and move on, and that would be wrong
//! in a way that reads as settled: the thing Unix lacks is not elevation, it
//! is **inheritance**.
//!
//! On Unix tracewing runs unprivileged and raises a child when it must, and
//! already does so in two places that each ask this question for themselves —
//! [`crate::quic`]'s `block_quic`, which reaches for `pkexec` when it is not
//! root, and `linux.rs`'s `run_as_admin`, which does the same for the machine
//! trust store. Both are "am I root already? if not, ask", which is this
//! table with two of its cells.
//!
//! On Windows a child **inherits**, so the question arrives the other way up:
//! an elevated tracewing hands its elevation to everything it starts unless
//! it takes steps not to. That is why the *setting* below is offered on
//! Windows only — it is the one place a child would otherwise be given
//! administrator rights **nobody asked for**.
//!
//! Which children it applies to: the ones tracewing starts **for the person
//! using it**, a browser and a terminal. The ones it starts for itself —
//! `pkexec`, `schtasks`, `keytool`, the watchdog, a `node --version` probe —
//! are its own business and want no setting.

/// What level a child should start at.
///
/// Named for the level rather than for stripping privilege, deliberately: the
/// day a Unix mode needs a privileged step, `Administrator` is the arm it
/// already wants rather than a contradiction in the name.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    /// Whatever the person using the computer has, whatever tracewing has.
    /// The default, and what a browser or a terminal gets unless somebody
    /// says otherwise.
    #[default]
    TheirOwn,
    /// An administrator, **on purpose**. For a terminal this is a first-class
    /// case rather than an edge: a program that needs an administrator will
    /// not start in a terminal that is not one, and the terminal way in
    /// exists to put the proxy's variables in front of a program.
    Administrator,
}

/// How to start a child so that it ends up at [`Level`], given what tracewing
/// has right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Start {
    /// An ordinary spawn. The child gets what tracewing has, and that is what
    /// was wanted — either both are unprivileged, or both are meant to be
    /// administrators.
    AsWeAre,
    /// Borrow the token of the shell the person is logged in with, because
    /// tracewing has **more** than the child should.
    AsTheShell,
    /// Ask the system to raise it, because tracewing has **less** than the
    /// child should. On Windows that is a UAC dialog each time, which is
    /// worth saying wherever the choice is offered.
    Elevated,
}

/// Which of the three, for a child that should run at `want`.
///
/// A plain function over two bits so that all four answers are reachable from
/// a test on any platform, while only two of them need a particular machine
/// to carry out.
pub fn start_at(tracewing_is_elevated: bool, want: Level) -> Start {
    match (tracewing_is_elevated, want) {
        (false, Level::TheirOwn) => Start::AsWeAre,
        (true, Level::Administrator) => Start::AsWeAre,
        (true, Level::TheirOwn) => Start::AsTheShell,
        (false, Level::Administrator) => Start::Elevated,
    }
}

/// Whether tracewing itself is running elevated.
///
/// On Windows this is a real question, and the capture mode makes it one that
/// changes within a run.
#[cfg(windows)]
pub fn elevated() -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    let mut token = std::ptr::null_mut();
    // SAFETY: a handle closed below on every path that opens one.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut raised = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut size = 0u32;
    // SAFETY: the buffer is the structure the class asks for, and its length
    // is given as that structure's.
    let got = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&raw mut raised).cast(),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
    };
    // SAFETY: the handle that call gave us, closed exactly once.
    unsafe { CloseHandle(token) };
    got != 0 && raised.TokenIsElevated != 0
}

/// Elsewhere tracewing is never the elevated one. Its privileged steps are
/// separate children it raises (`pkexec`) or a helper the system runs, not
/// the app itself — so there is nothing of its own to hand down.
#[cfg(not(windows))]
pub fn elevated() -> bool {
    false
}

/// A Windows command line for `program` with `args`, quoted as
/// `CommandLineToArgvW` will read it back.
///
/// Needed because the two ways of starting a child at another level do not
/// take an argument list: `CreateProcessWithTokenW` takes one string, and
/// `ShellExecuteExW` takes the program and the rest as a second string. So
/// the quoting that `std::process::Command` normally does for us has to be
/// done here instead — and it is the part of this with something to get
/// wrong, which is why it is a plain function with tests rather than a few
/// lines inside an `unsafe` block.
///
/// The rules are the ones `CommandLineToArgvW` documents: an argument with no
/// space, tab or quote in it needs none; otherwise it is wrapped, each quote
/// inside is escaped, and **a backslash is only special in front of a
/// quote** — including the quote that closes the argument, which is why a
/// trailing run of them is doubled.
pub fn command_line<'a>(program: &str, args: impl IntoIterator<Item = &'a str>) -> String {
    let mut line = quoted(program);
    for arg in args {
        line.push(' ');
        line.push_str(&quoted(arg));
    }
    line
}

fn quoted(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_owned();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut slashes = 0usize;
    for c in arg.chars() {
        match c {
            // Held back: whether these need doubling depends on what comes
            // after them, which is the whole subtlety of this format.
            '\\' => slashes += 1,
            '"' => {
                for _ in 0..slashes * 2 + 1 {
                    out.push('\\');
                }
                slashes = 0;
                out.push('"');
            }
            _ => {
                for _ in 0..slashes {
                    out.push('\\');
                }
                slashes = 0;
                out.push(c);
            }
        }
    }
    // The closing quote is a quote, so the run in front of it is doubled too.
    for _ in 0..slashes * 2 {
        out.push('\\');
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All four cells, including the two that no machine here can carry out.
    #[test]
    fn what_a_child_needs_to_reach_the_level_asked_for() {
        assert_eq!(start_at(false, Level::TheirOwn), Start::AsWeAre);
        assert_eq!(
            start_at(true, Level::Administrator),
            Start::AsWeAre,
            "an elevated tracewing starting an administrator hands down what it has"
        );
        assert_eq!(
            start_at(true, Level::TheirOwn),
            Start::AsTheShell,
            "tracewing has more than the child should, so the shell's token is borrowed"
        );
        assert_eq!(
            start_at(false, Level::Administrator),
            Start::Elevated,
            "tracewing has less than the child should, so the system is asked"
        );
    }

    #[test]
    fn the_default_is_the_person_s_own_level() {
        assert_eq!(Level::default(), Level::TheirOwn);
    }

    /// The quoting rules, which are the part with something to get wrong.
    /// Each case is one of the four things `CommandLineToArgvW` treats
    /// specially.
    #[test]
    fn a_command_line_quoted_as_windows_reads_it_back() {
        // Nothing special: left alone, so an ordinary line stays readable.
        assert_eq!(command_line("cmd", ["/c", "echo"]), "cmd /c echo");
        // A space means wrapping, and this is the common case on Windows.
        assert_eq!(
            command_line(r"C:\Program Files\x\x.exe", ["a b"]),
            r#""C:\Program Files\x\x.exe" "a b""#
        );
        // A quote inside is escaped with a backslash.
        assert_eq!(command_line("x", [r#"say "hi""#]), r#"x "say \"hi\"""#);
        // A backslash is **only** special in front of a quote, so these are
        // left as they are.
        assert_eq!(command_line("x", [r"a\b c"]), r#"x "a\b c""#);
        // But a run of them before a quote is doubled, or the quote would be
        // read as escaped.
        assert_eq!(command_line("x", [r#"a\\"b c"#]), r#"x "a\\\\\"b c""#);
        // And a trailing run is doubled as well, because the quote that
        // closes the argument is still a quote.
        assert_eq!(command_line("x", [r"ends with\"]), r#"x "ends with\\""#);
        // An empty argument has to be written as a pair of quotes, or it
        // vanishes and every argument after it shifts by one.
        assert_eq!(command_line("x", ["", "after"]), r#"x "" after"#);
    }

    /// The round trip in the one direction testable here: what the rules say
    /// a reader will do, done by hand over each case above.
    #[test]
    fn the_quoting_survives_being_read_back() {
        for arg in [
            "plain",
            "with space",
            r#"with "quotes""#,
            r"with\backslash",
            r#"back\\"slash-quote"#,
            r"trailing\",
            "",
        ] {
            let line = command_line("p", [arg]);
            let read = read_back(&line);
            assert_eq!(read, vec!["p".to_owned(), arg.to_owned()], "from {line}");
        }
    }

    /// `CommandLineToArgvW`'s rules, applied here so that the quoting above
    /// is checked against a reader as well as against a table of expected
    /// strings.
    ///
    /// **Neither of the two guards the other, and that was measured rather
    /// than assumed.** Ablating the doubling of a trailing backslash run
    /// fails the table and **passes this one** -- `"ends with\"` leaves the
    /// quote unterminated, so the backslash is no longer in front of a quote
    /// and reads back as itself, by luck. Ablating the empty argument fails
    /// both, and this one's message is the better of the two: `["p"]` against
    /// `["p", ""]` says an argument was **lost**, where the table only says
    /// two strings differ. So the table catches spelling and the reader
    /// catches meaning, and an earlier version of this comment claimed the
    /// reader was simply the stronger of the two, which the first ablation
    /// disproved.
    fn read_back(line: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        let mut started = false;
        let mut slashes = 0usize;
        for c in line.chars() {
            match c {
                '\\' => {
                    slashes += 1;
                    started = true;
                }
                '"' => {
                    for _ in 0..slashes / 2 {
                        current.push('\\');
                    }
                    if slashes % 2 == 1 {
                        current.push('"');
                    } else {
                        quoted = !quoted;
                    }
                    slashes = 0;
                    started = true;
                }
                ' ' | '\t' if !quoted => {
                    for _ in 0..slashes {
                        current.push('\\');
                    }
                    slashes = 0;
                    if started {
                        args.push(std::mem::take(&mut current));
                        started = false;
                    }
                }
                _ => {
                    for _ in 0..slashes {
                        current.push('\\');
                    }
                    slashes = 0;
                    current.push(c);
                    started = true;
                }
            }
        }
        for _ in 0..slashes {
            current.push('\\');
        }
        if started {
            args.push(current);
        }
        args
    }
}
