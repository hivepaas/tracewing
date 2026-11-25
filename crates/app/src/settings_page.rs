//! The settings screen: every storage limit, with the app's default until the user
//! says otherwise. Each change is written to `settings.toml`, the same file a
//! person could edit by hand, as it is made. A few apply at once; the rest
//! from the next start on, and the screen says which are still waiting.

use crate::text_size::ui;
use crate::{
    colors::{DarkStyle, LightStyle, colors},
    input::TextInput,
    layout::{Appearance, SettingsSection, Way},
    system::ProxySwitch,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{
    Context, Entity, Focusable, FontWeight, KeyDownEvent, Subscription, Window, div, prelude::*,
    rgb,
};
use std::time::{Duration, Instant};
use tracewing_i18n::{tr, tr_n};
use tracewing_proxy::rules::HostRules;
use tracewing_proxy::source::By;
use tracewing_store::settings::StorageSettings;

/// Where TLS secrets are written for Wireshark, in the data folder.
pub const TLS_KEYS_FILE: &str = "tls-keys.log";

/// One setting. `key` is its name in the file; a test sees that none is forgotten.
struct Row {
    key: &'static str,
    label: String,
    help: String,
    unit: &'static str,
}

fn row(key: &'static str, label: String, help: String, unit: &'static str) -> Row {
    Row {
        key,
        label,
        help,
        unit,
    }
}

/// A group of settings: where it is shown, and what it is called there.
struct Group {
    at: SettingsSection,
    /// The way in it belongs to, for the groups under [`SettingsSection::Capture`];
    /// `None` for the rest, which are about what tracewing does with traffic
    /// rather than about how traffic arrives.
    way: Option<Way>,
    title: String,
    rows: Vec<Row>,
}

fn group(at: SettingsSection, way: Option<Way>, title: String, rows: Vec<Row>) -> Group {
    Group {
        at,
        way,
        title,
        rows,
    }
}

/// Which way in's count a tab reads, where tracewing counts one.
///
/// Three ways in share the proxy port — this machine's proxy setting, a
/// browser tracewing started with a profile of its own, and a terminal it set
/// the variables in — so all three read the same count, and the sentence they
/// show has to say that it covers all three. A browser and a terminal are not
/// switches anybody leaves on, so neither carries the sentence at all; it is
/// for a way in somebody turned on and then waited at.
fn counted_as(way: Way) -> Option<By> {
    match way {
        Way::SystemProxy => Some(By::ProxyPort),
        Way::OtherDevices => Some(By::OtherDevices),
        Way::Socks5 => Some(By::Socks5),
        Way::WireGuard => Some(By::WireGuard),
        Way::Apps => Some(By::Extension),
        Way::Browser | Way::Terminal => None,
    }
}

/// How long a way in is left alone after it is switched on before the tab
/// says nothing has come through it.
///
/// Without this the line appears the instant somebody flips a switch, which
/// is both useless — nothing has had time to arrive — and alarming, which is
/// the opposite of what it is for.
const QUIET_AFTER: Duration = Duration::from_secs(5);

/// Whether a way in has been on this long with nothing arriving by it.
///
/// `since` is when it became able to deliver, and the heart of the thing: a
/// count that was not zero last week says nothing about a switch flipped a
/// minute ago, so the question is always asked from a moment rather than
/// about a total.
fn gone_quiet(
    way: Way,
    since: Option<Instant>,
    arrivals: &tracewing_proxy::source::Arrivals,
    now: Instant,
) -> bool {
    let Some(by) = counted_as(way) else {
        return false;
    };
    let Some(since) = since else {
        return false;
    };
    now.duration_since(since) >= QUIET_AFTER && !arrivals.anything_since(by, since)
}

/// What has to trust tracewing for a way in, and where that is done.
///
/// HTTPS is unreadable until something trusts tracewing, and which something
/// differs by way in. A tab that lists what it catches and leaves this out
/// has somebody turning a switch on, seeing certificate errors, and having
/// no way to tell which of the two halves is missing.
///
/// Out here rather than in the view because a way in with no answer here is
/// a tab with a blank line on it, and that is worth a test rather than a
/// reading of the window.
fn trust_for(way: Way) -> (String, Option<SettingsSection>) {
    match way {
        Way::SystemProxy | Way::Socks5 => (
            tr!(
                "This computer's own store, which Certificates installs tracewing's authority into. A program with a store of its own — Java, or Firefox — needs it there too."
            ),
            Some(SettingsSection::Certificates),
        ),
        Way::Browser => (
            tr!(
                "Nothing of yours: the profile tracewing makes for the browser has the authority in it already."
            ),
            None,
        ),
        Way::Terminal => (
            tr!(
                "Nothing of yours: the terminal points what starts in it at tracewing's roots, in a file, and installs nothing."
            ),
            None,
        ),
        Way::OtherDevices | Way::WireGuard => (
            tr!(
                "The other device, which installs the authority itself. Set up has a guide for each kind of device under \"my phone\"."
            ),
            None,
        ),
        Way::Apps if cfg!(target_os = "macos") => (
            tr!(
                "This computer's own store, as for the system proxy: macOS hands the flow over, and the app still checks the certificate it is given."
            ),
            Some(SettingsSection::Certificates),
        ),
        // The same store, for the same reason, reached the other way: the
        // adapter hands the connection over and the program still checks
        // what it is given.
        Way::Apps => (
            tr!(
                "This computer's own store, as for the system proxy: the adapter hands the connection over, and the program still checks the certificate it is given."
            ),
            Some(SettingsSection::Certificates),
        ),
    }
}

/// The groups a part of Settings shows — and, under
/// [`SettingsSection::Capture`], only those of the way in whose tab is open.
///
/// Every place that asks "what is on this page" goes through here, which is
/// what keeps "Back to defaults" and the restart notice from reaching across
/// tabs into settings nobody can see.
fn shown_in(at: SettingsSection, way: Way) -> Vec<Group> {
    groups()
        .into_iter()
        .filter(|g| g.at == at && (at != SettingsSection::Capture || g.way == Some(way)))
        .collect()
}

/// The settings by group, each in its part of Settings, said in the language
/// in use.
fn groups() -> Vec<Group> {
    #[allow(unused_mut)]
    let mut all = vec![
        group(
            SettingsSection::Capture,
            Some(Way::OtherDevices),
            tr!("Other devices"),
            vec![row(
                "other_devices",
                tr!("Let other devices use the proxy"),
                tr!(
                    "Phones and other computers on your network can then send their traffic here. \
                     So can anyone else on it: leave this off on a network you do not trust."
                ),
                "",
            )],
        ),
        group(
            SettingsSection::Capture,
            Some(Way::Socks5),
            // The protocol's name, as in the relay settings: not translated.
            "SOCKS5".to_owned(),
            vec![
                row(
                    "socks",
                    tr!("Open a SOCKS5 port as well"),
                    tr!(
                        "For programs that speak SOCKS and not HTTP proxying: anything that reads \
                         ALL_PROXY, Firefox's own SOCKS setting, proxychains. Takes effect when \
                         tracewing starts again."
                    ),
                    "",
                ),
                row(
                    "socks_port",
                    tr!("The port it listens on"),
                    tr!(
                        "1080 is the port a SOCKS client picks when it is told nothing else. This \
                         computer only: other devices use the proxy port, whatever this says."
                    ),
                    "",
                ),
            ],
        ),
        group(
            SettingsSection::Capture,
            Some(Way::WireGuard),
            // The protocol's name, as above: not translated.
            "WireGuard".to_owned(),
            vec![
                row(
                    "wireguard",
                    tr!("Let a phone join a tunnel"),
                    tr!(
                        "The phone installs the WireGuard app, scans or copies the configuration \
                         below, and everything it sends is captured — including apps that ignore \
                         the phone's proxy setting. Unlike the SOCKS port this one is open to your \
                         network, and only the phone whose key is in that configuration can use \
                         it. Takes effect when tracewing starts again."
                    ),
                    "",
                ),
                row(
                    "wireguard_port",
                    tr!("The port it listens on"),
                    tr!(
                        "51820 is WireGuard's own port, which is the one a router is most likely \
                         to allow already."
                    ),
                    "",
                ),
            ],
        ),
        group(
            SettingsSection::Certificates,
            None,
            tr!("Servers"),
            vec![row(
                "os_roots",
                tr!("Trust what this computer trusts"),
                tr!(
                    "Servers with a certificate from an authority installed on this computer, such as \
                     a company's own, are reached as this computer's browsers reach them. Off, only \
                     the public authorities are trusted. Takes effect when tracewing starts again."
                ),
                "",
            )],
        ),
        group(
            SettingsSection::Decoding,
            None,
            tr!("Reading values"),
            vec![row(
                "mark_readings",
                tr!("Underline what can be read"),
                tr!(
                    "A value with a reading behind it — a time, a token, something packed — is \
                     underlined, so that a right click on it is worth trying. Off, the readings \
                     are still there; nothing says so."
                ),
                "",
            )],
        ),
        group(
            SettingsSection::Ai,
            None,
            tr!("AI tools"),
            vec![
                row(
                    "ai_secrets",
                    tr!("Let AI tools see tokens and cookies"),
                    tr!(
                        "AI tools read recorded traffic through tracewing's MCP server. Off, the values of \
                         Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once."
                    ),
                    "",
                ),
                row(
                    "ai_actions",
                    tr!("Let AI tools add rules and send requests"),
                    tr!(
                        "Through the MCP server, an AI tool may then add and remove rules, which change \
                         traffic as yours do, and send requests through tracewing, again or changed, which \
                         reach their servers. Off, it can only read. Applies at once."
                    ),
                    "",
                ),
            ],
        ),
        group(
            SettingsSection::Network,
            None,
            tr!("HTTP/3"),
            vec![
                row(
                    "hide_http3",
                    tr!("Hide servers' offers of HTTP/3"),
                    tr!(
                        "HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around \
                         tracewing. This takes servers' offers of it out of their responses, so an app that \
                         would have learned of HTTP/3 from one stays on HTTP/2. An app that learns it from a \
                         DNS record instead tries anyway: blocking UDP port 443, on the Set up page, is what \
                         stops that. Flows still show the offers as servers sent them."
                    ),
                    "",
                ),
                row(
                    "capture_http3",
                    tr!("Read chosen programs' HTTP/3"),
                    tr!(
                        "Rather than refusing it. The HTTP/3 of the programs chosen for taking this \
                         computer's own traffic goes to a QUIC port of tracewing's own instead of to the \
                         server, and its flows are recorded like any others. A program nobody chose falls \
                         back to HTTP/2 as before, and so does every other way in. While this is on, \
                         servers' offers of HTTP/3 are left in whatever the setting above says, because a \
                         program never offered HTTP/3 never speaks it. Blocking UDP port 443, on the Set \
                         up page, stops it too: those datagrams never reach tracewing at all. Applies at \
                         once."
                    ),
                    "",
                ),
            ],
        ),
        group(
            SettingsSection::Storage,
            None,
            tr!("Sessions"),
            vec![
                row(
                    "session_max_mib",
                    tr!("Size of a session"),
                    tr!("Past this, the oldest flows are dropped."),
                    "MiB",
                ),
                row(
                    "disk_reserve_mib",
                    tr!("Room to leave on the disk"),
                    tr!(
                        "With less than this free, flows are still listed but bodies are not kept."
                    ),
                    "MiB",
                ),
                row(
                    "sessions_kept",
                    tr!("Earlier sessions to keep"),
                    tr!("Older ones are deleted when a new session starts."),
                    "",
                ),
            ],
        ),
        group(
            SettingsSection::Storage,
            None,
            tr!("Bodies"),
            vec![
                row(
                    "body_max_mib",
                    tr!("Most kept of one body"),
                    tr!("It still reaches the client whole."),
                    "MiB",
                ),
                row(
                    "view_max_mib",
                    tr!("Most unpacked for viewing"),
                    tr!("Stops a small download that unpacks to many GB."),
                    "MiB",
                ),
                row(
                    "log_max_kib",
                    tr!("Small body size"),
                    tr!(
                        "Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive."
                    ),
                    "KiB",
                ),
                row(
                    "compress_stored",
                    tr!("Compress plain text on disk"),
                    tr!(
                        "Takes about half the room, and is skipped when traffic is too fast for it."
                    ),
                    "",
                ),
                row(
                    "compress_level",
                    tr!("Compression level"),
                    tr!("1 to 19. Higher is smaller and slower."),
                    "",
                ),
            ],
        ),
        group(
            SettingsSection::Storage,
            None,
            tr!("Search"),
            vec![
                row(
                    "index_urls",
                    tr!("Search URLs"),
                    tr!("Finds any part of a URL as you type. Costs little."),
                    "",
                ),
                row(
                    "index_bodies",
                    tr!("Search bodies"),
                    tr!("Finds words in bodies. Can take as much disk as the bodies do."),
                    "",
                ),
                row(
                    "index_kib_per_body",
                    tr!("Searched part of each body"),
                    tr!("Words past this are not found. More takes more disk."),
                    "KiB",
                ),
            ],
        ),
        group(
            SettingsSection::Storage,
            None,
            tr!("Under heavy traffic"),
            vec![
                row(
                    "queue_budget_mib",
                    tr!("Memory for bodies waiting to be written"),
                    tr!(
                        "Past this, flows are listed without their bodies. Traffic is never held back."
                    ),
                    "MiB",
                ),
                row(
                    "spool_budget_mib",
                    tr!("Memory for large bodies waiting to be written"),
                    tr!("Past this, large bodies stop being kept."),
                    "MiB",
                ),
            ],
        ),
        group(
            SettingsSection::About,
            None,
            tr!("Updates"),
            vec![row(
                "check_updates",
                tr!("Look for a new version once a day"),
                tr!(
                    "tracewing reads its signed list of releases from GitHub, which sees this \
                     computer's address and nothing else. Nothing is downloaded until you say so. \
                     Off, it looks only when you press Check now."
                ),
                "",
            )],
        ),
    ];
    // Windows only, where a program's level and the person's can differ and
    // where tracewing has a way to ask for the other one. Nothing to choose
    // between elsewhere, so nothing is shown: see `system::level_wanted`,
    // which reads the switch as off on a machine that cannot offer it.
    #[cfg(windows)]
    all.extend([
        group(
            SettingsSection::Capture,
            Some(Way::Browser),
            tr!("Level"),
            vec![row(
                "browser_as_administrator",
                tr!("Open the browser as an administrator"),
                tr!(
                    "What tracewing starts then has full control of this computer, and tracewing \
                     cannot limit it. Chromium-based browsers also refuse or complain when they \
                     are run this way, so this is for the rare extension or kiosk being debugged \
                     under elevation. While tracewing is not an administrator itself, Windows \
                     asks at each launch. Read at each launch."
                ),
                "",
            )],
        ),
        group(
            SettingsSection::Capture,
            Some(Way::Terminal),
            tr!("Level"),
            vec![row(
                "terminal_as_administrator",
                tr!("Open the terminal as an administrator"),
                tr!(
                    "A program that needs an administrator will not start in a terminal that is \
                     not one, and this terminal is here to put the proxy's variables in front of \
                     a program -- so this is an ordinary case rather than an edge. What it starts \
                     has full control of this computer, and tracewing cannot limit it. While \
                     tracewing is not an administrator itself, Windows asks at each launch. Read \
                     at each launch."
                ),
                "",
            )],
        ),
    ]);
    all
}

enum Field {
    Toggle(bool),
    Number(Entity<TextInput>),
}

pub struct SettingsPage {
    fields: Vec<(&'static str, Field)>,
    /// Hosts not to decrypt, one a line. Applies at once, unlike the rest.
    no_decrypt: Entity<TextInput>,
    /// What `settings.toml` says, as last read or written.
    stored: StorageSettings,
    /// The setting last changed, under which a problem with the fields shows.
    editing: Option<&'static str>,
    pub problem: Option<String>,
    /// What there was before "Back to defaults", while it can be undone.
    undo: Option<StorageSettings>,
    /// What writes a field when it is left. Made at the first draw, which has
    /// the window.
    leaving: Vec<Subscription>,
}

fn value_text(value: &toml::Value) -> String {
    match value {
        toml::Value::Integer(n) => n.to_string(),
        other => other.to_string(),
    }
}

impl SettingsPage {
    pub fn new(settings: &StorageSettings, cx: &mut Context<Traffic>) -> Self {
        let table = toml::Value::try_from(settings).expect("settings are plain values");
        let mut fields = Vec::new();
        for group in groups() {
            for row in group.rows {
                let value = &table[row.key];
                let field = match value {
                    toml::Value::Boolean(on) => Field::Toggle(*on),
                    value => {
                        let text = value_text(value);
                        Field::Number(cx.new(|cx| {
                            let mut input = TextInput::new("", cx);
                            input.set_text(&text, text.len()..text.len(), cx);
                            input
                        }))
                    }
                };
                fields.push((row.key, field));
            }
        }
        let no_decrypt = cx.new(|cx| TextInput::new("", cx).multiline(5, 12));
        Self {
            fields,
            no_decrypt,
            stored: settings.clone(),
            editing: None,
            problem: None,
            undo: None,
            leaving: Vec::new(),
        }
    }

    /// The settings written that wait for tracewing to start again: those
    /// that differ from what it runs with.
    pub fn waiting(&self, running: &StorageSettings) -> Vec<&'static str> {
        let stored = toml::Value::try_from(&self.stored).expect("settings are plain values");
        let running = toml::Value::try_from(running).expect("settings are plain values");
        self.fields
            .iter()
            .map(|(key, _)| *key)
            .filter(|key| stored.get(key) != running.get(key))
            .collect()
    }

    /// What the fields say, if it can work.
    fn read(&self, cx: &gpui::App) -> Result<StorageSettings, String> {
        let mut table = toml::Table::new();
        for (key, field) in &self.fields {
            let value = match field {
                Field::Toggle(on) => toml::Value::Boolean(*on),
                Field::Number(input) => {
                    let text = input.read(cx).text().trim().to_owned();
                    let number: i64 = text.parse().map_err(|_| {
                        tr!(
                            "{key}: \"{text}\" is not a whole number",
                            key = *key,
                            text = text.as_str()
                        )
                    })?;
                    if number < 0 {
                        return Err(tr!("{key}: must not be negative", key = *key));
                    }
                    toml::Value::Integer(number)
                }
            };
            table.insert((*key).to_owned(), value);
        }
        let settings: StorageSettings = table.try_into().map_err(|e| format!("{e}"))?;
        settings.check().map_err(|e| format!("{e:#}"))?;
        Ok(settings)
    }
}

impl Traffic {
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings_page.is_none() {
            let shown = StorageSettings::load(&self.settings_path())
                .unwrap_or_else(|_| self.engine.settings.clone());
            self.settings_page = Some(SettingsPage::new(&shown, cx));
            self.show_rules(cx);
        }
        cx.notify();
    }

    /// The list as it stands, in its field.
    fn show_rules(&mut self, cx: &mut Context<Self>) {
        let lines = self
            .engine
            .rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .to_lines();
        if let Some(page) = &self.settings_page {
            page.no_decrypt.update(cx, |input, cx| {
                input.set_text(&lines, lines.len()..lines.len(), cx)
            });
        }
    }

    /// Types into the list. For tests.
    #[doc(hidden)]
    pub fn set_no_decrypt(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(page) = &self.settings_page {
            page.no_decrypt.update(cx, |input, cx| {
                input.set_text(text, text.len()..text.len(), cx)
            });
        }
    }

    /// Each field is written when it is left, so that what is half typed
    /// never applies.
    fn watch_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = &self.settings_page else {
            return;
        };
        if !page.leaving.is_empty() {
            return;
        }
        let mut fields: Vec<(Option<&'static str>, gpui::FocusHandle)> = page
            .fields
            .iter()
            .filter_map(|(key, field)| match field {
                Field::Number(input) => Some((Some(*key), input.focus_handle(cx))),
                Field::Toggle(_) => None,
            })
            .collect();
        fields.push((None, page.no_decrypt.focus_handle(cx)));
        let leaving = fields
            .into_iter()
            .map(|(key, focus)| {
                cx.on_blur(&focus, window, move |this, _, cx| this.field_left(key, cx))
            })
            .collect();
        if let Some(page) = &mut self.settings_page {
            page.leaving = leaving;
        }
    }

    /// A field was left, or Enter pressed in it: what it says is written.
    fn field_left(&mut self, key: Option<&'static str>, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.settings_page {
            page.editing = key;
        }
        self.save_settings(cx);
    }

    /// The screen is left: what is still unwritten is written. A field that
    /// cannot be goes back to what is written, and the footer says so, rather
    /// than keep showing what does not apply.
    pub fn leave_settings(&mut self, cx: &mut Context<Self>) {
        self.save_settings(cx);
        let Some(page) = &self.settings_page else {
            return;
        };
        if let Some(problem) = page.problem.clone() {
            let stored = page.stored.clone();
            self.settings_left = Some(problem);
            self.settings_page = Some(SettingsPage::new(&stored, cx));
            self.show_rules(cx);
        }
    }

    /// Writes what the fields say, if it can work and is not what is written
    /// already, and applies what applies at once. The hosts are a file of
    /// their own, written whatever the other fields say.
    pub fn save_settings(&mut self, cx: &mut Context<Self>) {
        let path = self.settings_path();
        let Some(page) = &mut self.settings_page else {
            return;
        };
        let rules = HostRules::from_lines(page.no_decrypt.read(cx).text());
        let mut written = None;
        page.problem = None;
        match page.read(cx) {
            Ok(settings) if settings == page.stored => {}
            Ok(settings) => {
                let write = settings
                    .to_toml()
                    .map_err(|e| format!("{e:#}"))
                    .and_then(|text| {
                        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
                    });
                match write {
                    Ok(()) => {
                        page.stored = settings.clone();
                        page.undo = None;
                        written = Some(settings);
                    }
                    Err(problem) => page.problem = Some(problem),
                }
            }
            Err(problem) => page.problem = Some(problem),
        }
        if let Some(settings) = written {
            self.engine.apply_settings(&settings);
        }
        let now = self
            .engine
            .rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if rules != now {
            self.change_rules(|now| *now = rules, cx);
        }
        cx.notify();
    }

    /// Turns a switch, and writes it.
    #[doc(hidden)]
    pub fn toggle_setting(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(page) = &mut self.settings_page else {
            return;
        };
        for (name, field) in &mut page.fields {
            if *name == key
                && let Field::Toggle(on) = field
            {
                *on = !*on;
                page.editing = Some(*name);
            }
        }
        // Switched on now, so the question "has anything arrived since" is
        // asked from now. Only *other devices* of the ways in here applies at
        // once; SOCKS5 and the tunnel wait for a restart, and until then
        // nothing can arrive by them at all, so their moment is the engine's.
        if key == "other_devices" {
            match self.engine.other_devices.is_on() {
                true => {
                    self.on_since
                        .insert(Way::OtherDevices, std::time::Instant::now());
                }
                false => {
                    self.on_since.remove(&Way::OtherDevices);
                }
            }
        }
        self.save_settings(cx);
    }

    /// Every setting of the part of Settings open back to its default,
    /// written, with a way back.
    pub fn reset_settings(&mut self, cx: &mut Context<Self>) {
        let before = self.settings_page.as_ref().map(|page| page.stored.clone());
        let Some(now) = before.clone() else {
            return;
        };
        let keys: Vec<&str> = shown_in(self.layout.settings_section, self.layout.capture_way)
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.key))
            .collect();
        let defaults =
            toml::Value::try_from(StorageSettings::default()).expect("settings are plain values");
        let mut table = toml::Value::try_from(&now).expect("settings are plain values");
        for key in keys {
            table[key] = defaults[key].clone();
        }
        let reset = table.try_into().unwrap_or(now);
        self.replace_settings(&reset, cx);
        if let (Some(page), Some(before)) = (&mut self.settings_page, before)
            && page.stored != before
        {
            page.undo = Some(before);
        }
        cx.notify();
    }

    /// What there was before "Back to defaults", written again.
    pub fn undo_reset_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(before) = self
            .settings_page
            .as_mut()
            .and_then(|page| page.undo.take())
        {
            self.replace_settings(&before, cx);
        }
    }

    /// Shows `settings` in the fields and writes them.
    fn replace_settings(&mut self, settings: &StorageSettings, cx: &mut Context<Self>) {
        let stored = self.settings_page.as_ref().map(|page| page.stored.clone());
        let mut page = SettingsPage::new(settings, cx);
        if let Some(stored) = stored {
            page.stored = stored;
        }
        self.settings_page = Some(page);
        self.show_rules(cx);
        self.save_settings(cx);
    }

    /// Puts the cursor in a number field. For tests.
    #[doc(hidden)]
    pub fn focus_setting(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        // Its part open, and its tab, as a field is only reached where it is
        // shown.
        if let Some(group) = groups()
            .into_iter()
            .find(|group| group.rows.iter().any(|row| row.key == key))
        {
            self.layout.settings_section = group.at;
            if let Some(way) = group.way {
                self.layout.capture_way = way;
            }
        }
        let Some(page) = &self.settings_page else {
            return;
        };
        for (name, field) in &page.fields {
            if *name == key
                && let Field::Number(input) = field
            {
                window.focus(&input.focus_handle(cx), cx);
            }
        }
    }

    /// Types into a number field. For tests.
    #[doc(hidden)]
    pub fn set_setting(&mut self, key: &str, text: &str, cx: &mut Context<Self>) {
        let Some(page) = &self.settings_page else {
            return;
        };
        for (name, field) in &page.fields {
            if *name == key
                && let Field::Number(input) = field
            {
                input.update(cx, |input, cx| {
                    input.set_text(text, text.len()..text.len(), cx)
                });
            }
        }
    }

    /// Opens a part of Settings, kept for next time.
    pub fn show_settings_section(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.layout.settings_section = section;
        if let Some(page) = &mut self.settings_page {
            // What was said was about the part left.
            page.undo = None;
            page.problem = None;
            page.editing = None;
        }
        self.save_layout();
        self.went();
        cx.notify();
    }

    /// Opens a way in's tab under Capture.
    ///
    /// The same forgetting as leaving a part of Settings: what was said about
    /// the tab left is not about the one arrived at.
    pub fn show_way(&mut self, way: Way, cx: &mut Context<Self>) {
        self.layout.capture_way = way;
        if let Some(page) = &mut self.settings_page {
            page.undo = None;
            page.problem = None;
            page.editing = None;
        }
        self.save_layout();
        self.went();
        cx.notify();
    }

    /// The tabs of Settings › Capture: one for each way traffic reaches
    /// tracewing, so that no setting is left looking as if it belonged to
    /// another way in, or to none.
    fn render_capture_tabs(&self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let open = self.layout.capture_way;
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(ui(6.))
            .children(Way::ALL.into_iter().map(|way| {
                let on = way == open;
                div()
                    .id(("capture-way", way as usize))
                    .h(ui(32.))
                    .px(ui(14.))
                    .rounded(ui(16.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .reachable()
                    .when(on, |d| {
                        d.bg(rgb(c.accent))
                            .text_color(rgb(c.on_accent))
                            .font_weight(FontWeight::BOLD)
                    })
                    .when(!on, |d| {
                        d.bg(rgb(c.chip))
                            .text_color(rgb(c.on_chip))
                            .hover(|d| d.bg(rgb(c.surface)))
                    })
                    .child(way.name())
                    .on_click(cx.listener(move |this, _, _, cx| this.show_way(way, cx)))
            }))
    }

    /// What a way in catches and what it does not, as its tab says them.
    ///
    /// Out of the view for the same reason as [`trust_for`], and one more: the
    /// Apps pair is the only one that turns on a setting rather than on the
    /// platform, so it is the only one that can go stale while everything
    /// still compiles -- and a pair asked for with the setting each way is
    /// something a test can hold, where a reading of the window is not.
    ///
    /// Here, just above the view, rather than beside `trust_for`: the
    /// message catalogues are in the order the code says them, and a pair of
    /// some twenty-five sentences moved eight hundred lines up reorders every
    /// one of fifteen files -- a diff nobody can review, in the files every
    /// pull request touches.
    fn catches_for(way: Way, addr: &str, http3: bool) -> (String, String) {
        match way {
            Way::SystemProxy => (
                tr!(
                    "Apps that use this computer's own proxy setting: most browsers, and anything on the system's networking."
                ),
                tr!(
                    "Java, which reads no proxy setting of the system's; programs with a setting of their own; and on macOS, anything sent to localhost."
                ),
            ),
            Way::Browser => (
                tr!(
                    "Chrome, Edge, Brave, Opera, Chromium and Firefox, each in a profile of tracewing's — the system proxy left alone."
                ),
                tr!("Safari, which has no profile of its own: use the system proxy for it."),
            ),
            Way::Terminal => (
                tr!(
                    "What is started in it: curl, git, Python, Go, Deno, Bun, and AI agents such as Claude Code and Codex."
                ),
                tr!(
                    "Node.js before 22.21, and 23 to 24.4; Java, which reads neither the proxy nor the trust variables."
                ),
            ),
            Way::OtherDevices => (
                tr!(
                    "A phone or another computer on your network, once it is told {addr} as its proxy by hand.",
                    addr = addr.to_owned()
                ),
                tr!(
                    "Whatever on that device ignores its proxy setting, which on a phone is most apps: a tunnel catches those."
                ),
            ),
            Way::Socks5 => (
                tr!(
                    "What speaks SOCKS: anything reading ALL_PROXY, git with a socks5h:// proxy, Firefox's own SOCKS setting, proxychains."
                ),
                tr!(
                    "Programs that read only HTTP_PROXY and HTTPS_PROXY, and anything with no SOCKS setting of its own."
                ),
            ),
            Way::WireGuard => (
                tr!(
                    "Everything a phone sends once it joins the tunnel, apps that ignore the phone's proxy setting included."
                ),
                tr!(
                    "Apps that trust only their own certificates, and anything sent before tracewing's authority is installed on the phone."
                ),
            ),
            // Two ways in wear this tab, and each needs its own pair: the
            // macOS one is the network extension, the other is a route into
            // tracewing's own adapter. The sentences below used to be the
            // extension's on both, which said "macOS" on a Windows tab --
            // true, and about the wrong page.
            Way::Apps if cfg!(target_os = "macos") => (
                tr!(
                    "Whatever a chosen app sends over TCP, whether or not it reads any proxy setting: a browser, an Electron app, Node of any version, Java."
                ),
                tr!(
                    "UDP, so HTTP/3; connections to this computer itself, which macOS does not hand over; and go run, which builds its program somewhere new each time: add the folder go build -o writes to instead."
                ),
            ),
            // What this one catches is not fixed. "Read chosen programs'
            // HTTP/3" (Settings > Network) sends a chosen program's QUIC to
            // a door of tracewing's own instead of refusing it. Until 0.1.0
            // this tab had one pair, and said UDP was never caught whatever
            // the setting -- which denied in the present tense what the app
            // was doing the moment that setting went on: the failure this
            // file's other comments are about, in the one place that is a
            // setting away rather than a platform away, so no `cfg!` would
            // have caught it.
            //
            // Both pairs name HTTP/3 as what is refused and the rest of UDP
            // as what is passed on, because those are two different fates:
            // the stack turns away a datagram to port 443 (`datagrams.rs`),
            // which is what makes a program fall back to HTTP/2, and only
            // the rest goes through. The old sentence joined them with "so",
            // as if passing UDP on were why HTTP/3 failed.
            //
            // The macOS arm above needs no such pair: `local_source::catching`
            // is Windows and Linux only, so there is no HTTP/3 of ours to read
            // there and "UDP, so HTTP/3" stays true however that setting is
            // left.
            Way::Apps if http3 => (
                tr!(
                    "Whatever a chosen program sends over TCP, whether or not it reads any proxy setting, and its HTTP/3 with it: this machine's traffic comes through tracewing's own adapter, and what is kept is the programs you chose."
                ),
                tr!(
                    "The HTTP/3 of a program nobody chose, which is refused so that it falls back to HTTP/2; other UDP, which is passed straight through; connections to this computer itself, which never leave it; and go run, which builds its program somewhere new each time: add the folder go build -o writes to instead."
                ),
            ),
            Way::Apps => (
                tr!(
                    "Whatever a chosen program sends over TCP, whether or not it reads any proxy setting: this machine's traffic comes through tracewing's own adapter, and what is kept is the programs you chose."
                ),
                tr!(
                    "HTTP/3, which is refused so that the program falls back to HTTP/2 (Settings › Network can read it instead); other UDP, which is passed straight through; connections to this computer itself, which never leave it; and go run, which builds its program somewhere new each time: add the folder go build -o writes to instead."
                ),
            ),
        }
    }

    /// What the open tab's way in catches, what it does not, and whatever it
    /// is turned on or off by that is not a setting in `settings.toml`.
    ///
    /// The second line is the one worth the room. Somebody whose Java program
    /// shows nothing has a way in that cannot catch it, not a tracewing that
    /// is broken, and the tab is where that can be said before the issue
    /// tracker hears about it.
    fn render_way(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let way = self.layout.capture_way;
        let addr = self.engine.addr.to_string();
        // Asked of `live` rather than of the stored settings because the
        // setting applies at once, and this tab is read to find out what is
        // happening now.
        let (catches, misses) = Self::catches_for(way, &addr, self.engine.live.capture_http3());
        let (trusts, goes_to) = trust_for(way);
        // When this way in became able to deliver anything. A switch the
        // window owns is remembered when it is flipped; one that needs a
        // restart could only have started delivering when the engine did,
        // and only if it was actually served — a port something else holds
        // is not a way in that has gone quiet, it is a way in that is not
        // there, and the tab already says so above.
        let since = match way {
            Way::SystemProxy | Way::OtherDevices | Way::Apps => self.on_since.get(&way).copied(),
            Way::Socks5 => self.engine.socks.map(|_| self.engine.started),
            Way::WireGuard => self.engine.wireguard.map(|_| self.engine.started),
            Way::Browser | Way::Terminal => None,
        };
        let quiet = gone_quiet(way, since, &self.engine.arrivals, Instant::now()).then(|| {
            match way {
                // The three that share the proxy port are named, because this
                // count cannot tell them apart and a line that implied it
                // could would be a lie about which one is failing.
                Way::SystemProxy => tr!(
                    "Nothing has arrived on the proxy port since this was switched on — not by the system proxy, a browser of tracewing's own, or a terminal, which all come in there. A program that was told to use the proxy and ignores it looks exactly like this."
                ),
                // Listed rather than caught by `_`, so that a way in added
                // later has to say whether this sentence is true of it. It
                // is true of all of these; a way in that shares a listener
                // with another, as the proxy port's three do, would need its
                // own line saying so.
                Way::Browser
                | Way::Terminal
                | Way::OtherDevices
                | Way::Socks5
                | Way::WireGuard
                | Way::Apps => tr!(
                    "Nothing has arrived by this since it was switched on. A program that was told to use it and ignores it looks exactly like this."
                ),
            }
        });

        let trust_link = goes_to.map(|section| {
            crate::setup_page::button("capture-trust", section.name(), false)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.show_settings_section(section, cx)),
                )
                .into_any_element()
        });

        // What turns this way in on, where that is not a row of settings.
        let control: Option<gpui::AnyElement> = match way {
            Way::SystemProxy => {
                let on = matches!(self.system.proxy, ProxySwitch::On(_));
                let working = matches!(self.system.proxy, ProxySwitch::Working);
                // Some desktops have no setting tracewing can change, and a
                // button that cannot work is worse than a sentence saying so.
                let no_setting = self
                    .system
                    .environment
                    .no_system_proxy
                    .clone()
                    .filter(|_| !on);
                (!working && no_setting.is_none()).then(|| {
                    crate::setup_page::button(
                        "capture-proxy",
                        if on { tr!("Turn off") } else { tr!("Turn on") },
                        !on,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_system_proxy(cx)))
                    .into_any_element()
                })
            }
            // A browser and a terminal are opened, not switched on, and Set
            // up is where that is done: one button rather than a second copy
            // of the list of browsers.
            Way::Browser | Way::Terminal => Some(
                crate::setup_page::button("capture-setup", tr!("Open from Set up"), false)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_screen(crate::traffic::Screen::Setup, cx)
                    }))
                    .into_any_element(),
            ),
            Way::OtherDevices | Way::Socks5 | Way::WireGuard | Way::Apps => None,
        };
        // The apps and their switch, on the Mac; elsewhere a line saying
        // why there is none, rather than a switch that could not work.
        // The one thing on a tab that is not a sentence: what a way in that
        // chooses programs shows for choosing them. Listed rather than caught
        // by `_`, because `_ => None` gives a way in added later a tab whose
        // whole subject is which programs, with nothing on it to choose any —
        // and the compiler, which asks about every other `Way` match in this
        // file, would say nothing at all about that one.
        // Whether this tab describes a way in at all. A tab for one this
        // system does not have must not say what it catches in the present
        // tense: every one of those sentences is true **of the way in** and
        // about the wrong page here, which is the failure no assertion
        // catches.
        let describes = way != Way::Apps
            || cfg!(target_os = "macos")
            || self.local.support != crate::local_apps::Support::NotOnThisSystem;
        let apps = match way {
            Way::Apps if cfg!(target_os = "macos") => Some(self.render_mac_apps(cx)),
            // Windows and Linux: the same promise as the macOS tab -- choose
            // the programs, see those programs -- reached by route rather
            // than by an extension. The sentence that used to be here said
            // "macOS only", which was true of the extension and about the
            // wrong tab the moment this one existed.
            Way::Apps => Some(self.render_local_apps(cx)),
            Way::SystemProxy
            | Way::Browser
            | Way::Terminal
            | Way::OtherDevices
            | Way::Socks5
            | Way::WireGuard => None,
        };
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(16.))
                    .children(describes.then(|| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(ui(4.))
                            .child(eyebrow(tr!("Catches")))
                            .child(catches)
                            .child(eyebrow(tr!("Does not catch")))
                            .child(div().text_color(rgb(c.muted)).child(misses))
                            .child(eyebrow(tr!("Needs to trust tracewing")))
                            .child(div().text_color(rgb(c.muted)).child(trusts))
                            .children(
                                quiet.map(|says| {
                                    div().mt(ui(4.)).text_color(rgb(c.warn)).child(says)
                                }),
                            )
                    }))
                    .children(control),
            )
            // In a row of its own: a column stretches what is in it, and a
            // button the width of the card reads as a banner rather than as
            // something to press.
            .children(
                trust_link
                    .filter(|_| describes)
                    .map(|button| div().flex().flex_row().child(button)),
            )
            .children(apps)
    }

    /// The body text size, `None` for the default. Everything grows with it.
    pub fn set_text_size(&mut self, size: Option<u8>, cx: &mut Context<Self>) {
        self.layout.text_size = size.filter(|s| *s != crate::text_size::DEFAULT);
        self.save_layout();
        cx.notify();
    }

    /// The interface in tracewing's own font, or the system's.
    pub fn set_ui_font(&mut self, system: bool, cx: &mut Context<Self>) {
        self.layout.ui_font = system.then(|| crate::layout::SYSTEM_FONT.to_owned());
        self.save_layout();
        cx.notify();
    }

    /// The code in `font`, `None` for tracewing's own.
    pub fn set_code_font(&mut self, font: Option<String>, cx: &mut Context<Self>) {
        self.layout.code_font = font.filter(|f| f != crate::MONO_FONT);
        self.save_layout();
        cx.notify();
    }

    /// The monospaced families on this computer, tracewing's own first: those
    /// whose i, M and W are as wide. Found once.
    pub fn mono_fonts(&mut self, window: &Window) -> Vec<String> {
        if let Some(found) = &self.mono_fonts {
            return found.clone();
        }
        let names = self.installed_fonts(window).to_vec();
        let text = window.text_system();
        let mut found: Vec<String> = names
            .into_iter()
            .filter(|name| !name.starts_with('.') && name != crate::MONO_FONT)
            .filter(|name| {
                let id = text.resolve_font(&gpui::font(name.clone()));
                let wide = |c| {
                    text.advance(id, gpui::px(12.), c)
                        .ok()
                        .map(|s| f32::from(s.width))
                };
                match (wide('i'), wide('M'), wide('W')) {
                    (Some(i), Some(m), Some(w)) => {
                        i > 0. && (i - m).abs() < 0.01 && (m - w).abs() < 0.01
                    }
                    _ => false,
                }
            })
            .collect();
        found.sort_by_key(|name| name.to_lowercase());
        found.dedup();
        found.insert(0, crate::MONO_FONT.to_owned());
        self.mono_fonts = Some(found.clone());
        found
    }

    /// The fonts: the interface's, tracewing's own or the system's, and the
    /// code's, of the monospaced fonts on this computer. Each is drawn in
    /// itself.
    fn render_fonts(&mut self, window: &Window, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let system = crate::system_ui_font(self.installed_fonts(window));
        let monos = self.mono_fonts(window);
        let ui_system = self.layout.ui_font.as_deref() == Some(crate::layout::SYSTEM_FONT);
        let code = self
            .layout
            .code_font
            .clone()
            .unwrap_or_else(|| crate::MONO_FONT.to_owned());
        let chip = |id: gpui::ElementId, label: String, face: String, on: bool| {
            div()
                .id(id)
                .h(ui(30.))
                .px(ui(12.))
                .rounded(ui(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .font_family(face)
                .text_size(ui(13.))
                .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let row = |name: String| {
            div()
                .px(ui(6.))
                .pt(ui(4.))
                .text_size(ui(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(name)
        };
        let chips = || div().px(ui(6.)).flex().flex_row().flex_wrap().gap(ui(6.));
        let own = chip(
            "ui-font-own".into(),
            tr!("{name} (default)", name = crate::FONT),
            crate::FONT.to_owned(),
            !ui_system,
        )
        .on_click(cx.listener(|this, _, _, cx| this.set_ui_font(false, cx)));
        let theirs = system.map(|name| {
            chip(
                "ui-font-system".into(),
                tr!("System ({name})", name = name.as_str()),
                name.clone(),
                ui_system,
            )
            .on_click(cx.listener(|this, _, _, cx| this.set_ui_font(true, cx)))
        });
        let codes = monos.into_iter().enumerate().map(|(i, name)| {
            let label = if name == crate::MONO_FONT {
                tr!("{name} (default)", name = name.as_str())
            } else {
                name.clone()
            };
            let on = name == code;
            let pick = name.clone();
            chip(("code-font", i).into(), label, name, on).on_click(
                cx.listener(move |this, _, _, cx| this.set_code_font(Some(pick.clone()), cx)),
            )
        });
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Fonts")).px(ui(6.)))
            .child(row(tr!("Interface")))
            .child(chips().child(own).children(theirs))
            .child(row(tr!("Code")))
            .child(chips().children(codes))
            .child(
                div()
                    .px(ui(6.))
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!(
                        "For code, the fonts on this computer whose letters are all as wide. Applies at once."
                    )),
            )
    }

    /// The text size, from small to large, the default marked.
    fn render_text_size(&self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let chosen = self.text_size();
        let chips = crate::text_size::CHOICES.map(|size| {
            let on = f32::from(size) == chosen;
            let label = if size == crate::text_size::DEFAULT {
                tr!("{size} (default)", size = size)
            } else {
                size.to_string()
            };
            div()
                .id(("text-size", size as usize))
                .h(ui(30.))
                .px(ui(12.))
                .rounded(ui(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.5))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.set_text_size(Some(size), cx)))
        });
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Text size")).px(ui(6.)))
            .child(
                div()
                    .px(ui(6.))
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(ui(6.))
                    .children(chips),
            )
            .child(
                div()
                    .px(ui(6.))
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!(
                        "In px, for the text of lists and bodies. Everything grows with it. Applies at once."
                    )),
            )
    }

    /// The look: the system's, light or dark, as the title bar's switch has it;
    /// and how each looks, for those that can be shown.
    fn render_appearance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let chosen = self.layout.appearance;
        let chip = |look: Appearance| {
            let on = look == chosen;
            div()
                .id(("appearance", look as usize))
                .h(ui(30.))
                .px(ui(12.))
                .rounded(ui(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.5))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(look.name())
                .on_click(
                    cx.listener(move |this, _, window, cx| this.set_appearance(look, window, cx)),
                )
        };
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Appearance")).px(ui(6.)))
            .child(
                div()
                    .px(ui(6.))
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(ui(6.))
                    .children([Appearance::System, Appearance::Light, Appearance::Dark].map(chip)),
            )
            .when(chosen != Appearance::Dark, |d| {
                d.child(style_row(tr!("Light")))
                    .child(
                        style_chips().children(LightStyle::ALL.into_iter().map(|style| {
                            let name = if style == LightStyle::default() {
                                tr!("{name} (default)", name = style.name())
                            } else {
                                style.name()
                            };
                            style_chip(
                                ("light-style", style as usize).into(),
                                name,
                                style.palette(),
                                style == self.layout.light_style,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| this.set_light_style(style, window, cx),
                            ))
                        })),
                    )
            })
            .when(chosen != Appearance::Light, |d| {
                d.child(style_row(tr!("Dark")))
                    .child(
                        style_chips().children(DarkStyle::ALL.into_iter().map(|style| {
                            let name = if style == DarkStyle::default() {
                                tr!("{name} (default)", name = style.name())
                            } else {
                                style.name()
                            };
                            style_chip(
                                ("dark-style", style as usize).into(),
                                name,
                                style.palette(),
                                style == self.layout.dark_style,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| this.set_dark_style(style, window, cx),
                            ))
                        })),
                    )
            })
    }

    /// The hosts whose traffic goes through unread.
    fn render_no_decrypt(&self) -> gpui::Div {
        let c = colors();
        let page = self.settings_page.as_ref().expect("made");
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Hosts not to decrypt")).px(ui(6.)))
            .child(
                div()
                    .px(ui(6.))
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.")),
            )
            .child(
                div()
                    .p(ui(10.))
                    .rounded(ui(12.))
                    .bg(rgb(c.input))
                    .font_family(crate::mono_font())
                    .font_features(gpui::FontFeatures::disable_ligatures())
                    .child(page.no_decrypt.clone()),
            )
    }

    /// The menu of Settings' parts.
    fn render_settings_menu(&self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let open = self.layout.settings_section;
        let items = SettingsSection::ALL.into_iter().map(|section| {
            let on = section == open;
            div()
                .id(("settings-section", section as usize))
                .h(ui(38.))
                .px(ui(14.))
                .rounded(ui(19.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .when(on, |d| {
                    d.bg(rgb(c.tonal))
                        .text_color(rgb(c.on_tonal))
                        .font_weight(FontWeight::BOLD)
                })
                .when(!on, |d| {
                    d.font_weight(FontWeight::MEDIUM)
                        .hover(|d| d.bg(rgb(c.surface)))
                })
                .child(section.name())
                .on_click(
                    cx.listener(move |this, _, _, cx| this.show_settings_section(section, cx)),
                )
        });
        div()
            .w(ui(200.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(ui(2.))
            .child(
                div()
                    .px(ui(14.))
                    .pb(ui(10.))
                    .text_size(ui(20.))
                    .font_weight(FontWeight::BOLD)
                    .child(tr!("Settings")),
            )
            .children(items)
    }

    pub(crate) fn render_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = colors();
        if self.settings_page.is_none() {
            self.open_settings(cx);
        }
        self.watch_fields(window, cx);
        let section = self.layout.settings_section;
        let menu = self.render_settings_menu(cx);
        // What the part open shows besides its settings, before them or after.
        let (before, after): (Vec<gpui::AnyElement>, Vec<gpui::AnyElement>) = match section {
            SettingsSection::General => (
                vec![
                    self.render_language(cx).into_any_element(),
                    self.render_appearance(cx).into_any_element(),
                    self.render_text_size(cx).into_any_element(),
                    self.render_fonts(window, cx).into_any_element(),
                ],
                Vec::new(),
            ),
            SettingsSection::Capture => (
                vec![
                    self.render_capture_tabs(cx).into_any_element(),
                    self.render_way(cx).into_any_element(),
                ],
                Vec::new(),
            ),
            SettingsSection::Network => (
                Vec::new(),
                vec![
                    self.render_relay(cx).into_any_element(),
                    self.render_no_decrypt().into_any_element(),
                ],
            ),
            SettingsSection::Certificates => (
                Vec::new(),
                vec![
                    self.render_trusted(cx).into_any_element(),
                    self.render_client_certs(cx).into_any_element(),
                    self.render_company_ca(cx).into_any_element(),
                    self.render_tls_keys(cx).into_any_element(),
                ],
            ),
            SettingsSection::Decoding => (
                vec![
                    self.render_protos(cx).into_any_element(),
                    self.render_abis(cx).into_any_element(),
                    self.render_readings(cx).into_any_element(),
                ],
                Vec::new(),
            ),
            SettingsSection::Storage => (Vec::new(), Vec::new()),
            SettingsSection::Ai => (Vec::new(), vec![self.render_ai_tools().into_any_element()]),
            // Who this is and whether a newer one is out, then the switch
            // that says whether to look, then the rest.
            SettingsSection::About => (
                vec![self.render_identity(cx).into_any_element()],
                vec![self.render_about().into_any_element()],
            ),
        };
        let page = self.settings_page.as_ref().expect("just made");
        let waiting = page.waiting(&self.engine.settings);
        // A port the settings asked for that could not be opened. Said under
        // the switch that asked for it, and said as it came: the address and
        // the reason the system gave, which no wrapper of ours improves.
        let socks_problem = self.engine.socks_problem.clone();
        let wireguard_problem = self.engine.wireguard_problem.clone();
        // Asked for now rather than kept: the addresses a phone could reach
        // are found after the proxy starts, so this is empty if it is read
        // too early. Settings is drawn long after, which is why it is read
        // here and not in `Engine::start`.
        let wireguard_config = self.engine.wireguard_config();
        // A problem shows under the setting being changed, or else at the top.
        let problem_under = page.problem.as_ref().and(page.editing);
        let field_of = |key: &str| {
            page.fields
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, field)| field)
                .expect("every row has a field")
        };
        let shown: Vec<(String, Vec<Row>)> = shown_in(section, self.layout.capture_way)
            .into_iter()
            .map(|group| (group.title, group.rows))
            .collect();
        let has_rows = !shown.is_empty();
        // Those waiting for a restart that this part holds.
        let waiting_here = waiting
            .iter()
            .filter(|key| {
                shown
                    .iter()
                    .any(|(_, rows)| rows.iter().any(|row| row.key == **key))
            })
            .count();
        let groups = shown.into_iter().map(|(title, rows)| {
            div()
                .flex()
                .flex_col()
                .gap(ui(2.))
                .p(ui(14.))
                .rounded(ui(22.))
                .bg(rgb(c.surface))
                .child(eyebrow(title).px(ui(6.)).pb(ui(6.)))
                .children(rows.into_iter().map(|row| {
                    let key = row.key;
                    let control = match field_of(key) {
                        Field::Toggle(on) => {
                            let on = *on;
                            div()
                                .id(key)
                                .w(ui(44.))
                                .h(ui(24.))
                                .p(ui(3.))
                                .rounded(ui(12.))
                                .flex()
                                .cursor_pointer()
                                .reachable()
                                .when(on, |d| d.bg(rgb(c.accent)).justify_end())
                                .when(!on, |d| d.bg(rgb(c.input)))
                                .child(div().size(ui(18.)).rounded(ui(9.)).bg(rgb(if on {
                                    c.on_accent
                                } else {
                                    c.muted
                                })))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.toggle_setting(key, cx)),
                                )
                                .into_any_element()
                        }
                        Field::Number(input) => div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(ui(8.))
                            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                                if ev.keystroke.key == "enter" {
                                    this.field_left(Some(key), cx);
                                }
                            }))
                            .child(
                                div()
                                    .w(ui(96.))
                                    .h(ui(32.))
                                    .px(ui(10.))
                                    .rounded(ui(12.))
                                    .flex()
                                    .items_center()
                                    .bg(rgb(c.input))
                                    .font_family(crate::mono_font())
                                    .font_features(gpui::FontFeatures::disable_ligatures())
                                    .child(input.clone()),
                            )
                            .child(
                                div()
                                    .w(ui(28.))
                                    .text_size(ui(12.))
                                    .text_color(rgb(c.muted))
                                    .child(row.unit),
                            )
                            .into_any_element(),
                    };
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(ui(16.))
                        .min_h(ui(52.))
                        .px(ui(6.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(ui(8.))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(row.label),
                                        )
                                        .when(waiting.contains(&key), |d| {
                                            d.child(
                                                div()
                                                    .px(ui(8.))
                                                    .rounded(ui(8.))
                                                    .text_size(ui(11.))
                                                    .bg(rgb(c.warn_ground))
                                                    .text_color(rgb(c.warn))
                                                    .child(tr!("After a restart")),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(ui(12.))
                                        .text_color(rgb(c.muted))
                                        .child(row.help),
                                )
                                .when(key == "socks", |d| {
                                    d.children(socks_problem.clone().map(|why| {
                                        div().text_size(ui(12.)).text_color(rgb(c.error)).child(why)
                                    }))
                                })
                                .when(key == "wireguard", |d| {
                                    d.children(wireguard_problem.clone().map(|why| {
                                        div().text_size(ui(12.)).text_color(rgb(c.error)).child(why)
                                    }))
                                    // What the phone is given: a code for
                                    // the WireGuard app's "add tunnel", and
                                    // the same text to paste where a camera
                                    // is no use. 300 is not a round number
                                    // — a real configuration is 65 squares
                                    // across, which at that size is four
                                    // pixels a square, and a phone's camera
                                    // wants about that much.
                                    .children(
                                        wireguard_config.clone().map(|config| {
                                            div()
                                                .mt(ui(8.))
                                                .flex()
                                                .flex_wrap()
                                                .gap(ui(12.))
                                                .items_start()
                                                .children(crate::qr::qr_code(&config, 300.))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w(ui(240.))
                                                        .p(ui(8.))
                                                        .bg(rgb(c.code))
                                                        .rounded(ui(4.))
                                                        .font_family("monospace")
                                                        .text_size(ui(11.))
                                                        .text_color(rgb(c.text))
                                                        .child(config),
                                                )
                                        }),
                                    )
                                })
                                .when(problem_under == Some(key), |d| {
                                    d.children(page.problem.clone().map(|problem| {
                                        div()
                                            .text_size(ui(12.))
                                            .text_color(rgb(c.error))
                                            .child(problem)
                                    }))
                                }),
                        )
                        .child(control)
                }))
        });
        let button = |id: &'static str, label: String, loud: bool| {
            div()
                .id(id)
                .h(ui(36.))
                .px(ui(16.))
                .rounded(ui(18.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .font_weight(FontWeight::BOLD)
                .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let status = match (&page.problem, problem_under) {
            (Some(problem), None) => Some((problem.clone(), c.error)),
            _ if page.undo.is_some() => {
                Some((tr!("Every setting here is back to its default."), c.muted))
            }
            _ if waiting_here > 0 => Some((
                tr_n!(
                    waiting_here,
                    "{n} change takes effect when tracewing starts again.",
                    "{n} changes take effect when tracewing starts again."
                ),
                c.warn,
            )),
            _ => None,
        };
        let undo = page.undo.is_some();
        let content = div()
            // Each part keeps where it was scrolled to.
            .id(("settings", section as usize))
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(ui(10.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(10.))
                    .min_h(ui(36.))
                    .child(
                        div()
                            .text_size(ui(20.))
                            .font_weight(FontWeight::BOLD)
                            .child(section.name()),
                    )
                    .child(div().flex_1())
                    .children(status.map(|(text, color)| div().text_color(rgb(color)).child(text)))
                    .when(undo, |d| {
                        d.child(
                            button("settings-undo", tr!("Undo"), true).on_click(
                                cx.listener(|this, _, _, cx| this.undo_reset_settings(cx)),
                            ),
                        )
                    })
                    .when(!undo && has_rows, |d| {
                        d.child(
                            button("settings-defaults", tr!("Back to defaults"), false)
                                .on_click(cx.listener(|this, _, _, cx| this.reset_settings(cx))),
                        )
                    }),
            )
            .children(before)
            .children(groups)
            .children(after);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_row()
            .gap(ui(10.))
            .child(menu)
            .child(content)
    }

    /// The fields that are always read a certain way, to look over and to
    /// forget. They are remembered from the value itself, in the menu a right
    /// click opens; this is where they can be found again afterwards.
    fn render_readings(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let rules: Vec<(String, String, String)> = self
            .inspector
            .remembered
            .borrow()
            .all()
            .map(|(host, field, read_as)| (host.to_owned(), field.to_owned(), read_as.to_owned()))
            .collect();
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Fields always read a certain way")).px(ui(6.)))
            .child(
                div()
                    .px(ui(6.))
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!(
                        "A value read in the inspector can be kept that way for its field, and \
                         every later flow from that host reads itself. They are kept in \
                         readings.toml in the data folder."
                    )),
            )
            .when(rules.is_empty(), |d| {
                d.child(
                    div()
                        .px(ui(6.))
                        .text_color(rgb(c.faint))
                        .child(tr!("Nothing is remembered yet.")),
                )
            })
            .children(rules.into_iter().map(|(host, field, read_as)| {
                let forgetting = (host.clone(), field.clone());
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(10.))
                    .px(ui(6.))
                    .min_h(ui(34.))
                    .child(
                        div()
                            .w(gpui::relative(0.34))
                            .flex_shrink_0()
                            .font_family(crate::mono_font())
                            .text_size(ui(12.))
                            .font_weight(FontWeight::BOLD)
                            .child(field.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(crate::mono_font())
                            .text_size(ui(12.))
                            .text_color(rgb(c.muted))
                            .child(if host.is_empty() {
                                tr!("{reading} · every host", reading = read_as.as_str())
                            } else {
                                tr!(
                                    "{reading} · {host}",
                                    reading = read_as.as_str(),
                                    host = host.as_str()
                                )
                            }),
                    )
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("forget-{host}-{field}")))
                            .px(ui(12.))
                            .py(ui(5.))
                            .rounded(ui(16.))
                            .cursor_pointer()
                            .reachable()
                            .text_size(ui(12.))
                            .font_weight(FontWeight::BOLD)
                            .bg(rgb(c.chip))
                            .text_color(rgb(c.on_chip))
                            .child(tr!("Forget"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let (host, field) = forgetting.clone();
                                let _ =
                                    this.inspector.remembered.borrow_mut().forget(&host, &field);
                                cx.notify();
                            })),
                    )
            }))
    }

    /// The authorities trusted for servers from the folder, which a restart
    /// takes in.
    fn render_trusted(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .h(ui(32.))
                .px(ui(14.))
                .rounded(ui(16.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.5))
                .font_weight(FontWeight::BOLD)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(label)
        };
        let line = |name: String, file: String, color: u32| {
            div()
                .px(ui(6.))
                .flex()
                .flex_row()
                .gap(ui(10.))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(color))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(ui(12.))
                        .text_color(rgb(c.muted))
                        .child(file),
                )
        };
        let found = &self.trusted;
        let changed = found.paths() != self.engine.extra_roots;
        let folder = crate::trusted::folder(&self.data_dir);
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Authorities trusted for servers")).px(ui(6.)))
            .child(div().px(ui(6.)).text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                "tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.",
                folder = folder.display().to_string()
            )))
            .children(found.good.iter().map(|authority| {
                let file = authority
                    .file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                line(authority.name.clone(), file, c.text)
            }))
            .children(found.bad.iter().map(|(file, why)| {
                line(
                    file.clone(),
                    tr!("Left out: {why}", why = why.clone()),
                    c.error,
                )
            }))
            .when(found.good.is_empty() && found.bad.is_empty(), |d| {
                d.child(div().px(ui(6.)).text_color(rgb(c.muted)).child(tr!("None yet.")))
            })
            .when(changed, |d| {
                d.child(div().px(ui(6.)).text_color(rgb(c.warn)).child(tr!(
                    "The folder has changed since tracewing started: restart it to use what it holds now."
                )))
            })
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(ui(8.))
                    .child(button("trusted-open", tr!("Open the folder")).on_click(cx.listener(
                        move |_, _, _, cx| {
                            let _ = std::fs::create_dir_all(&folder);
                            cx.open_with_system(&folder);
                        },
                    )))
                    .child(button("trusted-read", tr!("Read it again")).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.trusted = crate::trusted::read(&this.data_dir);
                            cx.notify();
                        },
                    ))),
            )
    }
}

impl Traffic {
    /// Writes TLS secrets to the file in the data folder, or stops.
    pub fn switch_tls_keys(&mut self, cx: &mut Context<Self>) {
        let keys = &self.engine.tls_keys;
        if keys.path().is_some() {
            keys.stop();
        } else {
            // What went wrong shows under it.
            let _ = keys.start(&self.data_dir.join(TLS_KEYS_FILE));
        }
        cx.notify();
    }

    /// The TLS secrets written for Wireshark: switched on here, off at the next
    /// start.
    fn render_tls_keys(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let keys = &self.engine.tls_keys;
        let writing = keys.path();
        let on = writing.is_some();
        let path = writing.unwrap_or_else(|| self.data_dir.join(TLS_KEYS_FILE));
        let shown = path.display().to_string();
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .h(ui(32.))
                .px(ui(14.))
                .rounded(ui(16.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.5))
                .font_weight(FontWeight::BOLD)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(label)
        };
        let switch = div()
            .id("tls-keys")
            .w(ui(44.))
            .h(ui(24.))
            .p(ui(3.))
            .flex_shrink_0()
            .rounded(ui(12.))
            .flex()
            .cursor_pointer()
            .reachable()
            .when(on, |d| d.bg(rgb(c.accent)).justify_end())
            .when(!on, |d| d.bg(rgb(c.input)))
            .child(div().size(ui(18.)).rounded(ui(9.)).bg(rgb(if on {
                c.on_accent
            } else {
                c.muted
            })))
            .on_click(cx.listener(|this, _, _, cx| this.switch_tls_keys(cx)));
        let folder = path.parent().map(|p| p.to_owned());
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("TLS keys for Wireshark")).px(ui(6.)))
            .child(div().px(ui(6.)).text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                "Writes the secrets of each HTTPS connection, from apps to tracewing and from it to servers, to a file Wireshark reads: Preferences, Protocols, TLS, (Pre)-Master-Secret log filename. Anyone with the file can read the traffic it covers. Off again when tracewing starts, unless SSLKEYLOGFILE names a file."
            )))
            .child(
                div()
                    .px(ui(6.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(10.))
                    .child(switch)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(crate::mono_font())
                            .text_size(ui(12.))
                            .text_color(rgb(if on { c.text } else { c.muted }))
                            .child(shown.clone()),
                    ),
            )
            .children(keys.problem().map(|why| {
                div()
                    .px(ui(6.))
                    .text_color(rgb(c.error))
                    .child(tr!("Cannot write to it: {why}", why = why))
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(ui(8.))
                    .child(button("tls-keys-copy", tr!("Copy the path")).on_click(cx.listener(
                        move |_, _, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(shown.clone()));
                        },
                    )))
                    .children(folder.map(|folder| {
                        button("tls-keys-open", tr!("Open the folder")).on_click(cx.listener(
                            move |_, _, _, cx| {
                                let _ = std::fs::create_dir_all(&folder);
                                cx.open_with_system(&folder);
                            },
                        ))
                    })),
            )
    }

    /// The language: the system's, or one of those there are, each named in
    /// itself so that it is found by someone who cannot read the others.
    fn render_language(&self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let chosen = self.layout.language.clone();
        let chip = |id: String, label: String, on: bool| {
            div()
                .id(gpui::ElementId::Name(id.into()))
                .h(ui(30.))
                .px(ui(12.))
                .rounded(ui(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.5))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let system = tracewing_i18n::system_language();
        let system_name = tracewing_i18n::LANGUAGES
            .iter()
            .find(|(tag, _)| *tag == system)
            .map_or("English", |(_, name)| name);
        let languages = tracewing_i18n::LANGUAGES.iter().map(|&(tag, name)| {
            chip(
                format!("language-{tag}"),
                name.to_owned(),
                chosen.as_deref() == Some(tag),
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.set_language(Some(tag), window, cx)),
            )
        });
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Language")).px(ui(6.)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(ui(6.))
                    .child(
                        chip(
                            "language-system".to_owned(),
                            tr!("System ({system_name})", system_name = system_name),
                            chosen.is_none(),
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.set_language(None, window, cx)),
                        ),
                    )
                    .children(languages),
            )
            .child(
                div()
                    .px(ui(6.))
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("Applies at once.")),
            )
    }
}

/// The name over a row of styles.
fn style_row(name: String) -> gpui::Div {
    div()
        .px(ui(6.))
        .pt(ui(4.))
        .text_size(ui(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(name)
}

fn style_chips() -> gpui::Div {
    div().px(ui(6.)).flex().flex_row().flex_wrap().gap(ui(6.))
}

/// A style's chip: its name after a dot of its panels on its page, so that
/// it shows how it looks before it is chosen.
fn style_chip(
    id: gpui::ElementId,
    name: String,
    palette: &crate::colors::Palette,
    on: bool,
) -> gpui::Stateful<gpui::Div> {
    let c = colors();
    div()
        .id(id)
        .h(ui(30.))
        .pl(ui(7.))
        .pr(ui(12.))
        .rounded(ui(15.))
        .flex()
        .items_center()
        .gap(ui(7.))
        .cursor_pointer()
        .reachable()
        .text_size(ui(12.5))
        .font_weight(FontWeight::BOLD)
        .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
        .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
        .child(
            div()
                .size(ui(18.))
                .flex_shrink_0()
                .p(ui(1.))
                .rounded(ui(9.))
                .bg(rgb(palette.soft))
                .child(
                    div()
                        .size_full()
                        .rounded(ui(8.))
                        .border(ui(4.))
                        .border_color(rgb(palette.page))
                        .bg(rgb(palette.surface)),
                ),
        )
        .child(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sentence tracewing could always have said and never did: this way
    /// in is on, and nothing has come through it since you turned it on.
    ///
    /// The `since` is the whole of it. A way in that carried traffic before
    /// the switch was flipped has a count that is not zero, and a tab that
    /// read the count would tell somebody their way in works on the strength
    /// of traffic from before they touched it.
    #[test]
    fn a_way_in_is_quiet_only_when_nothing_came_after_it_was_switched_on() {
        let arrivals = tracewing_proxy::source::Arrivals::default();

        // It carried something, once, before the switch was flipped.
        //
        // Waited past rather than assumed: `anything_since` is `last >=
        // since`, which is the semantics wanted — an arrival at the instant
        // of the switch is since it — so a clock coarse enough to return the
        // same `Instant` twice in adjacent statements makes the old arrival
        // look like a new one. That is a property of the platform's timer,
        // and a virtualized one on a CI runner is where it shows up.
        arrivals.came(By::Socks5);
        // A moment strictly after that arrival, built from the arrival
        // rather than read from the clock: waiting for the clock to move
        // needs a bound, and a bound needs a number nobody can justify.
        let switched_on = arrivals.of(By::Socks5).last.expect("it came") + Duration::from_nanos(1);
        let later = switched_on + QUIET_AFTER;
        assert!(
            gone_quiet(Way::Socks5, Some(switched_on), &arrivals, later),
            "the old arrival is not an answer about this switch"
        );

        // And now it carries something.
        arrivals.came(By::Socks5);
        assert!(!gone_quiet(
            Way::Socks5,
            Some(switched_on),
            &arrivals,
            later
        ));
    }

    /// Nothing is said in the first few seconds, and nothing is said about a
    /// way in that was never switched on.
    #[test]
    fn a_switch_just_flipped_is_left_alone() {
        let arrivals = tracewing_proxy::source::Arrivals::default();
        let on = Instant::now();
        assert!(
            !gone_quiet(Way::Socks5, Some(on), &arrivals, on),
            "nothing has had time to arrive yet"
        );
        assert!(gone_quiet(
            Way::Socks5,
            Some(on),
            &arrivals,
            on + QUIET_AFTER
        ));
        assert!(
            !gone_quiet(Way::Socks5, None, &arrivals, on + QUIET_AFTER),
            "never switched on is not the same as switched on and quiet"
        );
    }

    /// A browser and a terminal are opened, not left on, so neither carries
    /// the sentence; everything else reads a count of its own, and the three
    /// that share the proxy port read the same one.
    #[test]
    fn only_a_way_in_somebody_waits_at_reads_a_count() {
        let counted: Vec<Option<By>> = Way::ALL.into_iter().map(counted_as).collect();
        assert_eq!(
            counted,
            vec![
                Some(By::ProxyPort),
                None,
                None,
                Some(By::OtherDevices),
                Some(By::Socks5),
                Some(By::WireGuard),
                Some(By::Extension),
            ]
        );
        let arrivals = tracewing_proxy::source::Arrivals::default();
        let on = Instant::now();
        for way in [Way::Browser, Way::Terminal] {
            assert!(
                !gone_quiet(way, Some(on), &arrivals, on + QUIET_AFTER),
                "{} is not a switch",
                way.name()
            );
        }
    }

    /// Every tab says what has to trust tracewing, and the two that cannot
    /// do it themselves say where it is done. A blank line here is a tab
    /// that lists what it catches and leaves somebody with certificate
    /// errors and no idea which half is missing.
    #[test]
    fn every_way_in_says_what_has_to_trust_tracewing() {
        for way in Way::ALL {
            let (says, goes_to) = trust_for(way);
            assert!(!says.is_empty(), "{} says nothing", way.name());
            // Where tracewing installs the authority itself there is
            // somewhere to send people; where the way in brings its own
            // trust there is nothing to open, and a button to Certificates
            // would be an invitation to install something nobody needs.
            let installs_here = matches!(way, Way::SystemProxy | Way::Socks5 | Way::Apps);
            assert_eq!(
                goes_to,
                installs_here.then_some(SettingsSection::Certificates),
                "{} points the wrong way",
                way.name()
            );
        }
    }

    /// Of every pair a tab says, only the Apps one turns on a setting, and
    /// only off macOS: "Read chosen programs' HTTP/3" sends a chosen
    /// program's QUIC to tracewing instead of refusing it. Said flatly, the
    /// tab denied in the present tense what the app was doing the moment
    /// that setting went on -- shipped that way in 0.1.0, and nothing a
    /// platform `cfg!` or a window shot would have caught, because both
    /// states compile and both draw.
    #[test]
    fn the_apps_tab_says_whether_http3_is_being_read() {
        let addr = "127.0.0.1:9090";
        for way in Way::ALL {
            let (off, on) = (
                Traffic::catches_for(way, addr, false),
                Traffic::catches_for(way, addr, true),
            );
            // macOS has no HTTP/3 of ours to read: `local_source::catching`
            // is Windows and Linux only, so its tab must not move either.
            if way != Way::Apps || cfg!(target_os = "macos") {
                assert_eq!(
                    off,
                    on,
                    "{} changes with a setting it has nothing to do with",
                    way.name()
                );
                continue;
            }
            assert_ne!(
                off.1, on.1,
                "the Apps tab says the same about UDP either way"
            );
            assert!(
                on.0.contains("HTTP/3"),
                "the Apps tab does not say it is reading HTTP/3 while it is: {}",
                on.0
            );
            assert!(
                !off.0.contains("HTTP/3"),
                "the Apps tab says it reads HTTP/3 while that is off: {}",
                off.0
            );
        }
    }

    /// The whole point of the tabs: a setting belongs to one way in, and is
    /// shown under that one and nowhere else. If this fails, somebody looking
    /// at the SOCKS5 tab is being shown — and "Back to defaults" is reaching
    /// — settings that belong to a different way in.
    #[test]
    fn a_tab_shows_its_own_way_in_and_no_other() {
        for way in Way::ALL {
            for group in shown_in(SettingsSection::Capture, way) {
                assert_eq!(
                    group.way,
                    Some(way),
                    "{} is shown under {}",
                    group.title,
                    way.name()
                );
            }
        }
        // Which ways in have settings of their own; the rest are a switch or
        // an action, and their tabs are text and a button. Said here so that
        // an empty tab is a decision rather than a group somebody forgot to
        // give a `way`.
        let with_rows: Vec<String> = Way::ALL
            .into_iter()
            .filter(|way| !shown_in(SettingsSection::Capture, *way).is_empty())
            .map(Way::name)
            .collect();
        // A browser and a terminal carry the level switch, and only on
        // Windows, where a program's level and the person's can differ. In
        // `Way::ALL`'s order, which is the order the tabs are in.
        let mut want = Vec::new();
        if cfg!(windows) {
            want.extend([tr!("A browser"), tr!("A terminal")]);
        }
        want.extend([
            tr!("Other devices"),
            "SOCKS5".to_owned(),
            "WireGuard".to_owned(),
        ]);
        assert_eq!(with_rows, want);
    }

    /// What is left of `Network` after the ways in moved out of it: the things
    /// that are about traffic which has already arrived.
    #[test]
    fn network_keeps_only_what_is_not_a_way_in() {
        let network: Vec<String> = groups()
            .into_iter()
            .filter(|group| group.at == SettingsSection::Network)
            .map(|group| group.title)
            .collect();
        assert_eq!(network, vec![tr!("HTTP/3")]);
        for group in groups() {
            assert_eq!(
                group.way.is_some(),
                group.at == SettingsSection::Capture,
                "{} has a way in exactly when it is under Capture",
                group.title
            );
        }
    }

    /// Nothing is stored that cannot be changed in Settings -- with two
    /// named exceptions, rather than a pattern, so that a third is a
    /// decision somebody makes here.
    #[test]
    fn every_setting_has_a_row() {
        let table = toml::Value::try_from(StorageSettings::default()).unwrap();
        let mut keys: Vec<&str> = table
            .as_table()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        // The level switches are **stored** on every platform and **offered**
        // only on Windows. Stored everywhere because a settings file travels:
        // the struct denies unknown fields, so dropping them off Windows
        // would make a settings.toml written on Windows unreadable on a Mac.
        // Offered only on Windows because that is the only system where a
        // program's level and the person's can differ.
        if !cfg!(windows) {
            keys.retain(|key| {
                !["browser_as_administrator", "terminal_as_administrator"].contains(key)
            });
        }
        let mut rows: Vec<&str> = groups()
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.key))
            .collect();
        keys.sort_unstable();
        rows.sort_unstable();
        assert_eq!(keys, rows);
    }
}
