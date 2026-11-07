//! The cookies of one side of a flow: what a request sends in `Cookie`, and
//! what a response sets with each `Set-Cookie`.

use crate::detail::Side;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    /// What a response says about it, as written: `Path=/`, `HttpOnly`.
    pub attributes: Vec<String>,
}

pub fn of(headers: &[(String, String)], side: Side) -> Vec<Cookie> {
    let wanted = match side {
        Side::Request => "cookie",
        Side::Response => "set-cookie",
    };
    let values = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
        .map(|(_, value)| value.as_str());
    match side {
        Side::Request => values
            .flat_map(|value| value.split(';'))
            .filter_map(|pair| pair_of(pair, Vec::new()))
            .collect(),
        Side::Response => values
            .filter_map(|value| {
                let mut parts = value.split(';');
                let first = parts.next()?;
                let attributes = parts
                    .map(str::trim)
                    .filter(|a| !a.is_empty())
                    .map(str::to_owned)
                    .collect();
                pair_of(first, attributes)
            })
            .collect(),
    }
}

/// How many cookies each side has, request first.
pub fn counts(request: &[(String, String)], response: &[(String, String)]) -> (usize, usize) {
    (
        of(request, Side::Request).len(),
        of(response, Side::Response).len(),
    )
}

fn pair_of(pair: &str, attributes: Vec<String>) -> Option<Cookie> {
    let pair = pair.trim();
    if pair.is_empty() {
        return None;
    }
    let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
    Some(Cookie {
        name: name.trim().to_owned(),
        value: value.trim().to_owned(),
        attributes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn a_request_sends_them_in_one_line_or_several() {
        let sent = of(
            &headers(&[
                ("Cookie", "sid=abc; theme=dark;"),
                ("accept", "*/*"),
                ("cookie", "flag; a=b=c"),
            ]),
            Side::Request,
        );
        let names: Vec<_> = sent
            .iter()
            .map(|c| (c.name.as_str(), c.value.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("sid", "abc"),
                ("theme", "dark"),
                ("flag", ""),
                ("a", "b=c")
            ]
        );
    }

    #[test]
    fn a_response_sets_one_a_header() {
        let set = of(
            &headers(&[
                (
                    "set-cookie",
                    "sid=xyz; Path=/; Expires=Wed, 21 Oct 2026 07:28:00 GMT; HttpOnly",
                ),
                ("Set-Cookie", "lang=vi"),
                ("cookie", "not=this"),
            ]),
            Side::Response,
        );
        assert_eq!(set.len(), 2);
        assert_eq!(
            (set[0].name.as_str(), set[0].value.as_str()),
            ("sid", "xyz")
        );
        assert_eq!(
            set[0].attributes,
            [
                "Path=/",
                "Expires=Wed, 21 Oct 2026 07:28:00 GMT",
                "HttpOnly"
            ]
        );
        assert!(set[1].attributes.is_empty());
    }
}
