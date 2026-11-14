//! The transparent proxy itself: the class macOS starts in the extension's
//! process, and the two directions of each flow it takes.
//!
//! macOS hands over every outbound TCP flow of the computer. The ones from the
//! apps the configuration names go to tracewing's listener on 127.0.0.1, as a
//! `CONNECT` to where the app was going, and tracewing treats them as any
//! client of its proxy. Every other flow, and every flow while nothing listens
//! on tracewing's port, is handed back and goes out as it would have: the
//! extension never holds the network up.

use std::{
    ffi::CStr,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use block2::{DynBlock, ManualBlockEncoding, RcBlock};
use log::{debug, info, warn};
use objc2::{
    AllocAnyThread, ClassType, Message, define_class,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol},
};
use objc2_foundation::{NSArray, NSData, NSDictionary, NSError, NSString};
#[allow(deprecated)] // NWHostEndpoint: the flow's endpoint is still one.
use objc2_network_extension::{
    NEAppProxyFlow, NEAppProxyProvider, NEAppProxyTCPFlow, NENetworkRule, NENetworkRuleProtocol,
    NEProvider, NEProviderStopReason, NETrafficDirection, NETransparentProxyNetworkSettings,
    NETransparentProxyProvider, NETunnelProvider, NETunnelProviderProtocol, NWHostEndpoint,
};

use crate::flow::{self, Config};

/// How long tracewing has to accept a connection before the flow goes out
/// straight instead: it is on this computer, so it answers at once or not at all.
const CONNECT_WAIT: Duration = Duration::from_millis(300);
/// How long tracewing has to answer the `CONNECT`.
const ANSWER_WAIT: Duration = Duration::from_secs(10);
/// The longest answer head read.
const HEAD_MAX: usize = 8 * 1024;
const BUFFER: usize = 64 * 1024;

/// The configuration the proxy was started with; none when stopped.
static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

/// The class name the extension's Info.plist gives under
/// `NetworkExtension.NEProviderClasses`.
pub const CLASS: &str = "TracewingProxy";

define_class!(
    #[unsafe(super(NETransparentProxyProvider, NEAppProxyProvider, NETunnelProvider, NEProvider, NSObject))]
    #[name = "TracewingProxy"]
    struct TracewingProxy;

    unsafe impl NSObjectProtocol for TracewingProxy {}

    impl TracewingProxy {
        #[unsafe(method(startProxyWithOptions:completionHandler:))]
        fn start_proxy(
            &self,
            _options: Option<&NSDictionary<NSString, AnyObject>>,
            done: &DynBlock<dyn Fn(*mut NSError)>,
        ) {
            start(self, done);
        }

        #[unsafe(method(stopProxyWithReason:completionHandler:))]
        fn stop_proxy(&self, reason: NEProviderStopReason, done: &DynBlock<dyn Fn()>) {
            info!("stopped (reason {})", reason.0);
            *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = None;
            done.call(());
        }

        #[unsafe(method(handleNewFlow:))]
        fn handle_new_flow(&self, flow: &NEAppProxyFlow) -> bool {
            take(flow)
        }
    }
);

/// macOS passes a completion block across NSXPC, which refuses one without a
/// signature, and a block made in Rust has none unless given one.
struct ErrorArg;
// SAFETY: the signature of a block taking one object and returning nothing.
unsafe impl ManualBlockEncoding for ErrorArg {
    type Arguments = (*mut NSError,);
    type Return = ();
    const ENCODING_CSTR: &'static CStr = c"v16@?0@\"NSError\"8";
}

struct DataErrorArgs;
// SAFETY: the signature of a block taking two objects and returning nothing.
unsafe impl ManualBlockEncoding for DataErrorArgs {
    type Arguments = (*mut NSData, *mut NSError);
    type Return = ();
    const ENCODING_CSTR: &'static CStr = c"v24@?0@\"NSData\"8@\"NSError\"16";
}

fn on_error(f: impl Fn(*mut NSError) + 'static) -> RcBlock<dyn Fn(*mut NSError)> {
    RcBlock::with_encoding::<_, _, _, ErrorArg>(f)
}

fn on_data(
    f: impl Fn(*mut NSData, *mut NSError) + 'static,
) -> RcBlock<dyn Fn(*mut NSData, *mut NSError)> {
    RcBlock::with_encoding::<_, _, _, DataErrorArgs>(f)
}

fn configuration(provider: &TracewingProxy) -> Option<Config> {
    let protocol = unsafe { provider.protocolConfiguration() };
    let protocol = protocol.downcast_ref::<NETunnelProviderProtocol>()?;
    let saved = unsafe { protocol.providerConfiguration() }?;
    let string = |key: &str| -> Option<String> {
        let value = saved.objectForKey(&NSString::from_str(key))?;
        Some(value.downcast_ref::<NSString>()?.to_string())
    };
    Config::parse(
        &string(flow::PORT_KEY)?,
        &string(flow::APPS_KEY).unwrap_or_default(),
    )
}

fn start(provider: &TracewingProxy, done: &DynBlock<dyn Fn(*mut NSError)>) {
    let Some(config) = configuration(provider) else {
        warn!("not started: the configuration names no port");
        let error = NSError::new(1, &NSString::from_str(flow::EXTENSION_ID));
        done.call((Retained::as_ptr(&error) as *mut NSError,));
        return;
    };
    info!("starting: port {}, apps {:?}", config.port, config.apps);
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = Some(config);

    // Every outbound TCP flow is offered; `take` chooses.
    let settings = unsafe {
        NETransparentProxyNetworkSettings::initWithTunnelRemoteAddress(
            NETransparentProxyNetworkSettings::alloc(),
            &NSString::from_str("127.0.0.1"),
        )
    };
    #[allow(deprecated)]
    let every_flow = unsafe {
        NENetworkRule::initWithRemoteNetwork_remotePrefix_localNetwork_localPrefix_protocol_direction(
            NENetworkRule::alloc(),
            None,
            0,
            None,
            0,
            NENetworkRuleProtocol::TCP,
            NETrafficDirection::Outbound,
        )
    };
    unsafe { settings.setIncludedNetworkRules(Some(&NSArray::from_retained_slice(&[every_flow]))) };

    let done = done.copy();
    let applied = on_error(move |error: *mut NSError| {
        match unsafe { error.as_ref() } {
            None => info!("started"),
            Some(e) => warn!("not started: {e:?}"),
        }
        done.call((error,));
    });
    unsafe { provider.setTunnelNetworkSettings_completionHandler(Some(&settings), Some(&applied)) };
}

/// A flow, to be used from the threads that carry its data.
#[derive(Clone)]
struct Flow(Retained<NEAppProxyTCPFlow>);
// SAFETY: NEAppProxyFlow's methods may be called from any thread; Apple's own
// samples read and write a flow from whatever queue they are on.
unsafe impl Send for Flow {}

impl Flow {
    fn close(&self) {
        unsafe {
            self.0.closeReadWithError(None);
            self.0.closeWriteWithError(None);
        }
    }
}

/// Whether the flow goes to tracewing: answered at once, as macOS waits for it.
#[allow(deprecated)] // NWHostEndpoint
fn take(flow: &NEAppProxyFlow) -> bool {
    let Some(tcp) = flow.downcast_ref::<NEAppProxyTCPFlow>() else {
        return false;
    };
    let Some(config) = CONFIG.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
        return false;
    };
    let meta = unsafe { flow.metaData() };
    let app = unsafe { meta.sourceAppSigningIdentifier() }.to_string();
    let endpoint = unsafe { tcp.remoteEndpoint() };
    let Some(endpoint) = endpoint.downcast_ref::<NWHostEndpoint>() else {
        return false;
    };
    let ip = unsafe { endpoint.hostname() }.to_string();
    let Ok(port) = unsafe { endpoint.port() }.to_string().parse::<u16>() else {
        return false;
    };
    if !config.takes(&app, &ip, port) {
        return false;
    }

    let to = SocketAddr::from(([127, 0, 0, 1], config.port));
    let socket = match TcpStream::connect_timeout(&to, CONNECT_WAIT) {
        Ok(socket) => socket,
        Err(e) => {
            debug!("{app} to {ip} port {port}: nothing on {to} ({e}), so straight out");
            return false;
        }
    };
    let pid = unsafe { meta.sourceAppAuditToken() }.and_then(|t| flow::pid_of(&t.to_vec()));
    let name = unsafe { flow.remoteHostname() }.map(|n| n.to_string());
    let request = flow::connect_request(flow::host(name.as_deref(), &ip), port, &app, pid);
    debug!(
        "{app} ({pid:?}) to {} port {port}: to tracewing",
        name.as_deref().unwrap_or(&ip)
    );
    let flow = Flow(tcp.retain());
    thread::spawn(move || carry(flow, socket, request));
    true
}

/// Opens the tunnel through tracewing, then the flow, then carries the bytes:
/// the app's on a thread of their own, the server's on this one.
fn carry(flow: Flow, mut socket: TcpStream, request: String) {
    if let Err(e) = open_tunnel(&mut socket, &request) {
        warn!("tracewing did not open the tunnel: {e}");
        return flow.close();
    }
    let (opened_tx, opened) = mpsc::sync_channel(1);
    let opened_block = on_error(move |error: *mut NSError| {
        let _ = opened_tx.send(error.is_null());
    });
    #[allow(deprecated)] // openWithLocalFlowEndpoint is macOS 15 and later.
    unsafe {
        flow.0
            .openWithLocalEndpoint_completionHandler(None, &opened_block)
    };
    if !opened.recv().unwrap_or(false) {
        warn!("the flow did not open");
        return flow.close();
    }

    let socket = Arc::new(socket);
    let (app_flow, app_socket) = (flow.clone(), socket.clone());
    thread::spawn(move || from_app(app_flow, app_socket));

    let mut buffer = vec![0u8; BUFFER];
    loop {
        let n = match (&*socket).read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let data = NSData::with_bytes(&buffer[..n]);
        let (written_tx, written) = mpsc::sync_channel(1);
        let written_block = on_error(move |error: *mut NSError| {
            let _ = written_tx.send(error.is_null());
        });
        unsafe {
            flow.0
                .writeData_withCompletionHandler(&data, &written_block)
        };
        if !written.recv().unwrap_or(false) {
            break;
        }
    }
    unsafe { flow.0.closeWriteWithError(None) };
    let _ = socket.shutdown(Shutdown::Read);
}

fn open_tunnel(socket: &mut TcpStream, request: &str) -> std::io::Result<()> {
    socket.set_read_timeout(Some(ANSWER_WAIT))?;
    socket.write_all(request.as_bytes())?;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if socket.read(&mut byte)? == 0 || head.len() >= HEAD_MAX {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        head.push(byte[0]);
    }
    socket.set_read_timeout(None)?;
    if !flow::tunnel_opened(&head) {
        let line = String::from_utf8_lossy(head.split(|&b| b == b'\r').next().unwrap_or_default())
            .into_owned();
        return Err(std::io::Error::other(line));
    }
    Ok(())
}

/// The app's bytes to tracewing, one read of the flow at a time.
fn from_app(flow: Flow, socket: Arc<TcpStream>) {
    loop {
        let (read_tx, read) = mpsc::sync_channel(1);
        let read_block = on_data(move |data: *mut NSData, error: *mut NSError| {
            let bytes = match (unsafe { data.as_ref() }, error.is_null()) {
                (Some(data), true) => data.to_vec(),
                _ => Vec::new(),
            };
            let _ = read_tx.send(bytes);
        });
        unsafe { flow.0.readDataWithCompletionHandler(&read_block) };
        match read.recv() {
            Ok(bytes) if !bytes.is_empty() => {
                if (&*socket).write_all(&bytes).is_err() {
                    break;
                }
            }
            _ => break,
        }
    }
    let _ = socket.shutdown(Shutdown::Write);
    unsafe { flow.0.closeReadWithError(None) };
}

unsafe extern "C" {
    fn dispatch_main() -> !;
}

/// The extension's process: the class registered, then handed to macOS.
pub fn run() -> ! {
    oslog::OsLogger::new(flow::EXTENSION_ID)
        .level_filter(log::LevelFilter::Debug)
        .init()
        .ok();
    info!("the extension's process started");
    let _ = TracewingProxy::class();
    unsafe { NEProvider::startSystemExtensionMode() };
    unsafe { dispatch_main() }
}
