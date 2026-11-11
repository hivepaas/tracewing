//! gRPC's status codes, as its answers carry them.

/// What a code is called, or `None` for one gRPC has no name for.
pub fn code_name(code: u16) -> Option<&'static str> {
    Some(match code {
        0 => "OK",
        1 => "CANCELLED",
        2 => "UNKNOWN",
        3 => "INVALID_ARGUMENT",
        4 => "DEADLINE_EXCEEDED",
        5 => "NOT_FOUND",
        6 => "ALREADY_EXISTS",
        7 => "PERMISSION_DENIED",
        8 => "RESOURCE_EXHAUSTED",
        9 => "FAILED_PRECONDITION",
        10 => "ABORTED",
        11 => "OUT_OF_RANGE",
        12 => "UNIMPLEMENTED",
        13 => "INTERNAL",
        14 => "UNAVAILABLE",
        15 => "DATA_LOSS",
        16 => "UNAUTHENTICATED",
        _ => return None,
    })
}

/// The code a name stands for, in any case.
pub fn code_of(name: &str) -> Option<u16> {
    (0..=16).find(|&code| code_name(code).is_some_and(|known| known.eq_ignore_ascii_case(name)))
}

/// `NOT_FOUND (5)`, or the number alone for a code with no name.
pub fn code_text(code: u16) -> String {
    match code_name(code) {
        Some(name) => format!("{name} ({code})"),
        None => code.to_string(),
    }
}

/// The HTTP status that says the same, as gRPC gateways map them: what
/// colors it.
pub fn as_http(code: u16) -> u16 {
    match code {
        0 => 200,
        1 => 499,
        3 | 9 | 11 => 400,
        4 => 504,
        5 => 404,
        6 | 10 => 409,
        7 => 403,
        8 => 429,
        12 => 501,
        14 => 503,
        16 => 401,
        _ => 500,
    }
}

/// `grpc-message` from the trailers, or from the headers of an answer with
/// no body, with its percent-encoding undone.
pub fn message(trailers: &[(String, String)], headers: &[(String, String)]) -> Option<String> {
    let (_, value) = trailers
        .iter()
        .chain(headers)
        .find(|(name, _)| name.eq_ignore_ascii_case("grpc-message"))?;
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    let text = String::from_utf8_lossy(&out).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn codes_have_names_and_colors() {
        assert_eq!(code_text(5), "NOT_FOUND (5)");
        assert_eq!(code_text(42), "42");
        assert_eq!(as_http(0), 200);
        assert_eq!(as_http(16), 401);
        assert_eq!(as_http(13), 500);
    }

    #[test]
    fn the_message_is_percent_decoded() {
        let trailers = lines(&[
            ("grpc-status", "5"),
            ("grpc-message", "kh%C3%B4ng c%C3%B3 %zz 100%"),
        ]);
        assert_eq!(
            message(&trailers, &[]).as_deref(),
            Some("không có %zz 100%")
        );
        let headers = lines(&[("Grpc-Message", "from the head")]);
        assert_eq!(message(&[], &headers).as_deref(), Some("from the head"));
        assert_eq!(message(&lines(&[("grpc-message", " ")]), &[]), None);
    }
}
