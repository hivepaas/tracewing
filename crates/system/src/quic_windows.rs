//! Windows: a filter of the Windows Filtering Platform, in a dynamic session,
//! which Windows takes away as soon as the session ends. Adding one takes an
//! administrator, so an elevated copy of this program holds it, through the
//! UAC dialog, and ends when told to or when the app it serves has gone.

use anyhow::{Context, Result, bail};
use std::{path::Path, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_CANCELLED, GetLastError, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    NetworkManagement::WindowsFilteringPlatform::{
        FWP_ACTION_BLOCK, FWP_EMPTY, FWP_MATCH_EQUAL, FWP_UINT8, FWP_UINT16,
        FWPM_CONDITION_IP_PROTOCOL, FWPM_CONDITION_IP_REMOTE_PORT, FWPM_FILTER_CONDITION0,
        FWPM_FILTER0, FWPM_LAYER_ALE_AUTH_CONNECT_V4, FWPM_LAYER_ALE_AUTH_CONNECT_V6,
        FWPM_SESSION_FLAG_DYNAMIC, FWPM_SESSION0, FwpmEngineClose0, FwpmEngineOpen0,
        FwpmFilterAdd0,
    },
    System::{
        Rpc::RPC_C_AUTHN_WINNT,
        Threading::{
            CreateEventW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, OpenProcess,
            PROCESS_SYNCHRONIZE, SYNCHRONIZATION_SYNCHRONIZE, SetEvent, WaitForMultipleObjects,
            WaitForSingleObject,
        },
    },
    UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// How long the administrator's dialog may stay open.
const ASK_LIMIT_MS: u32 = 10 * 60 * 1000;
const UDP: u8 = 17;
const HELPER_LOG: &str = "quic-helper.log";

/// The elevated helper, and the events it waits on and answers with.
pub struct QuicBlock {
    helper: HANDLE,
    stop: HANDLE,
    ready: HANDLE,
}

// SAFETY: handles are good in any thread of the process, and the block only
// uses them as it is dropped.
unsafe impl Send for QuicBlock {}

fn event_names(name: &str) -> (String, String) {
    (
        format!("Local\\{name}-ready"),
        format!("Local\\{name}-stop"),
    )
}

pub fn block_quic(data_dir: &Path) -> Result<QuicBlock> {
    let name = format!("tracewing-quic-{}", std::process::id());
    let (ready_name, stop_name) = event_names(&name);
    let log = data_dir.join(HELPER_LOG);
    let _ = std::fs::remove_file(&log);
    // SAFETY: plain calls with names that outlive them; every handle made is
    // kept in the block or closed on the way out.
    unsafe {
        let ready = CreateEventW(ptr::null(), 1, 0, wide(&ready_name).as_ptr());
        let stop = CreateEventW(ptr::null(), 1, 0, wide(&stop_name).as_ptr());
        if ready.is_null() || stop.is_null() {
            let error = GetLastError();
            close(&[ready, stop]);
            bail!("make the events for the helper: error {error}");
        }
        let exe = std::env::current_exe().context("find this program")?;
        let exe = wide(&exe.display().to_string());
        let parameters = wide(&format!(
            "block-quic --parent {} --name {name} --data-dir \"{}\"",
            std::process::id(),
            data_dir.display()
        ));
        let verb = wide("runas");
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = exe.as_ptr();
        info.lpParameters = parameters.as_ptr();
        // SW_HIDE: the helper has nothing to show.
        info.nShow = 0;
        if ShellExecuteExW(&mut info) == 0 {
            let error = GetLastError();
            close(&[ready, stop]);
            if error == ERROR_CANCELLED {
                bail!("administrator rights were not given");
            }
            bail!("start the helper as administrator: error {error}");
        }
        let helper = info.hProcess;
        let waits = [ready, helper];
        match WaitForMultipleObjects(2, waits.as_ptr(), 0, ASK_LIMIT_MS) {
            WAIT_OBJECT_0 => Ok(QuicBlock {
                helper,
                stop,
                ready,
            }),
            WAIT_TIMEOUT => {
                SetEvent(stop);
                close(&[ready, stop, helper]);
                bail!("the helper did not answer");
            }
            _ => {
                close(&[ready, stop, helper]);
                let said = std::fs::read_to_string(&log).unwrap_or_default();
                bail!("the helper stopped: {}", said.trim());
            }
        }
    }
}

impl QuicBlock {
    pub fn release(self) -> Result<()> {
        // Drop does it all.
        Ok(())
    }
}

impl Drop for QuicBlock {
    fn drop(&mut self) {
        // SAFETY: the handles are this block's own.
        unsafe {
            SetEvent(self.stop);
            WaitForSingleObject(self.helper, 5_000);
            close(&[self.ready, self.stop, self.helper]);
        }
    }
}

unsafe fn close(handles: &[HANDLE]) {
    for handle in handles {
        if !handle.is_null() {
            // SAFETY: each is a handle this program opened and has not closed.
            unsafe { CloseHandle(*handle) };
        }
    }
}

/// The elevated half: `block-quic --parent PID --name NAME --data-dir DIR`.
/// Holds the filters until the app says stop or goes away.
pub fn quic_helper(parent: u32, name: &str, data_dir: &Path) -> Result<()> {
    let result = hold_filters(parent, name);
    if let Err(e) = &result {
        let _ = std::fs::write(data_dir.join(HELPER_LOG), format!("{e:#}"));
    }
    result
}

fn hold_filters(parent: u32, name: &str) -> Result<()> {
    let (ready_name, stop_name) = event_names(name);
    // SAFETY: plain calls; the structures handed over live past each call, and
    // every handle opened is closed before this returns.
    unsafe {
        let parent = OpenProcess(PROCESS_SYNCHRONIZE, 0, parent);
        let ready = OpenEventW(EVENT_MODIFY_STATE, 0, wide(&ready_name).as_ptr());
        let stop = OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, 0, wide(&stop_name).as_ptr());
        if parent.is_null() || ready.is_null() || stop.is_null() {
            let error = GetLastError();
            close(&[parent, ready, stop]);
            bail!("find the app that asked: error {error}");
        }
        let mut session: FWPM_SESSION0 = std::mem::zeroed();
        // Its filters go with it, however it ends.
        session.flags = FWPM_SESSION_FLAG_DYNAMIC;
        let mut engine: HANDLE = ptr::null_mut();
        let opened = FwpmEngineOpen0(
            ptr::null(),
            RPC_C_AUTHN_WINNT,
            ptr::null(),
            &session,
            &mut engine,
        );
        if opened != 0 {
            close(&[parent, ready, stop]);
            bail!("open the Windows Filtering Platform: error {opened:#x}");
        }
        let added = add_filters(engine);
        if let Err(e) = added {
            FwpmEngineClose0(engine);
            close(&[parent, ready, stop]);
            return Err(e);
        }
        SetEvent(ready);
        let waits = [parent, stop];
        WaitForMultipleObjects(2, waits.as_ptr(), 0, INFINITE);
        FwpmEngineClose0(engine);
        close(&[parent, ready, stop]);
    }
    Ok(())
}

/// Outgoing UDP to port 443, over IPv4 and IPv6, refused as it starts.
unsafe fn add_filters(engine: HANDLE) -> Result<()> {
    let mut name = wide("tracewing: keep apps off HTTP/3");
    for layer in [
        FWPM_LAYER_ALE_AUTH_CONNECT_V4,
        FWPM_LAYER_ALE_AUTH_CONNECT_V6,
    ] {
        // SAFETY: zeroed is how these structures start; the fields set are
        // those the filter needs, and the conditions outlive the call.
        unsafe {
            let mut conditions: [FWPM_FILTER_CONDITION0; 2] = std::mem::zeroed();
            conditions[0].fieldKey = FWPM_CONDITION_IP_PROTOCOL;
            conditions[0].matchType = FWP_MATCH_EQUAL;
            conditions[0].conditionValue.r#type = FWP_UINT8;
            conditions[0].conditionValue.Anonymous.uint8 = UDP;
            conditions[1].fieldKey = FWPM_CONDITION_IP_REMOTE_PORT;
            conditions[1].matchType = FWP_MATCH_EQUAL;
            conditions[1].conditionValue.r#type = FWP_UINT16;
            conditions[1].conditionValue.Anonymous.uint16 = 443;
            let mut filter: FWPM_FILTER0 = std::mem::zeroed();
            filter.displayData.name = name.as_mut_ptr();
            filter.layerKey = layer;
            filter.action.r#type = FWP_ACTION_BLOCK;
            filter.weight.r#type = FWP_EMPTY;
            filter.numFilterConditions = conditions.len() as u32;
            filter.filterCondition = conditions.as_mut_ptr();
            let added = FwpmFilterAdd0(engine, &filter, ptr::null_mut(), ptr::null_mut());
            if added != 0 {
                bail!("add the filter: error {added:#x}");
            }
        }
    }
    Ok(())
}
