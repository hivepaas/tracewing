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

use super::{Saved, Scope};

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

pub fn ca_locations(_cert_pem: &Path, der: &[u8]) -> Result<Vec<String>> {
    let mut places = Vec::new();
    // The user's root store also shows the machine's, so ask the machine first.
    let machine = Store::open_root(Scope::Machine, true)?;
    if let Some(found) = machine.find(der)? {
        unsafe { CertFreeCertificateContext(found) };
        places.push("Windows: this machine".to_owned());
    }
    let user = Store::open_root(Scope::User, true)?;
    if let Some(found) = user.find(der)? {
        unsafe { CertFreeCertificateContext(found) };
        if places.is_empty() {
            places.push("Windows: this user".to_owned());
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
