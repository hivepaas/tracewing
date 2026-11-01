//! A flow as a command to run it again.

/// How much of a body goes on the command line. Past this, it is left out and the
/// command says so: nobody pastes more.
const BODY_MAX: usize = 256 * 1024;

/// Hop-by-hop headers and ones curl works out itself.
const LEFT_OUT: [&str; 6] = [
    "content-length",
    "host",
    "connection",
    "proxy-connection",
    "accept-encoding",
    "transfer-encoding",
];

fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub fn command(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> String {
    let mut parts = vec![format!("curl {}", quoted(url))];
    if method != "GET" {
        parts.push(format!("-X {}", quoted(method)));
    }
    for (name, value) in headers {
        if !LEFT_OUT.contains(&name.to_ascii_lowercase().as_str()) {
            parts.push(format!("-H {}", quoted(&format!("{name}: {value}"))));
        }
    }
    parts.push("--compressed".to_owned());
    match body {
        Some([]) | None => {}
        Some(body) if body.len() > BODY_MAX => {
            parts.push(format!("# body of {} bytes left out", body.len()));
        }
        Some(body) => match std::str::from_utf8(body) {
            Ok(text) => parts.push(format!("--data-raw {}", quoted(text))),
            Err(_) => parts.push(format!("# binary body of {} bytes left out", body.len())),
        },
    }
    parts.join(" \\\n  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_post_with_everything() {
        let headers = vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("host".to_owned(), "a.dev".to_owned()),
            ("x-note".to_owned(), "it's here".to_owned()),
        ];
        assert_eq!(
            command("POST", "https://a.dev/v1?x=1", &headers, Some(b"{\"a\":1}")),
            "curl 'https://a.dev/v1?x=1' \\\n  -X 'POST' \\\n  -H 'content-type: application/json' \\\n  -H 'x-note: it'\\''s here' \\\n  --compressed \\\n  --data-raw '{\"a\":1}'"
        );
    }

    #[test]
    fn a_get_is_short() {
        assert_eq!(
            command("GET", "https://a.dev/", &[], None),
            "curl 'https://a.dev/' \\\n  --compressed"
        );
    }

    #[test]
    fn what_cannot_go_on_a_command_line_says_so() {
        let out = command("POST", "https://a.dev/", &[], Some(&[0xff, 0xfe]));
        assert!(out.ends_with("# binary body of 2 bytes left out"), "{out}");
        let big = vec![b'x'; BODY_MAX + 1];
        let out = command("POST", "https://a.dev/", &[], Some(&big));
        assert!(out.contains("left out"), "cut");
    }
}
