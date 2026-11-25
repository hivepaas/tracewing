//! `tracewing.com`: tracewing's command line in a Windows terminal.
//!
//! tracewing.exe is a program with a window (`windows_subsystem = "windows"`),
//! so that none of a console opens beside it when it is started from the
//! Start menu. A terminal does not wait for such a program, and gives it
//! nothing to write to. This one is built for the console, and is installed
//! beside it as `tracewing.com`. Windows tries `.com` before `.exe`
//! (`PATHEXT`), so `tracewing capture` typed in cmd or PowerShell runs this:
//! it starts tracewing.exe with the same arguments and the terminal's own
//! handles, waits for it, and ends as it ended. tracewing.exe takes this
//! program's console for the command line (its `attach_console`), so Ctrl+C
//! in the terminal reaches it, and it decides what that means; this one lets
//! it pass and goes on waiting.
//!
//! `tracewing` with nothing after it is the window: started, and not waited
//! for, as a click on it would be.
//!
//! The same thing devenv.com does for Visual Studio.

#[cfg(windows)]
fn main() {
    use std::process::{Command, exit};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    /// Ctrl+C, Ctrl+Break: taken, and nothing done.
    unsafe extern "system" fn let_pass(_event: u32) -> i32 {
        1
    }

    let Some(exe) = std::env::current_exe()
        .ok()
        .map(|me| me.with_file_name("tracewing.exe"))
        .filter(|exe| exe.is_file())
    else {
        eprintln!("tracewing: no tracewing.exe beside tracewing.com");
        exit(1);
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut command = Command::new(&exe);
    command.args(&args);
    if args.is_empty() {
        if let Err(e) = command.spawn() {
            eprintln!("tracewing: cannot start {}: {e}", exe.display());
            exit(1);
        }
        return;
    }
    // Ctrl+C and Ctrl+Break are tracewing.exe's to answer: this one waits for
    // it to end, however it ends. With a handler that takes them, not with
    // no handler and TRUE, which ignores them too but is inherited: the
    // program started after would ignore Ctrl+C as well, and a `capture`
    // could not be stopped (measured on Windows 11).
    // SAFETY: `let_pass` is a function that lives as long as the program.
    unsafe { SetConsoleCtrlHandler(Some(let_pass), 1) };
    match command.status() {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("tracewing: cannot start {}: {e}", exe.display());
            exit(1);
        }
    }
}

/// Windows' alone: elsewhere tracewing is one program, built for the terminal.
#[cfg(not(windows))]
fn main() {
    eprintln!("tracewing-console is tracewing.com, for Windows");
    std::process::exit(2);
}
