//! A request's fields: those of its URL, and those of a form it sends, either
//! urlencoded or multipart.

use tracewing_store::decode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Text(String),
    /// A file sent in a multipart form: its name, type and size.
    File {
        filename: String,
        content_type: Option<String>,
        bytes: usize,
    },
}

/// How much of a form body is read for its fields.
pub const FORM_MAX: usize = 4 << 20;

pub fn query(url: &str) -> Vec<Field> {
    let Some((_, query)) = url.split_once('?') else {
        return Vec::new();
    };
    let query = query.split_once('#').map_or(query, |(q, _)| q);
    decode::form_pairs(query)
        .into_iter()
        .map(|(name, value)| Field {
            name,
            value: Value::Text(value),
        })
        .collect()
}

/// The fields of a form body, or `None` when it is not a form.
pub fn form(content_type: Option<&str>, body: &[u8]) -> Option<Vec<Field>> {
    let content_type = content_type?;
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match essence.as_str() {
        "application/x-www-form-urlencoded" => Some(
            decode::form_pairs(&String::from_utf8_lossy(body))
                .into_iter()
                .map(|(name, value)| Field {
                    name,
                    value: Value::Text(value),
                })
                .collect(),
        ),
        "multipart/form-data" => multipart(&boundary(content_type)?, body),
        _ => None,
    }
}

fn boundary(content_type: &str) -> Option<String> {
    content_type.split(';').skip(1).find_map(|param| {
        let (key, value) = param.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("boundary")
            .then(|| value.trim().trim_matches('"').to_owned())
    })
}

fn multipart(boundary: &str, body: &[u8]) -> Option<Vec<Field>> {
    let delimiter = format!("--{boundary}");
    let delimiter = delimiter.as_bytes();
    let mut fields = Vec::new();
    let mut parts = split(body, delimiter).into_iter().skip(1);
    for part in parts.by_ref() {
        // The last delimiter ends in `--`.
        if part.starts_with(b"--") {
            break;
        }
        let part = part.strip_prefix(b"\r\n").unwrap_or(part);
        let split_at = find(part, b"\r\n\r\n")?;
        let head = String::from_utf8_lossy(&part[..split_at]);
        let mut content = &part[split_at + 4..];
        content = content.strip_suffix(b"\r\n").unwrap_or(content);
        let mut name = None;
        let mut filename = None;
        let mut content_type = None;
        for line in head.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            match key.trim().to_ascii_lowercase().as_str() {
                "content-disposition" => {
                    for param in value.split(';').skip(1) {
                        if let Some((k, v)) = param.split_once('=') {
                            let v = v.trim().trim_matches('"').to_owned();
                            match k.trim().to_ascii_lowercase().as_str() {
                                "name" => name = Some(v),
                                "filename" => filename = Some(v),
                                _ => {}
                            }
                        }
                    }
                }
                "content-type" => content_type = Some(value.trim().to_owned()),
                _ => {}
            }
        }
        let value = match filename {
            Some(filename) => Value::File {
                filename,
                content_type,
                bytes: content.len(),
            },
            None => Value::Text(String::from_utf8_lossy(content).into_owned()),
        };
        fields.push(Field {
            name: name.unwrap_or_default(),
            value,
        });
    }
    Some(fields)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn split<'a>(mut data: &'a [u8], delimiter: &[u8]) -> Vec<&'a [u8]> {
    let mut parts = Vec::new();
    while let Some(at) = find(data, delimiter) {
        parts.push(&data[..at]);
        data = &data[at + delimiter.len()..];
    }
    parts.push(data);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(name: &str, value: &str) -> Field {
        Field {
            name: name.to_owned(),
            value: Value::Text(value.to_owned()),
        }
    }

    #[test]
    fn a_query_is_read_from_the_url() {
        assert_eq!(
            query("https://a.test/s?q=red+shoes&page=2&empty=&flag#top"),
            [
                text("q", "red shoes"),
                text("page", "2"),
                text("empty", ""),
                text("flag", "")
            ]
        );
        assert!(query("https://a.test/").is_empty());
    }

    #[test]
    fn an_urlencoded_form() {
        let fields = form(
            Some("application/x-www-form-urlencoded; charset=utf-8"),
            b"user=an%20na&city=H%C3%A0+N%E1%BB%99i",
        )
        .unwrap();
        assert_eq!(fields, [text("user", "an na"), text("city", "Hà Nội")]);
        assert_eq!(form(Some("application/json"), b"{}"), None);
        assert_eq!(form(None, b"a=b"), None);
    }

    #[test]
    fn a_multipart_form_with_a_file() {
        let body = b"--XyZ\r\n\
Content-Disposition: form-data; name=\"title\"\r\n\r\n\
Holiday\r\n\
--XyZ\r\n\
Content-Disposition: form-data; name=\"photo\"; filename=\"beach.jpg\"\r\n\
Content-Type: image/jpeg\r\n\r\n\
\xff\xd8\xff\xe0abc\r\n\
--XyZ--\r\n";
        let fields = form(Some("multipart/form-data; boundary=\"XyZ\""), body).unwrap();
        assert_eq!(fields[0], text("title", "Holiday"));
        assert_eq!(
            fields[1],
            Field {
                name: "photo".to_owned(),
                value: Value::File {
                    filename: "beach.jpg".to_owned(),
                    content_type: Some("image/jpeg".to_owned()),
                    bytes: 7,
                },
            }
        );
    }
}
