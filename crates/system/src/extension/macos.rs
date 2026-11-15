//! The macOS calls behind [`super`]: `OSSystemExtensionRequest` to install
//! and remove the extension, `NETransparentProxyManager` for its
//! configuration.
//!
//! Every answer comes in a completion block or a delegate method, and each
//! call here waits for it on a channel. Blocks handed to these classes need
//! no signature, unlike those the extension hands across NSXPC: macOS calls
//! them in this process.

use super::{Installed, Running, Status, WHERE_TO_ALLOW, install_problem};
use anyhow::{Context, Result, anyhow, bail};
use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::{
    AllocAnyThread, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_foundation::{NSArray, NSDictionary, NSError, NSString, NSThread};
use objc2_network_extension::{NETransparentProxyManager, NETunnelProviderProtocol, NEVPNStatus};
use objc2_system_extensions::{
    OSSystemExtensionManager, OSSystemExtensionProperties, OSSystemExtensionReplacementAction,
    OSSystemExtensionRequest, OSSystemExtensionRequestDelegate, OSSystemExtensionRequestResult,
};
use std::{
    ffi::c_void,
    sync::{Mutex, OnceLock, mpsc},
    time::{Duration, Instant},
};
use tracewing_netext::flow::{Config, EXTENSION_ID};

/// How long the person has to allow the extension, or the configuration.
const APPROVAL_WITHIN: Duration = Duration::from_secs(10 * 60);
/// How long macOS has to answer anything else.
const ANSWER_WITHIN: Duration = Duration::from_secs(30);
/// How long the proxy has to start, or to stop.
const RUNNING_WITHIN: Duration = Duration::from_secs(15);
/// What the configuration is called in System Settings › VPN.
const NAME: &str = "tracewing";

/// A value of macOS's to send from the thread its answer came on to the one
/// waiting. These are used from any thread in Apple's own samples.
struct Sendable<T>(T);
unsafe impl<T> Send for Sendable<T> {}

fn not_on_main_thread() -> Result<()> {
    if NSThread::isMainThread_class() {
        bail!("tracewing asked macOS from its main thread, where the answer would never come");
    }
    Ok(())
}

fn said(error: &NSError) -> String {
    format!(
        "{} ({} {})",
        error.localizedDescription(),
        error.domain(),
        error.code()
    )
}

// ---- The extension ----

enum Event {
    Waiting,
    Done,
    Failed { code: isize, said: String },
    Found(Vec<(bool, bool)>),
}

/// Where the delegate sends what macOS says about the request in hand. One
/// request at a time: [`REQUEST`] is held while one is.
static EVENTS: Mutex<Option<mpsc::Sender<Event>>> = Mutex::new(None);
static REQUEST: Mutex<()> = Mutex::new(());

fn event(event: Event) {
    if let Some(events) = EVENTS.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = events.send(event);
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TracewingExtensionRequest"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl OSSystemExtensionRequestDelegate for Delegate {
        #[unsafe(method(request:actionForReplacingExtension:withExtension:))]
        fn replacing(
            &self,
            _request: &OSSystemExtensionRequest,
            _old: &OSSystemExtensionProperties,
            _new: &OSSystemExtensionProperties,
        ) -> OSSystemExtensionReplacementAction {
            // The app's own extension is the one that goes with it, older or
            // newer: one left from another version would not speak its port.
            OSSystemExtensionReplacementAction::Replace
        }

        #[unsafe(method(requestNeedsUserApproval:))]
        fn needs_approval(&self, _request: &OSSystemExtensionRequest) {
            event(Event::Waiting);
        }

        #[unsafe(method(request:didFinishWithResult:))]
        fn finished(
            &self,
            _request: &OSSystemExtensionRequest,
            _result: OSSystemExtensionRequestResult,
        ) {
            event(Event::Done);
        }

        #[unsafe(method(request:didFailWithError:))]
        fn failed(&self, _request: &OSSystemExtensionRequest, error: &NSError) {
            event(Event::Failed {
                code: error.code(),
                said: said(error),
            });
        }

        #[unsafe(method(request:foundProperties:))]
        fn found(
            &self,
            _request: &OSSystemExtensionRequest,
            properties: &NSArray<OSSystemExtensionProperties>,
        ) {
            let found = properties
                .to_vec()
                .into_iter()
                .map(|p| unsafe { (p.isEnabled(), p.isAwaitingUserApproval()) })
                .collect();
            event(Event::Found(found));
        }
    }
);

/// The delegate, made once and kept: a request holds its delegate weakly.
fn delegate() -> &'static Retained<Delegate> {
    static DELEGATE: OnceLock<Sendable<Retained<Delegate>>> = OnceLock::new();
    &DELEGATE
        .get_or_init(|| Sendable(unsafe { msg_send![Delegate::alloc(), init] }))
        .0
}

/// Submits the request `make` makes, and hands back where its events come.
/// Its delegate is called on a queue of its own, so this needs no main thread.
fn submit(
    make: impl FnOnce(&NSString, &DispatchQueue) -> Retained<OSSystemExtensionRequest>,
) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    *EVENTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
    let queue = DispatchQueue::new("com.hivepaas.tracewing.extension", None);
    let request = make(&NSString::from_str(EXTENSION_ID), &queue);
    unsafe {
        request.setDelegate(Some(ProtocolObject::from_ref(&**delegate())));
        OSSystemExtensionManager::sharedManager().submitRequest(&request);
    }
    rx
}

/// Waits for the request to finish, saying when macOS waits for the person.
fn finish(events: mpsc::Receiver<Event>, waiting: &dyn Fn(), what: &str) -> Result<()> {
    let mut within = ANSWER_WITHIN;
    loop {
        match events.recv_timeout(within) {
            Ok(Event::Waiting) => {
                waiting();
                within = APPROVAL_WITHIN;
            }
            Ok(Event::Done) => return Ok(()),
            Ok(Event::Failed { code, said }) => match install_problem(code) {
                Some(problem) => bail!("{what}: {problem} — macOS says {said}"),
                None => bail!("{what}: macOS says {said}"),
            },
            Ok(Event::Found(_)) => {}
            Err(_) if within == APPROVAL_WITHIN => {
                bail!("{what}: it was not allowed in {within:?} ({WHERE_TO_ALLOW})")
            }
            Err(_) => bail!("{what}: no answer from macOS in {within:?}"),
        }
    }
}

pub fn install(waiting: &dyn Fn()) -> Result<()> {
    let _one = REQUEST.lock().unwrap_or_else(|e| e.into_inner());
    let events = submit(|id, queue| unsafe {
        OSSystemExtensionRequest::activationRequestForExtension_queue(id, queue)
    });
    finish(events, waiting, "installing tracewing's network extension")
}

fn installed() -> Result<Installed> {
    let _one = REQUEST.lock().unwrap_or_else(|e| e.into_inner());
    let events = submit(|id, queue| unsafe {
        OSSystemExtensionRequest::propertiesRequestForExtension_queue(id, queue)
    });
    match events.recv_timeout(ANSWER_WITHIN) {
        Ok(Event::Found(found)) => Ok(if found.iter().any(|&(enabled, _)| enabled) {
            Installed::Yes
        } else if found.iter().any(|&(_, waiting)| waiting) {
            Installed::WaitingForApproval
        } else {
            Installed::No
        }),
        Ok(Event::Failed { said, .. }) => bail!("asking macOS about the extension: {said}"),
        Ok(_) => bail!("asking macOS about the extension: an answer to another question"),
        Err(_) => bail!("asking macOS about the extension: no answer in {ANSWER_WITHIN:?}"),
    }
}

// ---- The configuration ----

/// Waits for a completion block that hands over `T`, or an error.
fn wait<T>(
    within: Duration,
    what: &str,
    ask: impl FnOnce(mpsc::Sender<Result<T, String>>),
) -> Result<T> {
    let (tx, rx) = mpsc::channel();
    ask(tx);
    rx.recv_timeout(within)
        .map_err(|_| anyhow!("{what}: no answer from macOS in {within:?}"))?
        .map_err(|e| anyhow!("{what}: macOS says {e}"))
}

/// tracewing's own configuration, of those this app saved: one, normally.
fn ours() -> Result<Option<Retained<NETransparentProxyManager>>> {
    let all = wait(
        ANSWER_WITHIN,
        "reading tracewing's proxy configuration",
        |tx| {
            let block = RcBlock::new(
                move |list: *mut NSArray<NETransparentProxyManager>, error: *mut NSError| {
                    let answer = match unsafe { error.as_ref() } {
                        Some(error) => Err(said(error)),
                        None => Ok(Sendable(
                            unsafe { list.as_ref() }
                                .map(|l| l.to_vec())
                                .unwrap_or_default(),
                        )),
                    };
                    let _ = tx.send(answer);
                },
            );
            unsafe {
                NETransparentProxyManager::loadAllFromPreferencesWithCompletionHandler(&block)
            };
        },
    )?
    .0;
    Ok(all.into_iter().find(|manager| {
        unsafe { manager.protocolConfiguration() }
            .and_then(|p| {
                p.downcast_ref::<NETunnelProviderProtocol>()
                    .and_then(|p| unsafe { p.providerBundleIdentifier() })
            })
            .is_some_and(|id| id.to_string() == EXTENSION_ID)
    }))
}

fn save(manager: &NETransparentProxyManager) -> Result<()> {
    // The first save asks the person to allow it, so it may take a while.
    wait(
        APPROVAL_WITHIN,
        "saving tracewing's proxy configuration",
        |tx| {
            let block = RcBlock::new(move |error: *mut NSError| {
                let _ = tx.send(match unsafe { error.as_ref() } {
                    Some(error) => Err(said(error)),
                    None => Ok(()),
                });
            });
            unsafe { manager.saveToPreferencesWithCompletionHandler(Some(&block)) };
        },
    )
}

fn reload(manager: &NETransparentProxyManager) -> Result<()> {
    wait(
        ANSWER_WITHIN,
        "reading tracewing's proxy configuration",
        |tx| {
            let block = RcBlock::new(move |error: *mut NSError| {
                let _ = tx.send(match unsafe { error.as_ref() } {
                    Some(error) => Err(said(error)),
                    None => Ok(()),
                });
            });
            unsafe { manager.loadFromPreferencesWithCompletionHandler(&block) };
        },
    )
}

fn running(status: NEVPNStatus) -> Running {
    match status.0 {
        1 => Running::Off,
        2 | 4 => Running::Starting,
        3 => Running::On,
        5 => Running::Stopping,
        _ => Running::None,
    }
}

fn running_of(manager: &NETransparentProxyManager) -> Running {
    running(unsafe { manager.connection().status() })
}

/// Waits for the proxy to come to `wanted`, and says where it got to if not.
fn until(manager: &NETransparentProxyManager, wanted: &[Running], what: &str) -> Result<()> {
    let deadline = Instant::now() + RUNNING_WITHIN;
    loop {
        let now = running_of(manager);
        if wanted.contains(&now) {
            return Ok(());
        }
        if Instant::now() > deadline {
            bail!("{what}: it is {now:?} after {RUNNING_WITHIN:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub fn start(config: &Config) -> Result<()> {
    not_on_main_thread()?;
    let manager = match ours()? {
        Some(manager) => manager,
        None => unsafe { NETransparentProxyManager::new() },
    };
    let saved = config.to_saved();
    let keys: Vec<Retained<NSString>> = saved.iter().map(|(k, _)| NSString::from_str(k)).collect();
    let values: Vec<Retained<NSString>> =
        saved.iter().map(|(_, v)| NSString::from_str(v)).collect();
    let keys: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
    let saved = NSDictionary::from_retained_objects(&keys, &values);
    // Every value is a string, which is an object.
    let saved: Retained<NSDictionary<NSString, AnyObject>> =
        unsafe { Retained::cast_unchecked(saved) };
    unsafe {
        let protocol = NETunnelProviderProtocol::new();
        protocol.setProviderBundleIdentifier(Some(&NSString::from_str(EXTENSION_ID)));
        protocol.setServerAddress(Some(&NSString::from_str("127.0.0.1")));
        protocol.setProviderConfiguration(Some(&saved));
        manager.setProtocolConfiguration(Some(&protocol));
        manager.setLocalizedDescription(Some(&NSString::from_str(NAME)));
        manager.setEnabled(true);
    }
    save(&manager)?;
    reload(&manager)?;
    // One already running has the last run's port and key: it starts again
    // with these.
    if matches!(running_of(&manager), Running::Starting | Running::On) {
        unsafe { manager.connection().stopVPNTunnel() };
        until(
            &manager,
            &[Running::Off, Running::None],
            "stopping the proxy that ran",
        )?;
    }
    unsafe { manager.connection().startVPNTunnelAndReturnError() }
        .map_err(|e| anyhow!("starting tracewing's proxy: macOS says {}", said(&e)))?;
    until(&manager, &[Running::On], "starting tracewing's proxy")
}

pub fn stop() -> Result<()> {
    not_on_main_thread()?;
    let Some(manager) = ours()? else {
        return Ok(());
    };
    if matches!(running_of(&manager), Running::Starting | Running::On) {
        unsafe { manager.connection().stopVPNTunnel() };
        until(
            &manager,
            &[Running::Off, Running::None],
            "stopping tracewing's proxy",
        )?;
    }
    Ok(())
}

pub fn status() -> Result<Status> {
    not_on_main_thread()?;
    Ok(Status {
        installed: if super::in_app_bundle() {
            installed()?
        } else {
            Installed::Unknown
        },
        running: ours()?.map_or(Running::None, |m| running_of(&m)),
    })
}

pub fn uninstall(waiting: &dyn Fn()) -> Result<()> {
    not_on_main_thread()?;
    if let Some(manager) = ours()? {
        if matches!(running_of(&manager), Running::Starting | Running::On) {
            unsafe { manager.connection().stopVPNTunnel() };
        }
        wait(
            ANSWER_WITHIN,
            "removing tracewing's proxy configuration",
            |tx| {
                let block = RcBlock::new(move |error: *mut NSError| {
                    let _ = tx.send(match unsafe { error.as_ref() } {
                        Some(error) => Err(said(error)),
                        None => Ok(()),
                    });
                });
                unsafe { manager.removeFromPreferencesWithCompletionHandler(Some(&block)) };
            },
        )?;
    }
    let _one = REQUEST.lock().unwrap_or_else(|e| e.into_inner());
    let events = submit(|id, queue| unsafe {
        OSSystemExtensionRequest::deactivationRequestForExtension_queue(id, queue)
    });
    finish(events, waiting, "removing tracewing's network extension")
        .context("the configuration is gone; the extension can also be removed in System Settings")
}

// ---- The main thread ----

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopDefaultMode: *const c_void;
    fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: u8) -> i32;
}

pub fn serve_main_thread_while<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    if !NSThread::isMainThread_class() {
        return work();
    }
    std::thread::scope(|scope| {
        let worker = scope.spawn(work);
        // The main run loop drains the main queue, where macOS's answers come.
        while !worker.is_finished() {
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, 0) };
        }
        match worker.join() {
            Ok(done) => done,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}
