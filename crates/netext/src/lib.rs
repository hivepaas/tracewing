//! tracewing's network extension for macOS: a transparent proxy
//! (`NETransparentProxyProvider`) in a system extension, which hands the flows
//! of the apps chosen to tracewing's listener, so that they come through with
//! no proxy setting of their own.
//!
//! [`flow`] is what it decides and says, on every system; [`provider`] is the
//! extension itself, on macOS.

pub mod flow;
#[cfg(target_os = "macos")]
pub mod provider;
