//! Windows: WinINet's per-connection options for the proxy, the certificate
//! stores for the CA, `RunOnce` for recovery at the next login.
//!
//! WinINet's settings are per user and need no rights. WinHTTP has its own
//! machine-wide setting, which is left alone; programs built on it only follow
//! the WinINet setting if they ask to.

use anyhow::{Context, Result, bail};
use std::{ffi::c_void, mem::size_of, net::SocketAddr, path::Path, ptr};
use windows_sys::{
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, GetLastError, GlobalFree},
        Networking::WinInet::{
            INTERNET_OPTION_PER_CONNECTION_OPTION, INTERNET_OPTION_REFRESH,
            INTERNET_OPTION_SETTINGS_CHANGED, INTERNET_PER_CONN_AUTOCONFIG_URL,
            INTERNET_PER_CONN_FLAGS, INTERNET_PER_CONN_FLAGS_UI, INTERNET_PER_CONN_OPTION_LISTW,
            INTERNET_PER_CONN_OPTIONW, INTERNET_PER_CONN_OPTIONW_0, INTERNET_PER_CONN_PROXY_BYPASS,
            INTERNET_PER_CONN_PROXY_SERVER, InternetQueryOptionW, InternetSetOptionW,
            PROXY_TYPE_DIRECT, PROXY_TYPE_PROXY,
        },
        Security::Cryptography::{
            CERT_FIND_EXISTING, CERT_STORE_ADD_REPLACE_EXISTING, CERT_STORE_PROV_SYSTEM_W,
            CERT_STORE_READONLY_FLAG, CERT_SYSTEM_STORE_CURRENT_USER,
            CERT_SYSTEM_STORE_LOCAL_MACHINE, CertAddEncodedCertificateToStore, CertCloseStore,
            CertCreateCertificateContext, CertDeleteCertificateFromStore,
            CertFindCertificateInStore, CertFreeCertificateContext, CertOpenStore, HCERTSTORE,
            X509_ASN_ENCODING,
        },
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
            RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
        },
    },
    core::PWSTR,
};

use super::{
    BrowserTrust, Environment, Place, Saved, Scope,
    launch::{self, Browser, Engine},
};

const RUN_ONCE: &str = r"Software\Microsoft\Windows\CurrentVersion\RunOnce";
const RUN_ONCE_VALUE: &str = "tracewing-proxy-recovery";
const BYPASS: &str = "localhost;127.*;[::1];<local>";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn os_error(what: &str) -> anyhow::Error {
    let code = unsafe { GetLastError() };
    anyhow::anyhow!("{what}: {}", std::io::Error::from_raw_os_error(code as i32))
}

fn option(id: u32) -> INTERNET_PER_CONN_OPTIONW {
    INTERNET_PER_CONN_OPTIONW {
        dwOption: id,
        Value: INTERNET_PER_CONN_OPTIONW_0 { dwValue: 0 },
    }
}

fn list(options: &mut [INTERNET_PER_CONN_OPTIONW]) -> INTERNET_PER_CONN_OPTION_LISTW {
    INTERNET_PER_CONN_OPTION_LISTW {
        dwSize: size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
        // Null means the LAN settings, which every connection without its own
        // falls back on.
        pszConnection: ptr::null_mut(),
        dwOptionCount: options.len() as u32,
        dwOptionError: 0,
        pOptions: options.as_mut_ptr(),
    }
}

/// Takes a string WinINet allocated.
unsafe fn take_string(value: PWSTR) -> String {
    if value.is_null() {
        return String::new();
    }
    let text = unsafe {
        let len = (0..).take_while(|&i| *value.add(i) != 0).count();
        String::from_utf16_lossy(std::slice::from_raw_parts(value, len))
    };
    unsafe { GlobalFree(value as _) };
    text
}

pub fn read_proxy() -> Result<Saved> {
    // FLAGS_UI is what the Settings app shows, auto-detect included. Older
    // systems only have FLAGS, but everything GPUI runs on has FLAGS_UI.
    let mut options = [
        option(INTERNET_PER_CONN_FLAGS_UI),
        option(INTERNET_PER_CONN_PROXY_SERVER),
        option(INTERNET_PER_CONN_PROXY_BYPASS),
        option(INTERNET_PER_CONN_AUTOCONFIG_URL),
    ];
    let mut query = list(&mut options);
    let mut size = size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32;
    let ok = unsafe {
        InternetQueryOptionW(
            ptr::null(),
            INTERNET_OPTION_PER_CONNECTION_OPTION,
            &mut query as *mut _ as *mut c_void,
            &mut size,
        )
    };
    if ok == 0 {
        return Err(os_error("read the proxy settings"));
    }

    let mut saved = Saved::default();
    unsafe {
        saved.set("flags", options[0].Value.dwValue.to_string());
        saved.set("server", take_string(options[1].Value.pszValue));
        saved.set("bypass", take_string(options[2].Value.pszValue));
        saved.set("autoconfig", take_string(options[3].Value.pszValue));
    }
    Ok(saved)
}

fn write_proxy(flags: u32, server: &str, bypass: &str, autoconfig: &str) -> Result<()> {
    let (mut server, mut bypass, mut autoconfig) = (wide(server), wide(bypass), wide(autoconfig));
    let text = |id, value: &mut Vec<u16>| INTERNET_PER_CONN_OPTIONW {
        dwOption: id,
        Value: INTERNET_PER_CONN_OPTIONW_0 {
            pszValue: value.as_mut_ptr(),
        },
    };
    let mut options = [
        INTERNET_PER_CONN_OPTIONW {
            dwOption: INTERNET_PER_CONN_FLAGS,
            Value: INTERNET_PER_CONN_OPTIONW_0 { dwValue: flags },
        },
        text(INTERNET_PER_CONN_PROXY_SERVER, &mut server),
        text(INTERNET_PER_CONN_PROXY_BYPASS, &mut bypass),
        text(INTERNET_PER_CONN_AUTOCONFIG_URL, &mut autoconfig),
    ];
    let set = list(&mut options);
    let ok = unsafe {
        InternetSetOptionW(
            ptr::null(),
            INTERNET_OPTION_PER_CONNECTION_OPTION,
            &set as *const _ as *const c_void,
            size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
        )
    };
    if ok == 0 {
        return Err(os_error("write the proxy settings"));
    }
    // Tells running programs to read the settings again.
    unsafe {
        InternetSetOptionW(
            ptr::null(),
            INTERNET_OPTION_SETTINGS_CHANGED,
            ptr::null(),
            0,
        );
        InternetSetOptionW(ptr::null(), INTERNET_OPTION_REFRESH, ptr::null(), 0);
    }
    Ok(())
}

fn server_for(addr: SocketAddr) -> String {
    // Named per scheme, so that FTP and SOCKS do not go to an HTTP proxy.
    format!("http={addr};https={addr}")
}

pub fn set_proxy(addr: SocketAddr) -> Result<()> {
    write_proxy(
        PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY,
        &server_for(addr),
        BYPASS,
        "",
    )
}

pub fn proxy_is(addr: SocketAddr) -> Result<bool> {
    let now = read_proxy()?;
    let flags: u32 = now.get("flags").parse()?;
    Ok(flags & PROXY_TYPE_PROXY != 0 && now.get("server") == server_for(addr))
}

pub fn restore_proxy(saved: &Saved, _addr: SocketAddr) -> Result<()> {
    write_proxy(
        saved.get("flags").parse()?,
        saved.get("server"),
        saved.get("bypass"),
        saved.get("autoconfig"),
    )
}

fn with_run_once<T>(run: impl FnOnce(HKEY) -> Result<T>) -> Result<T> {
    let mut key: HKEY = ptr::null_mut();
    let path = wide(RUN_ONCE);
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };
    if status != 0 {
        bail!(
            "open RunOnce: {}",
            std::io::Error::from_raw_os_error(status as i32)
        );
    }
    let result = run(key);
    unsafe { RegCloseKey(key) };
    result
}

pub fn arm_recovery(command: &[String]) -> Result<()> {
    let line = command
        .iter()
        .map(|part| format!("\"{part}\""))
        .collect::<Vec<_>>()
        .join(" ");
    if line.len() > 260 {
        bail!("the recovery command is longer than RunOnce allows");
    }
    let name = wide(RUN_ONCE_VALUE);
    let data = wide(&line);
    with_run_once(|key| {
        let status = unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        };
        if status != 0 {
            bail!(
                "write RunOnce: {}",
                std::io::Error::from_raw_os_error(status as i32)
            );
        }
        Ok(())
    })
}

pub fn disarm_recovery() -> Result<()> {
    let name = wide(RUN_ONCE_VALUE);
    with_run_once(|key| {
        let status = unsafe { RegDeleteValueW(key, name.as_ptr()) };
        if status != 0 && status != ERROR_FILE_NOT_FOUND {
            bail!(
                "clear RunOnce: {}",
                std::io::Error::from_raw_os_error(status as i32)
            );
        }
        Ok(())
    })
}

const TASK_NAME: &str = "tracewing-proxy-recovery";

/// schtasks, with no console window of its own.
fn schtasks(args: &[&str]) -> Result<std::process::Output> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = std::process::Command::new("schtasks.exe");
    command.args(args);
    std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
    command.output().context("run schtasks")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Whose task it is: the account Task Scheduler is to run it as.
///
/// The name on its own, which Windows resolves against this machine and, on
/// one that has joined a domain, against the domain. Not
/// `%USERDOMAIN%\%USERNAME%`: on a machine that has joined nothing,
/// `USERDOMAIN` holds the workgroup's name — `WORKGROUP` — and no account
/// answers to that. Task Scheduler's words for it are "No mapping between
/// account names and security IDs was done".
fn whose() -> Result<String> {
    std::env::var("USERNAME").context("USERNAME is not set")
}

/// The task's definition, as Task Scheduler keeps it.
///
/// Registered from XML rather than from `schtasks` arguments, because two
/// things that matter here can only be said this way: that it runs **without
/// the user's password** (S4U), and that it repeats for ever rather than for
/// a day.
///
/// S4U is not a detail. tracewing is one program for the window and the
/// command line, so it is built for the console; a task running in the user's
/// own session would put a console window on their screen once a minute, for
/// as long as the system proxy was on. S4U has no session and no desktop, so
/// there is nothing to see.
fn task_xml(command: &[String], user: &str) -> Result<String> {
    let (program, args) = command.split_first().context("no program for the timer")?;
    let arguments = args
        .iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>tracewing</Author>
    <Description>Puts the system proxy back if tracewing is gone, so that the network comes back by itself.</Description>
  </RegistrationInfo>
  <Triggers>
    <TimeTrigger>
      <StartBoundary>2000-01-01T00:00:00</StartBoundary>
      <Enabled>true</Enabled>
      <Repetition>
        <Interval>PT1M</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
    </TimeTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>S4U</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <Hidden>true</Hidden>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable>
    <ExecutionTimeLimit>PT1M</ExecutionTimeLimit>
    <AllowHardTerminate>true</AllowHardTerminate>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{program}</Command>
      <Arguments>{arguments}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        user = escape_xml(user),
        program = escape_xml(&program.to_string()),
        arguments = escape_xml(&arguments),
    ))
}

/// Runs `command` once a minute, for as long as the system proxy is ours.
///
/// A scheduled task, which goes on running after `taskkill` has taken both
/// tracewing and the watchdog: Task Scheduler, not the app, is what starts
/// it. It is registered when the system proxy goes on and deleted when it
/// goes off, so nothing of it is left behind.
pub fn arm_timer(command: &[String], _journal: &Path) -> Result<()> {
    let xml = task_xml(command, &whose()?)?;
    // Task Scheduler reads these as Unicode, and says nothing useful when
    // they are not.
    let mut bytes = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let path = std::env::temp_dir().join(format!("{TASK_NAME}.xml"));
    std::fs::write(&path, &bytes).with_context(|| format!("write {}", path.display()))?;
    let out = schtasks(&[
        "/Create",
        "/TN",
        TASK_NAME,
        "/XML",
        &path.display().to_string(),
        "/F",
    ]);
    let _ = std::fs::remove_file(&path);
    let out = out?;
    if !out.status.success() {
        bail!(
            "schtasks would not take the task: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

pub fn disarm_timer(_journal: &Path) -> Result<()> {
    // Nothing was armed: say so without asking Task Scheduler to do anything.
    if !schtasks(&["/Query", "/TN", TASK_NAME])?.status.success() {
        return Ok(());
    }
    let out = schtasks(&["/Delete", "/TN", TASK_NAME, "/F"])?;
    if !out.status.success() {
        bail!(
            "schtasks would not delete the task: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

struct Store(HCERTSTORE);

impl Store {
    fn open_root(scope: Scope, read_only: bool) -> Result<Self> {
        let location = match scope {
            Scope::User => CERT_SYSTEM_STORE_CURRENT_USER,
            Scope::Machine => CERT_SYSTEM_STORE_LOCAL_MACHINE,
        };
        let flags = location
            | if read_only {
                CERT_STORE_READONLY_FLAG
            } else {
                0
            };
        let name = wide("ROOT");
        let store = unsafe {
            CertOpenStore(
                CERT_STORE_PROV_SYSTEM_W,
                0,
                0,
                flags,
                name.as_ptr() as *const c_void,
            )
        };
        if store.is_null() {
            return Err(os_error("open the root store"));
        }
        Ok(Self(store))
    }

    /// The certificate in this store with the same contents as `der`, for
    /// `CertFreeCertificateContext` or `CertDeleteCertificateFromStore`.
    fn find(
        &self,
        der: &[u8],
    ) -> Result<Option<*mut windows_sys::Win32::Security::Cryptography::CERT_CONTEXT>> {
        let wanted = unsafe {
            CertCreateCertificateContext(X509_ASN_ENCODING, der.as_ptr(), der.len() as u32)
        };
        if wanted.is_null() {
            return Err(os_error("parse the certificate"));
        }
        let found = unsafe {
            CertFindCertificateInStore(
                self.0,
                X509_ASN_ENCODING,
                0,
                CERT_FIND_EXISTING,
                wanted as *const c_void,
                ptr::null(),
            )
        };
        unsafe { CertFreeCertificateContext(wanted) };
        Ok((!found.is_null()).then_some(found))
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        unsafe { CertCloseStore(self.0, 0) };
    }
}

pub fn ca_locations(_cert_pem: &Path, der: &[u8]) -> Result<Vec<Place>> {
    let mut places = Vec::new();
    // The user's root store also shows the machine's, so ask the machine first.
    let machine = Store::open_root(Scope::Machine, true)?;
    if let Some(found) = machine.find(der)? {
        unsafe { CertFreeCertificateContext(found) };
        places.push(Place {
            name: "Windows: this machine".to_owned(),
            browsers: true,
            scope: Scope::Machine,
        });
    }
    let user = Store::open_root(Scope::User, true)?;
    if let Some(found) = user.find(der)? {
        unsafe { CertFreeCertificateContext(found) };
        if places.is_empty() {
            places.push(Place {
                name: "Windows: this user".to_owned(),
                browsers: true,
                scope: Scope::User,
            });
        }
    }
    Ok(places)
}

pub fn ca_install(_cert_pem: &Path, der: &[u8], scope: Scope) -> Result<()> {
    let store = Store::open_root(scope, false)?;
    let ok = unsafe {
        CertAddEncodedCertificateToStore(
            store.0,
            X509_ASN_ENCODING,
            der.as_ptr(),
            der.len() as u32,
            CERT_STORE_ADD_REPLACE_EXISTING,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(os_error("add the certificate"));
    }
    Ok(())
}

pub fn ca_uninstall(_cert_pem: &Path, der: &[u8], scope: Scope) -> Result<()> {
    let store = Store::open_root(scope, false)?;
    let found = store
        .find(der)?
        .context("the certificate is not in that store")?;
    // This frees `found`, whether it succeeds or not.
    if unsafe { CertDeleteCertificateFromStore(found) } == 0 {
        return Err(os_error("remove the certificate"));
    }
    Ok(())
}

/// A console with the variables set, under a line saying so. The `cmd` that
/// starts it shows no window of its own.
pub fn open_terminal(env: &[(&str, String)], banner: &str, _data_dir: &Path) -> Result<()> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = std::process::Command::new("cmd");
    command
        .args(["/c", "start", "tracewing", "cmd", "/k"])
        .arg(format!("echo {banner}"))
        .envs(env.iter().map(|(name, value)| (*name, value.as_str())));
    std::os::windows::process::CommandExt::creation_flags(&mut command, CREATE_NO_WINDOW);
    if let Some(home) = std::env::var_os("USERPROFILE") {
        command.current_dir(home);
    }
    launch::spawn(command).context("open a console")
}

/// Programs, under Program Files or the user's local app data, and what they
/// are.
const BROWSERS: &[(&str, &str, Engine)] = &[
    (
        r"Google\Chrome\Application\chrome.exe",
        "Chrome",
        Engine::Chromium,
    ),
    (
        r"Microsoft\Edge\Application\msedge.exe",
        "Edge",
        Engine::Chromium,
    ),
    (
        r"BraveSoftware\Brave-Browser\Application\brave.exe",
        "Brave",
        Engine::Chromium,
    ),
    (
        r"Chromium\Application\chrome.exe",
        "Chromium",
        Engine::Chromium,
    ),
    (r"Programs\Opera\opera.exe", "Opera", Engine::Chromium),
    (r"Mozilla Firefox\firefox.exe", "Firefox", Engine::Firefox),
];

pub fn find_browsers(profiles: &Path) -> Vec<Browser> {
    let roots: Vec<std::path::PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(|var| std::env::var_os(var).map(std::path::PathBuf::from))
        .collect();
    BROWSERS
        .iter()
        .filter_map(|&(relative, name, engine)| {
            roots
                .iter()
                .map(|root| root.join(relative))
                .find(|program| program.is_file())
                .map(|program| Browser {
                    name,
                    engine,
                    program,
                    profile: profiles.join(name.to_lowercase()),
                    nss: Vec::new(),
                })
        })
        .collect()
}

/// Chromium's family reads the system's store, and so does Firefox since
/// version 120.
pub fn browser_trust(_browser: &Browser) -> BrowserTrust {
    BrowserTrust::System
}

/// A Firefox profile reads the system's store, as tracewing's sets
/// `security.enterprise_roots.enabled`.
pub fn trust_in_profile(_profile: &Path, _cert_pem: &Path) -> Result<()> {
    Ok(())
}

/// The system proxy is always there to set, and the stores need no tool.
pub fn environment() -> Environment {
    Environment::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two things the XML is there for, which `schtasks` arguments cannot
    /// say: that it runs without the user's password, and that it repeats for
    /// ever rather than for a day.
    #[test]
    fn the_task_runs_every_minute_without_a_password() {
        let command = [
            r"C:\Program Files\tracewing\tracewing.exe",
            "recover",
            "--data-dir",
            r"C:\d",
            "--only-if-gone",
        ]
        .map(str::to_owned);
        let xml = task_xml(&command, r"WORKGROUP\somebody").unwrap();
        assert!(xml.contains("<LogonType>S4U</LogonType>"), "{xml}");
        assert!(xml.contains("<Interval>PT1M</Interval>"), "{xml}");
        assert!(
            !xml.contains("<Duration>"),
            "for ever, not for a day: {xml}"
        );
        assert!(
            xml.contains(r"<UserId>WORKGROUP\somebody</UserId>"),
            "{xml}"
        );
        // The program and what follows it are two fields, so a folder with a
        // space in its name needs no quoting of the program itself.
        assert!(
            xml.contains(r"<Command>C:\Program Files\tracewing\tracewing.exe</Command>"),
            "{xml}"
        );
        assert!(
            xml.contains(r"<Arguments>recover --data-dir C:\d --only-if-gone</Arguments>"),
            "{xml}"
        );
    }

    /// A data folder may be anywhere and may be called anything, and XML has
    /// characters of its own that must go in as text.
    #[test]
    fn a_folder_named_in_xml_characters_goes_in_as_text() {
        let command = [r"C:\t.exe", "recover", "--data-dir", r"C:\a&b<c>"].map(str::to_owned);
        let xml = task_xml(&command, "WORKGROUP\\a&b").unwrap();
        assert!(xml.contains(r"C:\a&amp;b&lt;c&gt;"), "{xml}");
        assert!(xml.contains(r"<UserId>WORKGROUP\a&amp;b</UserId>"), "{xml}");
        // And a folder with a space in it is one argument, not two.
        let spaced = [r"C:\t.exe", "recover", "--data-dir", r"C:\two words"].map(str::to_owned);
        let xml = task_xml(&spaced, "me").unwrap();
        assert!(xml.contains(r"&quot;C:\two words&quot;"), "{xml}");
    }
}
