//! JSON Web Tokens where the app shows them.
//!
//! The reading itself is in `tracewing-convert`, which holds no clock; here
//! is the clock it does without.

pub use tracewing_convert::jwt::{Jwt, time_label};
pub use tracewing_convert::relative;

/// `2026-11-07 18:00:00` in local time.
pub fn local_time(at: i64) -> String {
    chrono::DateTime::from_timestamp(at, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| at.to_string())
}
