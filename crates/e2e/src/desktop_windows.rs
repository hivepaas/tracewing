//! Windows' proxy setting, WinINet's: read from the registry, where WinINet
//! keeps a copy of what it was told (`reg query`), rather than through the
//! option tracewing reads it by, so that what tracewing reads is checked too.
//! A proxy of the user's own is written the way the Settings app writes one,
//! through WinINet's own option.

use std::{process::Command, ptr};
use windows_sys::Win32::Networking::WinInet::{
    INTERNET_OPTION_PER_CONNECTION_OPTION, INTERNET_OPTION_REFRESH,
    INTERNET_OPTION_SETTINGS_CHANGED, INTERNET_PER_CONN_FLAGS, INTERNET_PER_CONN_OPTION_LISTW,
    INTERNET_PER_CONN_OPTIONW, INTERNET_PER_CONN_OPTIONW_0, INTERNET_PER_CONN_PROXY_BYPASS,
    INTERNET_PER_CONN_PROXY_SERVER, InternetSetOptionW, PROXY_TYPE_DIRECT, PROXY_TYPE_PROXY,
};

const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";

const VALUES: &[&str] = &[
    "ProxyEnable",
    "ProxyServer",
    "ProxyOverride",
    "AutoConfigURL",
];

/// The whole setting, by value name; a value that is not there is empty.
pub type Setting = Vec<(String, String)>;

fn value(name: &str) -> String {
    let out = Command::new("reg")
        .args(["query", KEY, "/v", name])
        .output()
        .expect("run reg");
    // `    ProxyServer    REG_SZ    http=127.0.0.1:1;https=127.0.0.1:1`
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix(name)?.trim_start();
            let (_kind, data) = rest.split_once(char::is_whitespace)?;
            Some(data.trim().to_owned())
        })
        .unwrap_or_default()
}

pub fn setting() -> Setting {
    VALUES
        .iter()
        .map(|name| (name.to_string(), value(name)))
        .collect()
}

pub fn get<'a>(setting: &'a Setting, key: &str) -> &'a str {
    &setting.iter().find(|(k, _)| k == key).unwrap().1
}

/// Whether the setting sends HTTP and HTTPS to tracewing on `port`.
pub fn names_tracewing(setting: &Setting, port: u16) -> bool {
    get(setting, "ProxyEnable") == "0x1"
        && get(setting, "ProxyServer") == format!("http=127.0.0.1:{port};https=127.0.0.1:{port}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// What the setting was before a test changed it, written back when the
/// test ends however it ends: the machine is left as the test found it.
pub struct PutBack(Setting);

impl Drop for PutBack {
    fn drop(&mut self) {
        let on = get(&self.0, "ProxyEnable") == "0x1";
        write(
            on,
            get(&self.0, "ProxyServer"),
            get(&self.0, "ProxyOverride"),
        );
    }
}

/// A proxy of the user's own, so that "as it was" is more than the defaults.
/// Keep what it gives until the test ends.
#[must_use = "the setting is put back when this is dropped"]
pub fn the_users_own() -> PutBack {
    let before = PutBack(setting());
    write(true, "corp.example:3128", "intranet.example");
    before
}

/// Writes the setting as the Settings app does, through WinINet's option.
fn write(on: bool, server: &str, bypass: &str) {
    let mut server = wide(server);
    let mut bypass = wide(bypass);
    let flags = if on {
        PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY
    } else {
        PROXY_TYPE_DIRECT
    };
    let mut options = [
        INTERNET_PER_CONN_OPTIONW {
            dwOption: INTERNET_PER_CONN_FLAGS,
            Value: INTERNET_PER_CONN_OPTIONW_0 { dwValue: flags },
        },
        INTERNET_PER_CONN_OPTIONW {
            dwOption: INTERNET_PER_CONN_PROXY_SERVER,
            Value: INTERNET_PER_CONN_OPTIONW_0 {
                pszValue: server.as_mut_ptr(),
            },
        },
        INTERNET_PER_CONN_OPTIONW {
            dwOption: INTERNET_PER_CONN_PROXY_BYPASS,
            Value: INTERNET_PER_CONN_OPTIONW_0 {
                pszValue: bypass.as_mut_ptr(),
            },
        },
    ];
    let list = INTERNET_PER_CONN_OPTION_LISTW {
        dwSize: size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
        pszConnection: ptr::null_mut(),
        dwOptionCount: options.len() as u32,
        dwOptionError: 0,
        pOptions: options.as_mut_ptr(),
    };
    // SAFETY: the list and the strings it points at outlive the calls.
    unsafe {
        assert!(
            InternetSetOptionW(
                ptr::null(),
                INTERNET_OPTION_PER_CONNECTION_OPTION,
                &list as *const _ as *const _,
                size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
            ) != 0,
            "WinINet refused the setting"
        );
        InternetSetOptionW(
            ptr::null(),
            INTERNET_OPTION_SETTINGS_CHANGED,
            ptr::null(),
            0,
        );
        InternetSetOptionW(ptr::null(), INTERNET_OPTION_REFRESH, ptr::null(), 0);
    }
}
