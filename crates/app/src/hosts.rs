//! Hosts the user keeps at the top of the sidebar, and hosts whose flows they do
//! not want to see. Both are kept with the layout.

use gpui::SharedString;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HostMarks {
    /// In the order they were pinned.
    pub pinned: Vec<String>,
    /// Their flows are left out of the list.
    pub hidden: Vec<String>,
}

/// A host in the sidebar, with how many flows it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostLine {
    pub host: SharedString,
    pub count: u64,
    pub pinned: bool,
}

impl HostMarks {
    pub fn is_pinned(&self, host: &str) -> bool {
        self.pinned.iter().any(|h| h == host)
    }

    pub fn is_hidden(&self, host: &str) -> bool {
        self.hidden.iter().any(|h| h == host)
    }

    /// Pins a host, or unpins a pinned one. A hidden host is shown again to be
    /// pinned: one at the top that shows nothing would make no sense.
    pub fn toggle_pin(&mut self, host: &str) {
        if let Some(at) = self.pinned.iter().position(|h| h == host) {
            self.pinned.remove(at);
        } else {
            self.hidden.retain(|h| h != host);
            self.pinned.push(host.to_owned());
        }
    }

    /// Hides a host's flows, or shows a hidden one's again. Hidden, it is no
    /// longer pinned.
    pub fn toggle_hidden(&mut self, host: &str) {
        if let Some(at) = self.hidden.iter().position(|h| h == host) {
            self.hidden.remove(at);
        } else {
            self.pinned.retain(|h| h != host);
            self.hidden.push(host.to_owned());
        }
    }

    /// The hosts to list, pinned first in the order they were pinned, then the
    /// rest as `counts` has them, busiest first; and apart, those hidden. A
    /// pinned or hidden host with no flows yet is listed with none.
    pub fn arrange(&self, counts: &[(SharedString, u64)]) -> (Vec<HostLine>, Vec<HostLine>) {
        let count_of = |host: &str| {
            counts
                .iter()
                .find(|(h, _)| h.as_ref() == host)
                .map_or(0, |(_, n)| *n)
        };
        let marked = |host: &str, pinned: bool| HostLine {
            host: host.to_owned().into(),
            count: count_of(host),
            pinned,
        };
        let mut shown: Vec<HostLine> = self.pinned.iter().map(|h| marked(h, true)).collect();
        shown.extend(
            counts
                .iter()
                .filter(|(h, _)| !self.is_pinned(h) && !self.is_hidden(h))
                .map(|(host, count)| HostLine {
                    host: host.clone(),
                    count: *count,
                    pinned: false,
                }),
        );
        let hidden = self.hidden.iter().map(|h| marked(h, false)).collect();
        (shown, hidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(list: &[(&str, u64)]) -> Vec<(SharedString, u64)> {
        list.iter()
            .map(|(h, n)| (SharedString::from(h.to_string()), *n))
            .collect()
    }

    fn names(lines: &[HostLine]) -> Vec<(String, u64, bool)> {
        lines
            .iter()
            .map(|l| (l.host.to_string(), l.count, l.pinned))
            .collect()
    }

    #[test]
    fn pinned_hosts_come_first_and_hidden_ones_apart() {
        let mut marks = HostMarks::default();
        marks.toggle_pin("api.example.com");
        marks.toggle_pin("quiet.example.com");
        marks.toggle_hidden("ads.example.net");
        let (shown, hidden) = marks.arrange(&counts(&[
            ("ads.example.net", 90),
            ("cdn.example.com", 40),
            ("api.example.com", 7),
        ]));
        assert_eq!(
            names(&shown),
            [
                ("api.example.com".into(), 7, true),
                ("quiet.example.com".into(), 0, true),
                ("cdn.example.com".into(), 40, false),
            ]
        );
        assert_eq!(names(&hidden), [("ads.example.net".into(), 90, false)]);
    }

    #[test]
    fn a_host_is_pinned_or_hidden_not_both() {
        let mut marks = HostMarks::default();
        marks.toggle_pin("a");
        marks.toggle_hidden("a");
        assert!(!marks.is_pinned("a") && marks.is_hidden("a"));
        marks.toggle_pin("a");
        assert!(marks.is_pinned("a") && !marks.is_hidden("a"));
        marks.toggle_pin("a");
        assert_eq!(marks, HostMarks::default());
    }

    #[test]
    fn they_are_kept_as_written() {
        let mut marks = HostMarks::default();
        marks.toggle_pin("a");
        marks.toggle_hidden("b");
        let text = toml::to_string(&marks).unwrap();
        assert_eq!(toml::from_str::<HostMarks>(&text).unwrap(), marks);
        assert_eq!(
            toml::from_str::<HostMarks>("").unwrap(),
            HostMarks::default()
        );
    }
}
