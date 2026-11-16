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

#[cfg(windows)]
use anyhow::Context as _;

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
    let rest = arguments(args);
    if !rest.is_empty() {
        line.push(' ');
        line.push_str(&rest);
    }
    line
}

/// The arguments alone, quoted the same way.
///
/// `ShellExecuteExW` wants the program and the rest as **two** strings, where
/// `CreateProcessWithTokenW` wants one, so both shapes are needed and the
/// quoting is shared.
pub fn arguments<'a>(args: impl IntoIterator<Item = &'a str>) -> String {
    let mut line = String::new();
    for arg in args {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&quoted(arg));
    }
    line
}

/// The environment a child is to have, as Windows wants it: `NAME=VALUE`
/// pairs in UTF-16, each ended by a zero, the lot ended by another.
///
/// Needed because `CreateProcessWithTokenW` takes a block rather than a list
/// of changes — so "the environment as it is, with these few set" has to be
/// worked out here. `std::process::Command` does that for us on the ordinary
/// path and cannot on this one.
///
/// A name is matched **ignoring case**, because Windows does: setting `Path`
/// where `PATH` exists has to replace it rather than add a second one, and a
/// child with two of them reads whichever it finds first.
pub fn environment<'a>(
    current: impl IntoIterator<Item = (String, String)>,
    set: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<u16> {
    let mut pairs: Vec<(String, String)> = current.into_iter().collect();
    for (name, value) in set {
        match pairs
            .iter_mut()
            .find(|(have, _)| have.eq_ignore_ascii_case(name))
        {
            Some(pair) => pair.1 = value.to_owned(),
            None => pairs.push((name.to_owned(), value.to_owned())),
        }
    }
    let mut block: Vec<u16> = Vec::new();
    for (name, value) in pairs {
        block.extend(format!("{name}={value}").encode_utf16());
        block.push(0);
    }
    // A block with nothing in it is still two zeroes, not none: one ends the
    // last entry that is not there, and one ends the block.
    block.push(0);
    block
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

/// Starts `command` so that it runs at `want`.
///
/// Everything tracewing launches **for the person using it** goes through
/// here, so that the decision is made in one place rather than at six call
/// sites. The ordinary case is still an ordinary spawn.
pub fn start(command: std::process::Command, want: Level) -> anyhow::Result<()> {
    match start_at(elevated(), want) {
        Start::AsWeAre => plainly(command),
        #[cfg(windows)]
        Start::AsTheShell => as_the_shell(command),
        #[cfg(windows)]
        Start::Elevated => asking_windows(command),
        // Neither exists off Windows, and nothing asks for them there: the
        // choice is only offered on Windows. Said rather than ignored,
        // because a level quietly dropped is a child running at the wrong
        // one -- which is the whole fault this module exists to prevent.
        #[cfg(not(windows))]
        other => anyhow::bail!("this system cannot start a child {other:?}"),
    }
}

fn plainly(mut command: std::process::Command) -> anyhow::Result<()> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// The program, its arguments and its directory as the two Windows calls
/// want them, out of a `Command` that was built for the ordinary path.
#[cfg(windows)]
fn parts(command: &std::process::Command) -> (String, Vec<String>, Option<String>) {
    (
        command.get_program().to_string_lossy().into_owned(),
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        command
            .get_current_dir()
            .map(|dir| dir.to_string_lossy().into_owned()),
    )
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Starts it with the token of the shell the person is logged in with, so a
/// child of an elevated tracewing runs at **their** level rather than at
/// tracewing's.
///
/// The token is the real interactive one -- medium integrity, with
/// Administrators present for deny only -- which is what a program started by
/// double-clicking gets. A restricted token made with
/// `SaferComputeTokenFromLevel` is **not** the same thing and a browser may
/// not survive it.
#[cfg(windows)]
fn as_the_shell(command: std::process::Command) -> anyhow::Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            DuplicateTokenEx, SECURITY_IMPERSONATION_LEVEL, SecurityImpersonation, TOKEN_DUPLICATE,
            TOKEN_QUERY, TokenPrimary,
        },
        System::Threading::{
            CREATE_UNICODE_ENVIRONMENT, OpenProcess, OpenProcessToken, PROCESS_INFORMATION,
            PROCESS_QUERY_LIMITED_INFORMATION, STARTUPINFOW,
        },
        UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId},
    };
    // `CreateProcessWithTokenW` lives in advapi32 and is not in the
    // `windows-sys` features this crate takes, so it is declared here.
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn CreateProcessWithTokenW(
            token: HANDLE,
            flags: u32,
            application: *const u16,
            command_line: *mut u16,
            creation_flags: u32,
            environment: *const core::ffi::c_void,
            directory: *const u16,
            startup: *const STARTUPINFOW,
            process: *mut PROCESS_INFORMATION,
        ) -> i32;
    }

    // SAFETY: a window handle that is only compared, and the ids it fills in.
    let shell = unsafe { GetShellWindow() };
    if shell.is_null() {
        // No shell is running -- a kiosk, a server session, an explorer that
        // died. There is no token to borrow, and the one thing this must not
        // do is start the child elevated anyway: that is the fault being
        // avoided, and it would be invisible.
        anyhow::bail!(
            "no desktop shell is running, so there is no ordinary level to start this at"
        );
    }
    let mut pid = 0u32;
    // SAFETY: a pointer to a `u32` of ours.
    unsafe { GetWindowThreadProcessId(shell, &mut pid) };
    if pid == 0 {
        anyhow::bail!("the desktop shell has no process to borrow a level from");
    }
    // SAFETY: a handle closed below on every path that opens one.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return Err(std::io::Error::last_os_error()).context("open the desktop shell");
    }
    let mut theirs: HANDLE = std::ptr::null_mut();
    // SAFETY: the handle above, and a pointer to a handle of ours.
    let opened =
        unsafe { OpenProcessToken(process, TOKEN_DUPLICATE | TOKEN_QUERY, &mut theirs) != 0 };
    // SAFETY: the handle that call gave us, closed exactly once.
    unsafe { CloseHandle(process) };
    if !opened {
        return Err(std::io::Error::last_os_error()).context("read the shell's level");
    }
    let mut ours: HANDLE = std::ptr::null_mut();
    // SAFETY: a token of ours, duplicated as a primary one so that it can
    // start a process rather than only impersonate.
    let copied = unsafe {
        DuplicateTokenEx(
            theirs,
            0,
            std::ptr::null(),
            SecurityImpersonation as SECURITY_IMPERSONATION_LEVEL,
            TokenPrimary,
            &mut ours,
        ) != 0
    };
    // SAFETY: as above.
    unsafe { CloseHandle(theirs) };
    if !copied {
        return Err(std::io::Error::last_os_error()).context("copy the shell's level");
    }

    let (program, args, dir) = parts(&command);
    let mut line = wide(&command_line(&program, args.iter().map(String::as_str)));
    let block = environment(
        std::env::vars(),
        command
            .get_envs()
            .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?))),
    );
    let dir = dir.map(|dir| wide(&dir));
    let mut startup = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    let mut made: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: a token of ours, a command line this call may write into, an
    // environment block ended twice over, and two structures of the sizes it
    // expects.
    let started = unsafe {
        CreateProcessWithTokenW(
            ours,
            0,
            std::ptr::null(),
            line.as_mut_ptr(),
            CREATE_UNICODE_ENVIRONMENT,
            block.as_ptr().cast(),
            dir.as_ref().map_or(std::ptr::null(), |dir| dir.as_ptr()),
            &startup,
            &mut made,
        ) != 0
    };
    let why = std::io::Error::last_os_error();
    // SAFETY: handles this call gave us or we made, each closed once.
    unsafe {
        CloseHandle(ours);
        if started {
            CloseHandle(made.hProcess);
            CloseHandle(made.hThread);
        }
    }
    let _ = &mut startup;
    if started {
        Ok(())
    } else {
        Err(why).with_context(|| format!("start {program} at the shell's level"))
    }
}

/// Starts it by asking Windows to raise it, which shows the person a UAC
/// dialog. For a tracewing that is not elevated and a child that should be.
///
/// **This cannot carry an environment.** `ShellExecuteExW` takes no
/// environment block, so a caller that has variables to set has to put them
/// in the command line instead -- which is what `tracewing env` prints
/// anyway. Refused rather than dropped: a terminal that came up without the
/// proxy variables would look like the proxy not working.
#[cfg(windows)]
fn asking_windows(command: std::process::Command) -> anyhow::Result<()> {
    use windows_sys::Win32::UI::{
        Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW},
        WindowsAndMessaging::SW_SHOWNORMAL,
    };
    // `SW_SHOWNORMAL` is a plain `i32` here, not a newtype as the `windows`
    // crate has it, so there is no `.0` to take. `quic_windows.rs` writes
    // this field as a bare number for the same reason.
    if command.get_envs().next().is_some() {
        anyhow::bail!(
            "a child started as an administrator cannot be given variables through its \
             environment, because Windows' own way of raising one carries none; they have to \
             go in its command line"
        );
    }
    let (program, args, dir) = parts(&command);
    let verb = wide("runas");
    let file = wide(&program);
    let rest = wide(&arguments(args.iter().map(String::as_str)));
    let dir = dir.map(|dir| wide(&dir));
    let mut what = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: rest.as_ptr(),
        lpDirectory: dir.as_ref().map_or(std::ptr::null(), |dir| dir.as_ptr()),
        nShow: SW_SHOWNORMAL,
        ..unsafe { std::mem::zeroed() }
    };
    // SAFETY: a structure of the size it is told, whose strings outlive the
    // call.
    if unsafe { ShellExecuteExW(&mut what) } != 0 {
        return Ok(());
    }
    Err(std::io::Error::last_os_error())
        .with_context(|| format!("ask Windows to start {program} as an administrator"))
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

    fn read_block(block: &[u16]) -> Vec<String> {
        String::from_utf16_lossy(block)
            .split('\0')
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_environment_is_what_was_there_with_the_changes_in_it() {
        let current = [
            ("PATH".to_owned(), r"C:\Windows".to_owned()),
            ("HOME".to_owned(), r"C:\Users\me".to_owned()),
        ];
        let block = environment(current, [("HTTPS_PROXY", "http://127.0.0.1:8080")]);
        assert_eq!(
            read_block(&block),
            vec![
                r"PATH=C:\Windows".to_owned(),
                r"HOME=C:\Users\me".to_owned(),
                "HTTPS_PROXY=http://127.0.0.1:8080".to_owned(),
            ],
            "what was there, in order, with the new one after it"
        );
    }

    /// Windows matches a variable's name without case, so setting `Path`
    /// where `PATH` exists has to **replace** it. A child with two of them
    /// reads whichever it happens to find first, which is the kind of fault
    /// that only shows on somebody else's machine.
    #[test]
    fn a_name_that_differs_only_in_case_replaces_rather_than_doubles() {
        let current = [("PATH".to_owned(), r"C:\Windows".to_owned())];
        let block = environment(current, [("Path", r"C:\Other")]);
        assert_eq!(read_block(&block), vec![r"PATH=C:\Other".to_owned()]);
    }

    /// The shape of the block itself, which is the part a reader gets wrong:
    /// every entry ends in a zero and the block ends in one more, so an empty
    /// environment is **two** zeroes rather than none.
    #[test]
    fn the_block_is_ended_twice_over() {
        assert_eq!(environment([], []), vec![0u16]);
        let one = environment([("A".to_owned(), "b".to_owned())], []);
        assert_eq!(one, vec!['A' as u16, '=' as u16, 'b' as u16, 0, 0]);
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
