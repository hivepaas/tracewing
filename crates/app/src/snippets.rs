//! A flow's request as code to send it again: a command line, or a few lines
//! of a program, to paste.

/// How much of a body goes in. Past this, it is left out and the code says
/// so: nobody pastes more.
const BODY_MAX: usize = 256 * 1024;

/// Hop-by-hop headers, and ones every client works out itself.
const LEFT_OUT: [&str; 6] = [
    "content-length",
    "host",
    "connection",
    "proxy-connection",
    "accept-encoding",
    "transfer-encoding",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Curl,
    Httpie,
    Fetch,
    Python,
    Go,
    PowerShell,
}

impl Language {
    pub const ALL: [Self; 6] = [
        Self::Curl,
        Self::Httpie,
        Self::Fetch,
        Self::Python,
        Self::Go,
        Self::PowerShell,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Curl => "cURL",
            Self::Httpie => "HTTPie",
            Self::Fetch => "JavaScript fetch",
            Self::Python => "Python requests",
            Self::Go => "Go net/http",
            Self::PowerShell => "PowerShell",
        }
    }
}

/// What a body can be in code.
enum Body<'a> {
    None,
    Text(&'a str),
    /// Why it is not there.
    LeftOut(String),
}

fn body_of(body: Option<&[u8]>) -> Body<'_> {
    match body {
        None | Some([]) => Body::None,
        Some(body) if body.len() > BODY_MAX => {
            Body::LeftOut(format!("body of {} bytes left out", body.len()))
        }
        Some(body) => match std::str::from_utf8(body) {
            Ok(text) => Body::Text(text),
            Err(_) => Body::LeftOut(format!("binary body of {} bytes left out", body.len())),
        },
    }
}

fn kept(headers: &[(String, String)]) -> impl Iterator<Item = &(String, String)> {
    headers
        .iter()
        .filter(|(name, _)| !LEFT_OUT.contains(&name.to_ascii_lowercase().as_str()))
}

/// One value per name, in the order first seen, for a language that keeps
/// headers in a map. Cookies join with `; `, others with `, `.
fn merged(headers: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (name, value) in kept(headers) {
        match out.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            Some((n, v)) => {
                v.push_str(if n.eq_ignore_ascii_case("cookie") {
                    "; "
                } else {
                    ", "
                });
                v.push_str(value);
            }
            None => out.push((name.clone(), value.clone())),
        }
    }
    out
}

/// A shell's single quotes.
fn sh(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// A double-quoted string as JSON writes it, which JavaScript, Python and Go
/// read the same.
fn dq(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

/// PowerShell's single quotes, which double a quote inside.
fn ps(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The request as code in `language`.
pub fn snippet(
    language: Language,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> String {
    let body = body_of(body);
    match language {
        Language::Curl => curl(method, url, headers, &body),
        Language::Httpie => httpie(method, url, headers, &body),
        Language::Fetch => fetch(method, url, headers, &body),
        Language::Python => python(method, url, headers, &body),
        Language::Go => go(method, url, headers, &body),
        Language::PowerShell => powershell(method, url, headers, &body),
    }
}

fn curl(method: &str, url: &str, headers: &[(String, String)], body: &Body) -> String {
    let mut parts = vec![format!("curl {}", sh(url))];
    if method != "GET" {
        parts.push(format!("-X {}", sh(method)));
    }
    for (name, value) in kept(headers) {
        parts.push(format!("-H {}", sh(&format!("{name}: {value}"))));
    }
    parts.push("--compressed".to_owned());
    match body {
        Body::None => {}
        Body::Text(text) => parts.push(format!("--data-raw {}", sh(text))),
        Body::LeftOut(why) => parts.push(format!("# {why}")),
    }
    parts.join(" \\\n  ")
}

fn httpie(method: &str, url: &str, headers: &[(String, String)], body: &Body) -> String {
    let mut parts = vec![if method == "GET" {
        format!("http {}", sh(url))
    } else {
        format!("http {method} {}", sh(url))
    }];
    for (name, value) in kept(headers) {
        parts.push(sh(&format!("{name}:{value}")));
    }
    match body {
        Body::None => {}
        Body::Text(text) => parts.push(format!("--raw {}", sh(text))),
        Body::LeftOut(why) => parts.push(format!("# {why}")),
    }
    parts.join(" \\\n  ")
}

fn fetch(method: &str, url: &str, headers: &[(String, String)], body: &Body) -> String {
    let mut options = Vec::new();
    if method != "GET" {
        options.push(format!("  method: {},", dq(method)));
    }
    let headers = merged(headers);
    if !headers.is_empty() {
        let lines: Vec<String> = headers
            .iter()
            .map(|(name, value)| format!("    {}: {},", dq(name), dq(value)))
            .collect();
        options.push(format!("  headers: {{\n{}\n  }},", lines.join("\n")));
    }
    match body {
        Body::None => {}
        Body::Text(text) => options.push(format!("  body: {},", dq(text))),
        Body::LeftOut(why) => options.push(format!("  // {why}")),
    }
    if options.is_empty() {
        format!("const response = await fetch({});", dq(url))
    } else {
        format!(
            "const response = await fetch({}, {{\n{}\n}});",
            dq(url),
            options.join("\n")
        )
    }
}

fn python(method: &str, url: &str, headers: &[(String, String)], body: &Body) -> String {
    let call = match method {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS" => {
            format!(
                "requests.{}(\n    {},",
                method.to_ascii_lowercase(),
                dq(url)
            )
        }
        other => format!("requests.request(\n    {},\n    {},", dq(other), dq(url)),
    };
    let mut lines = vec![
        "import requests".to_owned(),
        String::new(),
        format!("response = {call}"),
    ];
    let headers = merged(headers);
    if !headers.is_empty() {
        lines.push("    headers={".to_owned());
        for (name, value) in &headers {
            lines.push(format!("        {}: {},", dq(name), dq(value)));
        }
        lines.push("    },".to_owned());
    }
    match body {
        Body::None => {}
        // Bytes: a str goes out as Latin-1, which not every text is.
        Body::Text(text) => lines.push(format!("    data={}.encode(),", dq(text))),
        Body::LeftOut(why) => lines.push(format!("    # {why}")),
    }
    lines.push(")".to_owned());
    lines.join("\n")
}

fn go(method: &str, url: &str, headers: &[(String, String)], body: &Body) -> String {
    let text = match body {
        Body::Text(text) => Some(*text),
        _ => None,
    };
    let mut imports = vec!["\"fmt\"", "\"io\"", "\"net/http\""];
    if text.is_some() {
        imports.push("\"strings\"");
    }
    let mut lines = vec![
        "package main".to_owned(),
        String::new(),
        "import (".to_owned(),
    ];
    lines.extend(imports.iter().map(|i| format!("\t{i}")));
    lines.extend([")".to_owned(), String::new(), "func main() {".to_owned()]);
    if let Body::LeftOut(why) = body {
        lines.push(format!("\t// {why}"));
    }
    let reader = match text {
        Some(text) => format!("strings.NewReader({})", dq(text)),
        None => "nil".to_owned(),
    };
    lines.push(format!(
        "\treq, err := http.NewRequest({}, {}, {reader})",
        dq(method),
        dq(url)
    ));
    lines.extend(["\tif err != nil {", "\t\tpanic(err)", "\t}"].map(str::to_owned));
    for (name, value) in kept(headers) {
        lines.push(format!("\treq.Header.Add({}, {})", dq(name), dq(value)));
    }
    lines.extend(
        [
            "\tresp, err := http.DefaultClient.Do(req)",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tdefer resp.Body.Close()",
            "\tout, _ := io.ReadAll(resp.Body)",
            "\tfmt.Println(resp.Status, string(out))",
            "}",
        ]
        .map(str::to_owned),
    );
    lines.join("\n")
}

fn powershell(method: &str, url: &str, headers: &[(String, String)], body: &Body) -> String {
    let mut lines = Vec::new();
    let mut content_type = None;
    let mut rest = Vec::new();
    for (name, value) in merged(headers) {
        // Windows PowerShell takes it only as its own parameter.
        if name.eq_ignore_ascii_case("content-type") {
            content_type = Some(value);
        } else {
            rest.push((name, value));
        }
    }
    if !rest.is_empty() {
        lines.push("$headers = @{".to_owned());
        for (name, value) in &rest {
            lines.push(format!("  {} = {}", ps(name), ps(value)));
        }
        lines.push("}".to_owned());
    }
    let mut call = format!("Invoke-WebRequest -Uri {}", ps(url));
    if method != "GET" {
        call.push_str(&format!(" -Method {}", ps(method)));
    }
    if !rest.is_empty() {
        call.push_str(" -Headers $headers");
    }
    if let Some(content_type) = &content_type {
        call.push_str(&format!(" -ContentType {}", ps(content_type)));
    }
    match body {
        Body::None => {}
        Body::Text(text) => call.push_str(&format!(" -Body {}", ps(text))),
        Body::LeftOut(why) => lines.push(format!("# {why}")),
    }
    lines.push(call);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers() -> Vec<(String, String)> {
        vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("host".to_owned(), "a.dev".to_owned()),
            ("x-note".to_owned(), "it's here".to_owned()),
            ("cookie".to_owned(), "a=1".to_owned()),
            ("cookie".to_owned(), "b=2".to_owned()),
        ]
    }

    const URL: &str = "https://a.dev/v1?x=1";
    const BODY: &[u8] = "{\"a\":\"đ\"}".as_bytes();

    fn post(language: Language) -> String {
        snippet(language, "POST", URL, &headers(), Some(BODY))
    }

    #[test]
    fn curl_as_before() {
        assert_eq!(
            post(Language::Curl),
            "curl 'https://a.dev/v1?x=1' \\\n  -X 'POST' \\\n  -H 'content-type: application/json' \\\n  -H 'x-note: it'\\''s here' \\\n  -H 'cookie: a=1' \\\n  -H 'cookie: b=2' \\\n  --compressed \\\n  --data-raw '{\"a\":\"đ\"}'"
        );
        assert_eq!(
            snippet(Language::Curl, "GET", "https://a.dev/", &[], None),
            "curl 'https://a.dev/' \\\n  --compressed"
        );
    }

    #[test]
    fn httpie_names_the_method_and_sends_the_body_raw() {
        assert_eq!(
            post(Language::Httpie),
            "http POST 'https://a.dev/v1?x=1' \\\n  'content-type:application/json' \\\n  'x-note:it'\\''s here' \\\n  'cookie:a=1' \\\n  'cookie:b=2' \\\n  --raw '{\"a\":\"đ\"}'"
        );
        assert_eq!(
            snippet(Language::Httpie, "GET", "https://a.dev/", &[], None),
            "http 'https://a.dev/'"
        );
    }

    #[test]
    fn fetch_keeps_one_value_per_header() {
        assert_eq!(
            post(Language::Fetch),
            "const response = await fetch(\"https://a.dev/v1?x=1\", {\n  method: \"POST\",\n  headers: {\n    \"content-type\": \"application/json\",\n    \"x-note\": \"it's here\",\n    \"cookie\": \"a=1; b=2\",\n  },\n  body: \"{\\\"a\\\":\\\"đ\\\"}\",\n});"
        );
        assert_eq!(
            snippet(Language::Fetch, "GET", "https://a.dev/", &[], None),
            "const response = await fetch(\"https://a.dev/\");"
        );
    }

    #[test]
    fn python_sends_bytes() {
        let code = post(Language::Python);
        assert!(
            code.starts_with(
                "import requests\n\nresponse = requests.post(\n    \"https://a.dev/v1?x=1\",\n    headers={\n"
            ),
            "{code}"
        );
        assert!(
            code.contains("        \"cookie\": \"a=1; b=2\",\n"),
            "{code}"
        );
        assert!(
            code.contains("    data=\"{\\\"a\\\":\\\"đ\\\"}\".encode(),\n)"),
            "{code}"
        );
        let odd = snippet(Language::Python, "PROPFIND", "https://a.dev/", &[], None);
        assert!(
            odd.contains("requests.request(\n    \"PROPFIND\",\n    \"https://a.dev/\","),
            "{odd}"
        );
    }

    #[test]
    fn go_builds_a_whole_program() {
        let code = post(Language::Go);
        assert!(code.starts_with("package main\n"), "{code}");
        assert!(code.contains("\t\"strings\"\n"), "{code}");
        assert!(
            code.contains(
                "http.NewRequest(\"POST\", \"https://a.dev/v1?x=1\", strings.NewReader(\"{\\\"a\\\":\\\"đ\\\"}\"))"
            ),
            "{code}"
        );
        assert!(
            code.contains("\treq.Header.Add(\"cookie\", \"b=2\")\n"),
            "{code}"
        );
        let get = snippet(Language::Go, "GET", "https://a.dev/", &[], None);
        assert!(get.contains(", nil)") && !get.contains("strings"), "{get}");
    }

    #[test]
    fn powershell_gives_the_content_type_its_own_parameter() {
        assert_eq!(
            post(Language::PowerShell),
            "$headers = @{\n  'x-note' = 'it''s here'\n  'cookie' = 'a=1; b=2'\n}\nInvoke-WebRequest -Uri 'https://a.dev/v1?x=1' -Method 'POST' -Headers $headers -ContentType 'application/json' -Body '{\"a\":\"đ\"}'"
        );
    }

    #[test]
    fn what_cannot_go_in_says_so_in_each() {
        let big = vec![b'x'; BODY_MAX + 1];
        for language in Language::ALL {
            let binary = snippet(language, "POST", URL, &[], Some(&[0xff, 0xfe]));
            assert!(
                binary.contains("binary body of 2 bytes left out"),
                "{language:?}: {binary}"
            );
            let large = snippet(language, "POST", URL, &[], Some(&big));
            assert!(large.contains("left out"), "{language:?}");
        }
    }
}
