//! The main screen: traffic as it happens. The toolbar, the hosts, the list, and
//! the inspector beside it or below it, which can be moved by dragging, from the
//! toolbar, or from the keyboard. Looks at the session a few times a second; all
//! that can take long is done on a worker thread, so a flood or a million rows
//! never holds up a frame.

use crate::{
    chrome,
    colors::{self, colors, with_alpha},
    columns::{Column, Columns, Sort},
    detail::Side,
    devices::{CLIENT_IP, Devices, NOT_LOOPBACK, device_of},
    engine::Engine,
    filter::{Filter, Kind},
    flows::{Flows, Row, group_digits},
    hosts::{HostLine, HostMarks},
    input::{Changed, TextInput},
    inspector::InspectorState,
    layout::{Appearance, INSPECTOR_MIN, LIST_MIN, Layout, Place, SIDEBAR_MAX, SIDEBAR_MIN},
    placement,
    settings_page::SettingsPage,
    shortcuts::{self, Action, PAGE, action_of},
    system::{CaState, ProxySwitch, SystemState},
    widgets::{Reachable, eyebrow, icon, section_badge, status_colors},
};
use crate::{
    compose_page::ComposePage,
    paused_page::PausedPage,
    rules_page::RulesPage,
    sessions_page::{SessionsPage, when_text},
};
use gpui::{
    Anchor, Bounds, ClickEvent, Context, Div, Entity, FocusHandle, Focusable, FontWeight,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render,
    ScrollStrategy, ScrollWheelEvent, SharedString, Stateful, UniformListScrollHandle, Window,
    WindowAppearance, anchored, canvas, deferred, div, point, prelude::*, px, rgb, rgba,
    uniform_list,
};
use rusqlite::{Connection, OpenFlags};
use std::{
    cell::Cell,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};
use tracewing_i18n::{tr, tr_n};
use tracewing_proxy::{capture::CheckVisit, rules::HostRules};
use tracewing_store::{search::Indexer, session};

pub const ROW_HEIGHT: f32 = 32.;
const LOOK_EVERY: Duration = Duration::from_millis(100);
/// How long the longest job may take as a share of the time between runs.
const WORK_SHARE: u32 = 10;
const WORK_EVERY: Duration = Duration::from_secs(1);
const HOSTS_SHOWN: i64 = 500;
const BAR_W: f32 = 10.;
/// How long after the last change of the window's size to write it down: a drag
/// changes it many times a second.
const SAVE_BOUNDS: Duration = Duration::from_millis(400);
const THUMB_MIN: f32 = 28.;

/// What the worker thread does: anything that may read a large part of a session.
enum Job {
    Filter {
        generation: u64,
        filter: Box<Filter>,
    },
    Hosts {
        from_id: i64,
    },
}

enum Done {
    Filter {
        generation: u64,
        ids: Vec<i64>,
        took: Duration,
    },
    Hosts {
        hosts: Vec<(SharedString, u64)>,
        apps: Vec<(SharedString, u64)>,
        devices: Vec<(SharedString, u64)>,
        took: Duration,
    },
}

fn work(session_dir: PathBuf, jobs: Receiver<Job>, done: Sender<Done>) {
    let mut conn = None;
    while let Ok(mut job) = jobs.recv() {
        // Only the latest filter counts, and it is enough to ask for hosts once.
        let mut hosts_too = None;
        while let Ok(newer) = jobs.try_recv() {
            match (&job, newer) {
                (_, Job::Hosts { from_id }) => hosts_too = Some(from_id),
                (_, newer) => job = newer,
            }
        }
        if conn.is_none() {
            conn = Connection::open_with_flags(
                session::flows_db(&session_dir),
                OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .ok();
        }
        let Some(conn) = &conn else {
            continue;
        };
        let (filter_job, hosts_job) = match job {
            Job::Filter { generation, filter } => (Some((generation, filter)), hosts_too),
            Job::Hosts { from_id } => (None, Some(hosts_too.unwrap_or(from_id))),
        };
        if let Some((generation, filter)) = filter_job {
            let started = Instant::now();
            let ids = filter.find(conn, &session_dir).unwrap_or_default();
            let took = started.elapsed();
            let _ = done.send(Done::Filter {
                generation,
                ids,
                took,
            });
        }
        if let Some(from_id) = hosts_job {
            let started = Instant::now();
            let hosts = counts(conn, "host", from_id).unwrap_or_default();
            let apps = counts(conn, "app", from_id).unwrap_or_default();
            let devices = device_counts(conn, from_id).unwrap_or_default();
            let took = started.elapsed();
            let _ = done.send(Done::Hosts {
                hosts,
                apps,
                devices,
                took,
            });
        }
    }
}

/// How many flows each host, or each program, has: `column` is one of the two.
fn counts(
    conn: &Connection,
    column: &str,
    from_id: i64,
) -> rusqlite::Result<Vec<(SharedString, u64)>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {column}, count(*) AS n FROM flows WHERE id >= ?1 AND {column} IS NOT NULL
         GROUP BY {column} ORDER BY n DESC LIMIT ?2"
    ))?;
    let rows = stmt.query_map(rusqlite::params![from_id, HOSTS_SHOWN], |r| {
        Ok((
            SharedString::from(r.get::<_, String>(0)?),
            r.get::<_, i64>(1)? as u64,
        ))
    })?;
    rows.collect()
}

/// How many flows each other device sent: those no program here sent, from an
/// address that is not this machine's loopback.
fn device_counts(conn: &Connection, from_id: i64) -> rusqlite::Result<Vec<(SharedString, u64)>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT ip, count(*) AS n FROM
           (SELECT {CLIENT_IP} AS ip FROM flows WHERE id >= ?1 AND app IS NULL)
         WHERE {NOT_LOOPBACK} GROUP BY ip ORDER BY n DESC LIMIT ?2"
    ))?;
    let rows = stmt.query_map(rusqlite::params![from_id, HOSTS_SHOWN], |r| {
        Ok((
            SharedString::from(r.get::<_, String>(0)?),
            r.get::<_, i64>(1)? as u64,
        ))
    })?;
    rows.collect()
}

/// What the mouse is holding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    /// A column's edge, from where the mouse went down and how wide it was then.
    Column {
        column: Column,
        from_x: f32,
        from_w: f32,
    },
    Sidebar,
    /// The divider beside or above the inspector.
    InspectorSize,
    /// The inspector itself, by its handle, and where it would go now.
    Inspector(Place),
    /// The list's scroll bar.
    ListBar,
}

/// What the rail switches between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Traffic,
    Rules,
    Paused,
    Compose,
    Sessions,
    Settings,
}

fn looks_dark(window: &Window) -> bool {
    matches!(
        window.appearance(),
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

pub struct Traffic {
    pub engine: Engine,
    /// An old session on show, with recording going on behind it.
    pub(crate) viewing: Option<PathBuf>,
    /// Finishing the search index of the one on show.
    catching_up: Option<Indexer>,
    /// When it started, for a session with no name.
    viewing_started: String,
    pub sessions_page: SessionsPage,
    pub rules_page: Option<RulesPage>,
    pub paused_page: Option<PausedPage>,
    pub compose_page: Option<ComposePage>,
    /// The system's own appearance is dark.
    system_dark: bool,
    pub(crate) data_dir: PathBuf,
    pub screen: Screen,
    pub system: SystemState,
    pub settings_page: Option<SettingsPage>,
    /// Why the settings file was not used, if it was not.
    pub settings_problem: Option<String>,
    /// Why a setting was not written when Settings was left.
    pub settings_left: Option<String>,
    layout_path: PathBuf,
    pub(crate) layout: Layout,
    flows: Option<Flows>,
    jobs: Sender<Job>,
    done: Receiver<Done>,
    filter_input: Entity<TextInput>,
    /// Narrows the hosts in the sidebar to those with this in their name.
    host_filter: Entity<TextInput>,
    filter: Filter,
    /// With a filter on, the flows that fit. Without one, every id in `bounds`.
    ids: Option<Arc<Vec<i64>>>,
    generation: u64,
    filter_due: Instant,
    filter_seen: i64,
    filtering: bool,
    bounds: Option<(i64, i64)>,
    hosts: Vec<(SharedString, u64)>,
    /// The programs that sent flows, with how many each.
    apps: Vec<(SharedString, u64)>,
    /// Other devices that sent flows, by address, with how many each.
    device_counts: Vec<(SharedString, u64)>,
    /// The names given to devices.
    devices: Devices,
    /// The name of the device the chosen flow came from.
    device_name: Entity<TextInput>,
    hosts_due: Instant,
    hosts_seen: i64,
    hosts_asked: bool,
    /// Clear hides what came before this; the disk gives it back as usual.
    cleared_to: i64,
    scroll: UniformListScrollHandle,
    /// Stay on the newest flow as more arrive.
    follow: bool,
    look_for_bottom: bool,
    pub(crate) selected: Option<i64>,
    pub inspector: InspectorState,
    drag: Option<Drag>,
    content: Rc<Cell<Bounds<Pixels>>>,
    list: Rc<Cell<Bounds<Pixels>>>,
    pub(crate) focus: FocusHandle,
    last_count: usize,
    rows_missing: bool,
    places_asked: usize,
    bounds_change: u64,
    /// Who last reached the test page, as the window has seen it.
    pub check_visit: Option<CheckVisit>,
    /// Each browser's latest visit to the test page, oldest first.
    pub check_visits: Vec<CheckVisit>,
    /// The menu of columns to show is open.
    columns_menu: bool,
    /// The list of keyboard shortcuts is open over the window.
    shortcuts_open: bool,
    /// The slow network's fields, once the Rules screen has been shown.
    pub(crate) network_form: Option<crate::network::NetworkForm>,
}

impl Traffic {
    pub fn new(
        engine: Engine,
        data_dir: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let session_dir = engine.session.clone();
        std::thread::Builder::new()
            .name("traffic-worker".into())
            .spawn(move || work(session_dir, job_rx, done_tx))
            .expect("a thread can start");

        let filter_input =
            cx.new(|cx| TextInput::said(|| tr!("host:, status:4xx, or text in bodies"), cx));
        let device_name = cx.new(|cx| TextInput::said(|| tr!("A name, as iPhone test"), cx));
        let host_filter = cx.new(|cx| TextInput::said(|| tr!("Filter"), cx));
        cx.subscribe(&host_filter, |_, _, _: &Changed, cx| cx.notify())
            .detach();
        cx.subscribe(&device_name, |this, input, _: &Changed, cx| {
            let name = input.read(cx).text().to_owned();
            this.name_chosen_device(&name, cx);
        })
        .detach();
        cx.subscribe(&filter_input, |this, input, _: &Changed, cx| {
            let text = input.read(cx).text().to_owned();
            this.set_filter_text(&text, cx);
        })
        .detach();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(LOOK_EVERY).await;
                if this.update(cx, |this, cx| this.look(cx)).is_err() {
                    return;
                }
            }
        })
        .detach();
        cx.observe_window_bounds(window, Self::bounds_changed)
            .detach();
        cx.defer_in(window, Self::bounds_changed);
        cx.on_app_quit(|this, cx| {
            // What was typed last in Settings, if the field was never left.
            this.save_settings(cx);
            this.save_layout();
            if let Some(indexer) = this.catching_up.take() {
                let _ = indexer.leave();
            }
            this.release_system_proxy();
            this.engine.stop();
            async {}
        })
        .detach();

        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let layout_path = data_dir.join("layout.toml");
        let now = Instant::now();
        cx.defer_in(window, |this, _, cx| {
            this.start_ca_check(cx);
            this.recover_android(cx);
        });
        // The desktop may move the window's buttons to the other end.
        cx.observe_button_layout_changed(window, |_, _, cx| cx.notify())
            .detach();
        cx.observe_window_appearance(window, |this, window, cx| {
            this.system_dark = looks_dark(window);
            this.apply_appearance(window, cx);
        })
        .detach();
        let system_dark = looks_dark(window);
        let layout = Layout::load(&layout_path);
        let filter = Filter {
            hidden_hosts: layout.hosts.hidden.clone(),
            ..Filter::default()
        };
        colors::set_dark(match layout.appearance {
            Appearance::Light => false,
            Appearance::Dark => true,
            Appearance::System => system_dark,
        });
        Self {
            engine,
            viewing: None,
            catching_up: None,
            viewing_started: String::new(),
            sessions_page: SessionsPage::default(),
            rules_page: None,
            paused_page: None,
            compose_page: None,
            system_dark,
            data_dir: data_dir.to_owned(),
            screen: Screen::Traffic,
            system: SystemState::default(),
            settings_page: None,
            settings_problem: None,
            settings_left: None,
            layout,
            layout_path,
            flows: None,
            jobs: job_tx,
            done: done_rx,
            filter_input,
            host_filter,
            filter,
            columns_menu: false,
            shortcuts_open: false,
            network_form: None,
            ids: None,
            generation: 0,
            filter_due: now,
            filter_seen: 0,
            filtering: false,
            bounds: None,
            hosts: Vec::new(),
            apps: Vec::new(),
            device_counts: Vec::new(),
            devices: Devices::load(&data_dir.join("devices.toml")),
            device_name,
            hosts_due: now,
            hosts_seen: 0,
            hosts_asked: false,
            cleared_to: 0,
            scroll: UniformListScrollHandle::new(),
            follow: true,
            look_for_bottom: false,
            selected: None,
            inspector: InspectorState::default(),
            drag: None,
            content: Rc::new(Cell::new(Bounds::default())),
            list: Rc::new(Cell::new(Bounds::default())),
            focus,
            last_count: 0,
            rows_missing: false,
            places_asked: 0,
            bounds_change: 0,
            check_visit: None,
            check_visits: Vec::new(),
        }
    }

    fn apply_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        colors::set_dark(match self.layout.appearance {
            Appearance::Light => false,
            Appearance::Dark => true,
            Appearance::System => self.system_dark,
        });
        window.refresh();
        cx.notify();
    }

    pub fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.layout.appearance = appearance;
        self.save_layout();
        self.apply_appearance(window, cx);
    }

    /// What opens first: Set up, until it has been seen once.
    pub fn open_first_screen(&mut self) {
        if !self.layout.setup_seen {
            self.screen = Screen::Setup;
        }
    }

    fn rules_path(&self) -> PathBuf {
        self.data_dir.join("decrypt.toml")
    }

    /// Changes what is not decrypted, from the next connection on, and keeps it.
    pub fn change_rules(&mut self, change: impl FnOnce(&mut HostRules), cx: &mut Context<Self>) {
        let rules = {
            let mut rules = self.engine.rules.write().unwrap_or_else(|e| e.into_inner());
            change(&mut rules);
            rules.clone()
        };
        if let Err(e) = rules.save(&self.rules_path()) {
            self.system.problem = Some(format!("{e:#}"));
        }
        cx.notify();
    }

    pub fn settings_path(&self) -> PathBuf {
        self.data_dir.join("settings.toml")
    }

    pub fn set_screen(&mut self, screen: Screen, cx: &mut Context<Self>) {
        if self.screen == Screen::Settings && screen != Screen::Settings {
            self.leave_settings(cx);
        }
        if screen == Screen::Settings {
            self.settings_left = None;
        }
        // Once left, it does not open by itself again.
        if self.screen == Screen::Setup && !self.layout.setup_seen {
            self.layout.setup_seen = true;
            self.save_layout();
        }
        self.screen = screen;
        if screen == Screen::Sessions {
            self.refresh_sessions(cx);
        }
        // A browser may have come, or made a profile, since the last look.
        if screen == Screen::Setup {
            self.start_ca_check(cx);
        }
        cx.notify();
    }

    pub fn session_dir(&self) -> &Path {
        self.viewing.as_deref().unwrap_or(&self.engine.session)
    }

    pub(crate) fn forget_row(&mut self, id: i64) {
        if let Some(flows) = &mut self.flows {
            flows.forget(id);
        }
    }

    pub fn is_viewing(&self) -> bool {
        self.viewing.is_some()
    }

    pub fn is_catching_up(&self) -> bool {
        self.catching_up.is_some()
    }

    fn start_worker(&mut self, session_dir: PathBuf) {
        let (job_tx, job_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("traffic-worker".into())
            .spawn(move || work(session_dir, job_rx, done_tx))
            .expect("a thread can start");
        // Without the sender the old worker ends by itself.
        self.jobs = job_tx;
        self.done = done_rx;
    }

    /// Whatever session is on show, from its start.
    fn show_session(&mut self, cx: &mut Context<Self>) {
        self.leave_catching_up(cx);
        self.flows = None;
        self.bounds = None;
        self.hosts.clear();
        self.hosts_seen = 0;
        self.hosts_asked = false;
        self.hosts_due = Instant::now();
        self.cleared_to = 0;
        self.selected = None;
        self.inspector = InspectorState::default();
        self.start_worker(self.session_dir().to_owned());
        self.look_for_bottom = false;
        self.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(0.)));
        self.refilter(cx);
    }

    /// Shows an old session, and finishes its search index if a crash or an early
    /// close left it short.
    pub fn open_session(&mut self, dir: &Path, cx: &mut Context<Self>) {
        if dir == self.engine.session {
            self.back_to_live(cx);
            return;
        }
        self.viewing = Some(dir.to_owned());
        self.viewing_started = when_text(session::info(dir).first_us);
        self.show_session(cx);
        match Indexer::catch_up(dir, &self.engine.settings) {
            Ok(indexer) => self.catching_up = Some(indexer),
            Err(e) => {
                self.system.problem = Some(tr!("The search index: {e}", e = format!("{e:#}")))
            }
        }
    }

    pub fn back_to_live(&mut self, cx: &mut Context<Self>) {
        self.viewing = None;
        self.show_session(cx);
    }

    fn leave_catching_up(&mut self, cx: &mut Context<Self>) {
        if let Some(indexer) = self.catching_up.take() {
            cx.background_spawn(async move {
                let _ = indexer.leave();
            })
            .detach();
        }
    }

    fn render_viewing_banner(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let dir = self.viewing.as_ref()?;
        let c = colors();
        let name = session::name_of(dir).unwrap_or_else(|| self.viewing_started.clone());
        Some(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(10.))
                .px(px(14.))
                .py(px(8.))
                .rounded(px(16.))
                .bg(rgb(c.tonal))
                .text_color(rgb(c.on_tonal))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .child(tr!("An earlier session: {name}", name = name)),
                )
                .child(div().child(if self.catching_up.is_some() {
                    tr!(
                        "Read-only. Search covers more as its index is finished. Recording goes on."
                    )
                } else {
                    tr!("Read-only. Recording goes on.")
                }))
                .child(div().flex_1())
                .child(
                    div()
                        .id("back-to-live")
                        .h(px(30.))
                        .px(px(14.))
                        .rounded(px(15.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .reachable()
                        .bg(rgb(c.accent))
                        .text_color(rgb(c.on_accent))
                        .font_weight(FontWeight::BOLD)
                        .child(tr!("Back to live"))
                        .on_click(cx.listener(|this, _, _, cx| this.back_to_live(cx))),
                ),
        )
    }

    pub fn proxy_addr(&self) -> std::net::SocketAddr {
        self.engine.addr
    }

    pub(crate) fn flows(&self) -> Option<&Flows> {
        self.flows.as_ref()
    }

    /// The first flow in view: the oldest left, or the first after a clear.
    fn first_id(&self) -> Option<i64> {
        let (first, last) = self.bounds?;
        let first = first.max(self.cleared_to);
        (first <= last).then_some(first)
    }

    pub fn count(&self) -> usize {
        if let Some(ids) = &self.ids {
            return ids.len();
        }
        match (self.first_id(), self.bounds) {
            (Some(first), Some((_, last))) => (last - first + 1) as usize,
            _ => 0,
        }
    }

    /// Every flow in view, a filter or not.
    fn total(&self) -> usize {
        match (self.first_id(), self.bounds) {
            (Some(first), Some((_, last))) => (last - first + 1) as usize,
            _ => 0,
        }
    }

    fn ids_at(&self, range: Range<usize>) -> Vec<i64> {
        if let Some(ids) = &self.ids {
            return ids[range.start.min(ids.len())..range.end.min(ids.len())].to_vec();
        }
        let Some(first) = self.first_id() else {
            return Vec::new();
        };
        range.map(|ix| first + ix as i64).collect()
    }

    fn place_of(&self, id: i64) -> Option<usize> {
        match &self.ids {
            // Sorted by a column, they are not in the order of their ids.
            Some(ids) if self.filter.sort.is_some() => ids.iter().position(|i| *i == id),
            Some(ids) => ids.binary_search(&id).ok(),
            None => {
                let first = self.first_id()?;
                let (_, last) = self.bounds?;
                (first..=last).contains(&id).then(|| (id - first) as usize)
            }
        }
    }

    /// One look at the session: what has arrived, what the worker has done.
    pub fn look(&mut self, cx: &mut Context<Self>) {
        if self.flows.is_none() {
            self.flows = Flows::open(self.session_dir(), self.viewing.is_none()).ok();
        }
        let mut changed = false;
        if let Some(flows) = &self.flows {
            let bounds = flows.bounds().ok().flatten();
            changed |= bounds != self.bounds;
            self.bounds = bounds;
        }
        while let Ok(done) = self.done.try_recv() {
            changed = true;
            match done {
                Done::Filter {
                    generation,
                    ids,
                    took,
                } => {
                    if generation == self.generation {
                        self.ids = (!self.filter.is_empty()).then(|| Arc::new(ids));
                        self.filtering = false;
                        self.filter_due = Instant::now() + WORK_EVERY.max(took * WORK_SHARE);
                    }
                }
                Done::Hosts {
                    hosts,
                    apps,
                    devices,
                    took,
                } => {
                    self.hosts = hosts;
                    self.apps = apps;
                    self.device_counts = devices;
                    self.hosts_asked = false;
                    self.hosts_due = Instant::now() + WORK_EVERY.max(took * WORK_SHARE);
                }
            }
        }
        if self.catching_up.as_ref().is_some_and(Indexer::is_finished) {
            self.leave_catching_up(cx);
            // What it found is now there to find.
            self.filter_seen = -1;
            changed = true;
        }
        changed |= self.look_at_stream(cx);
        changed |= self.look_at_pauses(cx);
        let visits = self.engine.check.visits();
        if visits != self.check_visits {
            self.check_visit = visits.last().cloned();
            self.check_visits = visits;
            changed = true;
        }
        let newest = self.bounds.map_or(0, |(_, last)| last);
        let now = Instant::now();
        if !self.hosts_asked && newest != self.hosts_seen && now >= self.hosts_due {
            self.hosts_seen = newest;
            self.hosts_asked = true;
            let _ = self.jobs.send(Job::Hosts {
                from_id: self.cleared_to,
            });
        }
        if !self.filter.is_empty()
            && !self.filtering
            && newest != self.filter_seen
            && now >= self.filter_due
        {
            self.ask_filter();
        }
        self.keep_up_with_newest();
        // A row that could not be read gets another try.
        if changed || self.drag.is_some() || self.rows_missing {
            cx.notify();
        }
    }

    fn keep_up_with_newest(&mut self) {
        let count = self.count();
        let height = f32::from(self.list.get().size.height);
        let max = (count as f32 * ROW_HEIGHT - height).max(0.);
        if std::mem::take(&mut self.look_for_bottom) {
            let offset = -f32::from(self.scroll.0.borrow().base_handle.offset().y);
            self.follow = offset >= max - ROW_HEIGHT;
        }
        if self.follow && count != self.last_count {
            self.scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(-max)));
        }
        self.last_count = count;
    }

    fn ask_filter(&mut self) {
        self.filter_seen = self.bounds.map_or(0, |(_, last)| last);
        self.filtering = true;
        let mut filter = self.filter.clone();
        filter.from_id = self.cleared_to;
        let _ = self.jobs.send(Job::Filter {
            generation: self.generation,
            filter: Box::new(filter),
        });
    }

    /// The filter changed: the old answer no longer counts.
    fn refilter(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        if self.filter.is_empty() {
            self.ids = None;
            self.filtering = false;
        } else {
            self.ask_filter();
        }
        // Live, the newest is what matters; nothing new comes to an earlier session,
        // so its answer starts at the top. Sorted, the newest can be anywhere, and
        // the top is where the order starts.
        let sorted = self.filter.sort.is_some();
        self.follow = self.viewing.is_none() && !sorted;
        self.last_count = usize::MAX;
        if self.viewing.is_some() || sorted {
            self.scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(0.)));
        }
        cx.notify();
    }

    pub fn set_filter_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let mut filter = Filter::parse_with(text, &self.devices);
        filter.kind = self.filter.kind;
        filter.exact_host = self.filter.exact_host.clone();
        filter.exact_app = self.filter.exact_app.clone();
        filter.exact_device = self.filter.exact_device.clone();
        filter.sort = self.filter.sort;
        filter.hidden_hosts = self.filter.hidden_hosts.clone();
        if filter != self.filter {
            self.filter = filter;
            self.refilter(cx);
        }
    }

    pub fn set_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        self.filter.kind = kind;
        self.refilter(cx);
    }

    pub fn set_host(&mut self, host: Option<String>, cx: &mut Context<Self>) {
        self.filter.exact_host = host;
        self.refilter(cx);
    }

    /// The programs in the sidebar, with how many flows each sent.
    pub fn app_counts(&self) -> &[(SharedString, u64)] {
        &self.apps
    }

    pub fn set_app(&mut self, app: Option<String>, cx: &mut Context<Self>) {
        self.filter.exact_app = app;
        self.refilter(cx);
    }

    /// Every host with flows, busiest first, with how many each has.
    pub fn host_counts(&self) -> &[(SharedString, u64)] {
        &self.hosts
    }

    /// The hosts pinned and hidden.
    pub fn host_marks(&self) -> &HostMarks {
        &self.layout.hosts
    }

    /// Pins a host to the top of the sidebar, or unpins it.
    pub fn toggle_pin(&mut self, host: &str, cx: &mut Context<Self>) {
        let was_hidden = self.layout.hosts.is_hidden(host);
        self.layout.hosts.toggle_pin(host);
        self.save_layout();
        if was_hidden {
            self.hidden_hosts_changed(cx);
        }
        cx.notify();
    }

    /// Leaves a host's flows out of the list, or brings them back.
    pub fn toggle_hidden(&mut self, host: &str, cx: &mut Context<Self>) {
        self.layout.hosts.toggle_hidden(host);
        self.save_layout();
        // What is hidden cannot be the one chosen.
        if self.layout.hosts.is_hidden(host) && self.filter.exact_host.as_deref() == Some(host) {
            self.filter.exact_host = None;
        }
        self.hidden_hosts_changed(cx);
    }

    fn hidden_hosts_changed(&mut self, cx: &mut Context<Self>) {
        self.filter.hidden_hosts = self.layout.hosts.hidden.clone();
        self.refilter(cx);
    }

    /// What the list is sorted by, if not by when flows came.
    pub fn sort(&self) -> Option<Sort> {
        self.filter.sort
    }

    pub fn set_sort(&mut self, sort: Option<Sort>, cx: &mut Context<Self>) {
        self.filter.sort = sort;
        self.refilter(cx);
    }

    /// A click on a column's header.
    pub fn sort_by(&mut self, column: Column, cx: &mut Context<Self>) {
        self.set_sort(Sort::after_click(self.filter.sort, column), cx);
    }

    pub fn columns(&self) -> &Columns {
        &self.layout.columns
    }

    pub fn toggle_column(&mut self, column: Column, cx: &mut Context<Self>) {
        self.layout.columns.toggle(column);
        // A sort by a column no longer there would be hard to see, or undo.
        if self.filter.sort.is_some_and(|sort| sort.column == column)
            && !self.layout.columns.is_shown(column)
        {
            self.set_sort(None, cx);
        }
        self.save_layout();
        cx.notify();
    }

    pub fn set_column_width(&mut self, column: Column, width: f32, cx: &mut Context<Self>) {
        self.layout.columns.set_width(column, width);
        self.save_layout();
        cx.notify();
    }

    pub fn reset_columns(&mut self, cx: &mut Context<Self>) {
        self.layout.columns = Columns::default();
        self.save_layout();
        cx.notify();
    }

    pub fn set_columns_menu(&mut self, open: bool, cx: &mut Context<Self>) {
        self.columns_menu = open;
        cx.notify();
    }

    /// Other devices in the sidebar, by address, with how many flows each sent.
    pub fn device_counts(&self) -> &[(SharedString, u64)] {
        &self.device_counts
    }

    pub fn set_device(&mut self, ip: Option<String>, cx: &mut Context<Self>) {
        self.filter.exact_device = ip;
        self.refilter(cx);
    }

    pub fn devices(&self) -> &Devices {
        &self.devices
    }

    /// The box for the name of the device the chosen flow came from.
    pub fn device_name_input(&self) -> &Entity<TextInput> {
        &self.device_name
    }

    /// Names the device at `ip`, or with an empty name forgets it, and keeps that.
    pub fn name_device(&mut self, ip: &str, name: &str, cx: &mut Context<Self>) {
        if self.devices.name(ip).unwrap_or("") == name.trim() {
            return;
        }
        if let Err(e) = self.devices.set(ip, name) {
            self.system.problem = Some(format!("{e:#}"));
        }
        // Rows show names as they are drawn; `device:` finds them by name.
        if self.filter.device.is_some() {
            self.filter.name_devices(&self.devices);
            self.refilter(cx);
        }
        cx.notify();
    }

    fn name_chosen_device(&mut self, name: &str, cx: &mut Context<Self>) {
        let device = self
            .inspector
            .detail
            .as_ref()
            .filter(|detail| detail.app.is_none())
            .and_then(|detail| device_of(&detail.client));
        if let Some(ip) = device {
            self.name_device(&ip, name, cx);
        }
    }

    /// Puts the chosen flow's device's name in the box for it.
    pub(crate) fn show_device_name(&mut self, cx: &mut Context<Self>) {
        let device = self
            .inspector
            .detail
            .as_ref()
            .filter(|detail| detail.app.is_none())
            .and_then(|detail| device_of(&detail.client));
        let name = device
            .as_deref()
            .and_then(|ip| self.devices.name(ip))
            .unwrap_or("")
            .to_owned();
        self.device_name.update(cx, |input, cx| {
            if input.text() != name {
                input.set_text(&name, name.len()..name.len(), cx);
            }
        });
    }

    /// How many rows the list last asked for: a screenful, whatever happens.
    pub fn places_asked(&self) -> usize {
        self.places_asked
    }

    /// As if the session could not be read for a moment. For tests.
    #[doc(hidden)]
    pub fn lose_the_session(&mut self, cx: &mut Context<Self>) {
        self.flows = None;
        cx.notify();
    }

    /// The place of the row at the top of the list.
    pub fn first_row_in_view(&self) -> usize {
        let offset = -f32::from(self.scroll.0.borrow().base_handle.offset().y);
        (offset / ROW_HEIGHT) as usize
    }

    pub fn is_filtering(&self) -> bool {
        self.filtering
    }

    /// The ids of the flows the list shows, oldest first.
    pub(crate) fn shown_ids(&self) -> Vec<i64> {
        match &self.ids {
            Some(ids) => ids.to_vec(),
            None => self.bounds.map_or_else(Vec::new, |(first, last)| {
                (first.max(self.cleared_to)..=last).collect()
            }),
        }
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.cleared_to = self.bounds.map_or(0, |(_, last)| last + 1);
        self.hosts.clear();
        self.apps.clear();
        self.device_counts.clear();
        self.hosts_seen = 0;
        self.selected = None;
        self.inspector = InspectorState::default();
        self.refilter(cx);
    }

    pub fn select_place(&mut self, place: usize, cx: &mut Context<Self>) {
        if let Some(&id) = self.ids_at(place..place + 1).first() {
            self.select(id, cx);
        }
    }

    fn select(&mut self, id: i64, cx: &mut Context<Self>) {
        self.selected = Some(id);
        self.follow = false;
        self.load_detail(id, cx);
        cx.notify();
    }

    fn move_selection(&mut self, by: i64, cx: &mut Context<Self>) {
        let count = self.count() as i64;
        if count == 0 {
            return;
        }
        let place = self
            .selected
            .and_then(|id| self.place_of(id))
            .map_or(if by > 0 { 0 } else { count - 1 }, |p| {
                (p as i64 + by).clamp(0, count - 1)
            }) as usize;
        self.select_place(place, cx);
        self.scroll.scroll_to_item(place, ScrollStrategy::Nearest);
    }

    fn jump_to_newest(&mut self, cx: &mut Context<Self>) {
        self.follow = true;
        self.last_count = usize::MAX;
        self.keep_up_with_newest();
        cx.notify();
    }

    pub fn set_place(&mut self, place: Place, cx: &mut Context<Self>) {
        self.layout.place = place;
        self.layout.inspector_shown = true;
        self.save_layout();
        cx.notify();
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.layout.sidebar_shown = !self.layout.sidebar_shown;
        self.save_layout();
        cx.notify();
    }

    fn toggle_inspector(&mut self, cx: &mut Context<Self>) {
        self.layout.inspector_shown = !self.layout.inspector_shown;
        self.save_layout();
        cx.notify();
    }

    /// Remembers the window's size and place for the next start: saved shortly after
    /// they change, and when the app quits.
    fn bounds_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.layout.window = Some(placement::state(window.window_bounds()));
        self.bounds_change += 1;
        let change = self.bounds_change;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_BOUNDS).await;
            let _ = this.update(cx, |this, _| {
                if this.bounds_change == change {
                    this.save_layout();
                }
            });
        })
        .detach();
    }

    pub(crate) fn save_layout(&self) {
        self.layout.save(&self.layout_path);
    }

    /// `None` for the system's. Words change as views are drawn again, the
    /// list's rows as they are read again.
    pub fn set_language(&mut self, tag: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        tracewing_i18n::set_language(tag);
        if let Some(flows) = &mut self.flows {
            flows.forget_all();
        }
        self.layout.language = tag.map(str::to_owned);
        self.save_layout();
        window.refresh();
        cx.notify();
    }

    /// Where the inspector goes if let go here: in the right third it stays beside
    /// the list, in the lower half of the rest it goes below.
    fn place_under(&self, position: Point<Pixels>) -> Place {
        let content = self.content.get();
        let x = f32::from(position.x - content.origin.x);
        let y = f32::from(position.y - content.origin.y);
        let width = f32::from(content.size.width);
        let height = f32::from(content.size.height);
        if x > width * 2. / 3. || y < height / 2. {
            Place::Beside
        } else {
            Place::Below
        }
    }

    pub fn cancel_drag(&mut self, cx: &mut Context<Self>) {
        self.drag = None;
        cx.notify();
    }

    pub fn begin_inspector_drag(&mut self, cx: &mut Context<Self>) {
        self.drag = Some(Drag::Inspector(self.layout.place));
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag else {
            return;
        };
        if !event.dragging() {
            self.drag = None;
            cx.notify();
            return;
        }
        let content = self.content.get();
        match drag {
            Drag::Sidebar => {
                self.layout.sidebar_w =
                    f32::from(event.position.x - content.origin.x).clamp(SIDEBAR_MIN, SIDEBAR_MAX);
            }
            Drag::InspectorSize => match self.layout.place {
                Place::Beside => {
                    let room = f32::from(content.size.width) - LIST_MIN;
                    self.layout.inspector_w =
                        f32::from(content.origin.x + content.size.width - event.position.x)
                            .clamp(INSPECTOR_MIN, room.max(INSPECTOR_MIN));
                }
                Place::Below => {
                    let room = f32::from(content.size.height) - LIST_MIN;
                    self.layout.inspector_h =
                        f32::from(content.origin.y + content.size.height - event.position.y)
                            .clamp(INSPECTOR_MIN / 2., room.max(INSPECTOR_MIN / 2.));
                }
            },
            Drag::Inspector(_) => {
                self.drag = Some(Drag::Inspector(self.place_under(event.position)));
            }
            Drag::ListBar => self.drag_list_bar(event.position.y),
            Drag::Column {
                column,
                from_x,
                from_w,
            } => {
                let moved = f32::from(event.position.x) - from_x;
                let width = if column.on_the_right() {
                    from_w - moved
                } else {
                    from_w + moved
                };
                self.layout.columns.set_width(column, width);
            }
        }
        cx.notify();
    }

    fn list_max_scroll(&self) -> f32 {
        (self.count() as f32 * ROW_HEIGHT - f32::from(self.list.get().size.height)).max(0.)
    }

    fn thumb_height(&self) -> f32 {
        let height = f32::from(self.list.get().size.height);
        let content = self.count() as f32 * ROW_HEIGHT;
        if content <= height {
            height
        } else {
            (height * height / content).max(THUMB_MIN)
        }
    }

    /// Scrolls the list so its top edge is `y` pixels into the content.
    pub fn set_scroll_y(&mut self, y: f32) {
        self.follow = false;
        self.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(-y.clamp(0., self.list_max_scroll()))));
    }

    pub fn max_scroll(&self) -> f32 {
        self.list_max_scroll()
    }

    fn drag_list_bar(&mut self, y: Pixels) {
        let list = self.list.get();
        let thumb = self.thumb_height();
        let track = (f32::from(list.size.height) - thumb).max(1.);
        let at = (f32::from(y - list.origin.y) - thumb / 2.) / track;
        self.set_scroll_y(at.clamp(0., 1.) * self.list_max_scroll());
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        match self.drag.take() {
            Some(Drag::Inspector(place)) => self.set_place(place, cx),
            Some(_) => {
                self.save_layout();
                cx.notify();
            }
            None => {}
        }
    }

    pub fn toggle_recording(&mut self, cx: &mut Context<Self>) {
        let on = !self.engine.is_recording();
        self.engine.set_recording(on);
        cx.notify();
    }

    pub fn shortcuts_open(&self) -> bool {
        self.shortcuts_open
    }

    pub fn set_shortcuts(&mut self, open: bool, cx: &mut Context<Self>) {
        self.shortcuts_open = open;
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        // A text field has the keyboard when the window itself does not.
        let typing = !self.focus.is_focused(window);
        let Some(action) = action_of(
            &keys.key,
            keys.modifiers.secondary(),
            keys.modifiers.shift,
            typing,
        ) else {
            return;
        };
        if self.do_key(action, window, cx) {
            cx.stop_propagation();
        }
    }

    /// Does what a key says. False if it says nothing here, which leaves the
    /// key to whoever else wants it.
    pub fn do_key(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let in_list = self.screen == Screen::Traffic;
        let chosen = self.inspector.detail.is_some();
        match action {
            Action::Close if self.drag.is_some() => {
                self.drag = None;
                cx.notify();
            }
            Action::Close if self.shortcuts_open => self.set_shortcuts(false, cx),
            Action::Close if self.close_copy_menu(cx) => {}
            Action::Close => return false,
            Action::Shortcuts => self.set_shortcuts(!self.shortcuts_open, cx),
            Action::Filter => {
                self.set_screen(Screen::Traffic, cx);
                let handle = self.filter_input.read(cx).focus_handle(cx);
                window.focus(&handle, cx);
            }
            Action::Place => {
                let place = match self.layout.place {
                    Place::Beside => Place::Below,
                    Place::Below => Place::Beside,
                };
                self.set_place(place, cx);
            }
            Action::Sidebar => self.toggle_sidebar(cx),
            Action::Inspector => self.toggle_inspector(cx),
            Action::Up if in_list => self.move_selection(-1, cx),
            Action::Down if in_list => self.move_selection(1, cx),
            Action::PageUp if in_list => self.move_selection(-PAGE, cx),
            Action::PageDown if in_list => self.move_selection(PAGE, cx),
            Action::First if in_list => self.move_selection(i64::MIN / 2, cx),
            Action::Last if in_list => self.move_selection(i64::MAX / 2, cx),
            Action::Request if chosen => self.set_side(Side::Request, cx),
            Action::Response if chosen => self.set_side(Side::Response, cx),
            Action::SendAgain if self.screen == Screen::Paused => self.send_paused(cx),
            Action::SendUnchanged if self.screen == Screen::Paused => {
                self.send_paused_unchanged(cx)
            }
            Action::Drop if self.screen == Screen::Paused => self.drop_paused(cx),
            Action::SendAgain if chosen => self.send_flow_again(cx),
            Action::PreviousPart if chosen && in_list => self.step_part(-1, cx),
            Action::NextPart if chosen && in_list => self.step_part(1, cx),
            Action::SaveBody if chosen && in_list => self.ask_save_body(cx),
            Action::EditAndSend if chosen => self.edit_flow(cx),
            Action::CopyCurl if chosen => self.copy_as_curl(cx),
            Action::Copy if self.screen == Screen::Compose => return self.copy_composed(cx),
            Action::SelectAll if self.screen == Screen::Compose => {
                return self.select_all_composed(cx);
            }
            Action::Copy if chosen => return self.copy_selection(cx),
            Action::SelectAll if chosen && self.screen == Screen::Traffic => {
                return self.select_all(cx);
            }
            Action::Record => self.toggle_recording(cx),
            Action::Clear if in_list => self.clear(cx),
            Action::Go(screen) => self.set_screen(screen, cx),
            _ => return false,
        }
        true
    }

    /// The columns shown, with how wide each is.
    fn shown_columns(&self) -> Vec<(Column, f32)> {
        let columns = &self.layout.columns;
        columns
            .shown()
            .into_iter()
            .map(|column| (column, columns.width(column)))
            .collect()
    }

    /// The program that sent a row, or the device it came from by its name.
    fn sender(&self, row: &Row) -> SharedString {
        match (&row.app, &row.device) {
            (Some(app), _) => app.clone(),
            (None, Some(ip)) => match self.devices.name(ip) {
                Some(name) => name.to_owned().into(),
                None => ip.clone(),
            },
            (None, None) => SharedString::default(),
        }
    }

    /// Exactly one element per place asked for, always. `uniform_list` measures the
    /// first to learn the height of them all: given none it takes the height for
    /// zero, thinks every row is in view and asks for all of them, which at a million
    /// rows ran the machine out of memory.
    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<Stateful<Div>> {
        self.places_asked = range.len();
        let ids = self.ids_at(range.clone());
        let selected = self.selected;
        let columns = self.shown_columns();
        let mut rows = self
            .flows
            .as_mut()
            .map_or_else(Vec::new, |flows| flows.rows(&ids));
        rows.resize(range.len(), None);
        // A row missing gets another try, and one still going will change.
        let mut gaps = false;
        let elements = range
            .zip(rows)
            .map(|(place, row)| match row {
                Some(row) => {
                    gaps |= row.going;
                    let id = row.id;
                    let sender = self.sender(&row);
                    render_row(row, sender, &columns, selected == Some(id)).on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.select(id, cx);
                        },
                    ))
                }
                None => {
                    gaps = true;
                    div().id(("gap", place as u64)).h(px(ROW_HEIGHT)).w_full()
                }
            })
            .collect();
        self.rows_missing = gaps;
        elements
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let recording = self.engine.is_recording();
        let filtered = self.ids.is_some() || self.filtering;
        let count = if filtered {
            tr!(
                "{shown} of {total}",
                shown = group_digits(self.count() as u64),
                total = group_digits(self.total() as u64)
            )
        } else {
            String::new()
        };
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(
                pill_button("record")
                    .gap(px(8.))
                    .pl(px(12.))
                    .pr(px(16.))
                    .bg(rgb(c.surface))
                    .text_color(rgb(c.text))
                    .font_weight(FontWeight::BOLD)
                    .child(if recording {
                        icon("record", 14., c.record)
                    } else {
                        icon("pause", 14., c.muted)
                    })
                    .child(if recording {
                        tr!("Recording")
                    } else {
                        tr!("Paused")
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_recording(cx))),
            )
            .children(self.render_network_chip(cx))
            .child(
                pill_button("clear")
                    .w(px(40.))
                    .justify_center()
                    .bg(rgb(c.surface))
                    .child(icon("trash", 18., c.muted))
                    .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
            )
            .child(self.render_proxy_switch(cx))
            .child(
                div()
                    .flex_1()
                    .min_w(px(280.))
                    .h(px(40.))
                    .px(px(14.))
                    .rounded(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .bg(rgb(c.input))
                    .child(icon("search", 16., c.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(crate::MONO_FONT)
                            .font_features(gpui::FontFeatures::disable_ligatures())
                            .text_size(px(12.5))
                            .child(self.filter_input.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(12.))
                            .text_color(rgb(c.muted))
                            .child(count),
                    ),
            )
            .child(self.render_kinds(cx))
            .child(self.render_layout_buttons(cx))
    }

    fn render_proxy_switch(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let addr = self.engine.addr;
        let (text, on) = match self.system.proxy {
            ProxySwitch::Off => (
                tr!("System proxy off · {addr}", addr = addr.to_string()),
                false,
            ),
            ProxySwitch::Working => (tr!("System proxy…"), false),
            ProxySwitch::On(_) => (
                tr!("System proxy on · {addr}", addr = addr.to_string()),
                true,
            ),
        };
        pill_button("system-proxy")
            .px(px(14.))
            .gap(px(8.))
            .font_weight(FontWeight::BOLD)
            .when(on, |d| {
                d.bg(rgb(c.tonal))
                    .text_color(rgb(c.on_tonal))
                    .child(icon("check", 16., c.on_tonal))
            })
            .when(!on, |d| d.bg(rgb(c.surface)).text_color(rgb(c.muted)))
            .child(text)
            .on_click(cx.listener(|this, _, _, cx| this.toggle_system_proxy(cx)))
    }

    fn render_kinds(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let current = self.filter.kind;
        div()
            .flex()
            .gap(px(4.))
            .p(px(4.))
            .rounded(px(20.))
            .bg(rgb(c.surface))
            .children(Kind::ALL.into_iter().map(|kind| {
                let on = kind == current;
                div()
                    .id(("kind", kind as u64))
                    .h(px(32.))
                    .px(px(12.))
                    .rounded(px(16.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .reachable()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::BOLD)
                    .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                    .when(!on, |d| d.text_color(rgb(c.muted)))
                    .child(kind.label())
                    .on_click(cx.listener(move |this, _, _, cx| this.set_kind(kind, cx)))
            }))
    }

    fn render_layout_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let shown = self.layout.inspector_shown;
        let button = |id: &'static str, name: &'static str, on: bool| {
            div()
                .id(id)
                .size(px(32.))
                .rounded(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .reachable()
                .when(on, |d| d.bg(rgb(c.tonal)))
                .child(icon(name, 18., if on { c.on_tonal } else { c.muted }))
        };
        div()
            .flex()
            .gap(px(4.))
            .p(px(4.))
            .rounded(px(20.))
            .bg(rgb(c.surface))
            .child(
                button("sidebar", "sidebar", self.layout.sidebar_shown)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx))),
            )
            .child(
                button(
                    "beside",
                    "beside",
                    shown && self.layout.place == Place::Beside,
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_place(Place::Beside, cx))),
            )
            .child(
                button("below", "below", shown && self.layout.place == Place::Below)
                    .on_click(cx.listener(|this, _, _, cx| this.set_place(Place::Below, cx))),
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let chosen = self.filter.exact_host.clone();
        let chosen_app = self.filter.exact_app.clone();
        let chosen_device = self.filter.exact_device.clone();
        let total = self.total() as u64;
        let (shown, hidden) = self.layout.hosts.arrange(&self.hosts);
        let wanted = self.host_filter.read(cx).text().to_owned();
        let (shown, hidden) = (
            crate::hosts::matching(shown, &wanted),
            crate::hosts::matching(hidden, &wanted),
        );
        let none_match = !wanted.trim().is_empty() && shown.is_empty() && hidden.is_empty();
        let line = |id: SharedString, text: SharedString, count: u64, on: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(px(32.))
                .px(px(10.))
                .rounded(px(12.))
                .flex()
                .items_center()
                .gap(px(10.))
                .cursor_pointer()
                .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
                .when(!on, |d| d.text_color(rgb(c.text)))
                .child(div().flex_1().min_w_0().truncate().child(text))
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(12.))
                        .text_color(rgb(if on { c.on_tonal } else { c.muted }))
                        .child(group_digits(count)),
                )
        };
        div()
            .id("hosts")
            .w(px(self.layout.sidebar_w))
            .flex_shrink_0()
            .h_full()
            .p(px(8.))
            .rounded(px(22.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .overflow_y_scroll()
            .bg(rgb(c.surface))
            .when(!self.apps.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .px(px(6.))
                        .py(px(4.))
                        .child(section_badge(tr!("Apps"))),
                )
                .children(self.apps.iter().map(|(app, count)| {
                    let on = chosen_app.as_deref() == Some(app.as_ref());
                    let name = app.to_string();
                    line(format!("app-{app}").into(), app.clone(), *count, on).on_click(
                        cx.listener(move |this, _, _, cx| {
                            let next = (this.filter.exact_app.as_deref() != Some(name.as_str()))
                                .then(|| name.clone());
                            this.set_app(next, cx);
                        }),
                    )
                }))
                .child(div().h(px(8.)).flex_shrink_0())
            })
            .when(!self.device_counts.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .px(px(6.))
                        .py(px(4.))
                        .child(section_badge(tr!("Devices"))),
                )
                .children(self.device_counts.iter().map(|(ip, count)| {
                    let on = chosen_device.as_deref() == Some(ip.as_ref());
                    let shown: SharedString = self.devices.shown(ip).to_owned().into();
                    let ip = ip.to_string();
                    line(format!("device-{ip}").into(), shown, *count, on).on_click(cx.listener(
                        move |this, _, _, cx| {
                            let next = (this.filter.exact_device.as_deref() != Some(ip.as_str()))
                                .then(|| ip.clone());
                            this.set_device(next, cx);
                        },
                    ))
                }))
                .child(div().h(px(8.)).flex_shrink_0())
            })
            .child(self.render_hosts_heading(cx))
            .child(
                line(
                    "all-hosts".into(),
                    tr!("All hosts").into(),
                    total,
                    chosen.is_none(),
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_host(None, cx))),
            )
            .when(none_match, |d| {
                d.child(
                    div()
                        .px(px(10.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(rgb(c.muted))
                        .child(tr!("No host has this in its name")),
                )
            })
            .children(shown.into_iter().map(|host| {
                let on = chosen.as_deref() == Some(host.host.as_ref());
                let name = host.host.to_string();
                self.host_line(host, on, false, cx)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let next = (this.filter.exact_host.as_deref() != Some(name.as_str()))
                            .then(|| name.clone());
                        this.set_host(next, cx);
                    }))
            }))
            .when(!hidden.is_empty(), |d| {
                d.child(div().h(px(8.)).flex_shrink_0())
                    .child(
                        div()
                            .flex()
                            .px(px(6.))
                            .py(px(4.))
                            .child(section_badge(tr!("Hidden"))),
                    )
                    .children(
                        hidden
                            .into_iter()
                            .map(|host| self.host_line(host, false, true, cx)),
                    )
            })
    }

    /// "Hosts", and a field beside it that narrows them by name. Escape
    /// empties it, and only then goes on to whatever else it closes.
    fn render_hosts_heading(&self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .px(px(6.))
            .py(px(4.))
            .child(section_badge(tr!("Hosts")))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(26.))
                    .px(px(10.))
                    .rounded(px(13.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .bg(rgb(c.input))
                    .text_size(px(12.))
                    .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                        if ev.keystroke.key == "escape"
                            && !this.host_filter.read(cx).text().is_empty()
                        {
                            this.filter_hosts("", cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(icon("search", 12., c.muted))
                    .child(div().flex_1().min_w_0().child(self.host_filter.clone())),
            )
    }

    /// Types into the field that narrows the hosts. For tests, and Escape.
    #[doc(hidden)]
    pub fn filter_hosts(&mut self, text: &str, cx: &mut Context<Self>) {
        self.host_filter.update(cx, |input, cx| {
            input.set_text(text, text.len()..text.len(), cx)
        });
        cx.notify();
    }

    /// A host in the sidebar. On hover it can be pinned, or hidden; a hidden one
    /// can be shown again.
    fn host_line(
        &self,
        line: HostLine,
        on: bool,
        hidden: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = colors();
        let group: SharedString = format!("host-{}", line.host).into();
        let ink = if on { c.on_tonal } else { c.muted };
        let action = |id: String, icon_name: &'static str, always: bool| {
            div()
                .id(SharedString::from(id))
                .size(px(22.))
                .flex_shrink_0()
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|d| d.bg(rgb(c.page)))
                .when(!always, |d| {
                    d.invisible().group_hover(group.clone(), |s| s.visible())
                })
                .child(icon(icon_name, 13., ink))
        };
        let pin_host = line.host.to_string();
        let hide_host = line.host.to_string();
        let pin = (!hidden).then(|| {
            action(format!("pin-{}", line.host), "pin", line.pinned).on_click(cx.listener(
                move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.toggle_pin(&pin_host, cx);
                },
            ))
        });
        let eye = action(
            format!("{}-{}", if hidden { "show" } else { "hide" }, line.host),
            if hidden { "eye" } else { "eye-off" },
            hidden,
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            this.toggle_hidden(&hide_host, cx);
        }));
        div()
            .id(group.clone())
            .group(group)
            .flex_shrink_0()
            .h(px(32.))
            .px(px(10.))
            .rounded(px(12.))
            .flex()
            .items_center()
            .gap(px(2.))
            .when(!hidden, |d| d.cursor_pointer())
            .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
            .when(!on, |d| {
                d.text_color(rgb(if hidden { c.muted } else { c.text }))
            })
            .child(div().flex_1().min_w_0().truncate().child(line.host))
            .children(pin)
            .child(eye)
            // Last, so that counts line up with every other line's.
            .child(
                div()
                    .flex_shrink_0()
                    .pl(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(ink))
                    .child(group_digits(line.count)),
            )
    }

    fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let list = Rc::clone(&self.list);
        let count = self.count();
        let body = if count == 0 {
            self.render_empty().into_any_element()
        } else {
            uniform_list(
                "flows",
                count,
                cx.processor(|this, range: Range<usize>, _, cx| this.render_rows(range, cx)),
            )
            .size_full()
            .track_scroll(&self.scroll)
            .into_any_element()
        };
        div()
            .flex_1()
            .min_w(px(LIST_MIN))
            .min_h(px(120.))
            .h_full()
            .rounded(px(22.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(rgb(c.surface))
            .child(self.render_header(cx))
            .child(
                div()
                    .id("flow-list")
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, _| {
                        if f32::from(event.delta.pixel_delta(px(ROW_HEIGHT)).y) > 0. {
                            this.follow = false;
                        } else {
                            this.look_for_bottom = true;
                        }
                    }))
                    .child(
                        canvas(
                            move |b, window, _| {
                                // What is drawn from it was drawn from the old size.
                                if list.replace(b).size != b.size {
                                    window.request_animation_frame();
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(body)
                    .when(self.list_max_scroll() > 0., |d| {
                        let max = self.list_max_scroll();
                        let thumb = self.thumb_height();
                        let track = f32::from(self.list.get().size.height) - thumb;
                        let offset = -f32::from(self.scroll.0.borrow().base_handle.offset().y);
                        d.child(
                            div()
                                .id("list-bar")
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .right_0()
                                .w(px(BAR_W))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                        this.drag = Some(Drag::ListBar);
                                        this.drag_list_bar(event.position.y);
                                        cx.notify();
                                    }),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .top(px((offset / max).clamp(0., 1.) * track))
                                        .h(px(thumb))
                                        .left(px(2.))
                                        .right(px(2.))
                                        .rounded_sm()
                                        .bg(rgb(c.splitter)),
                                ),
                        )
                    })
                    .when(!self.follow && self.list_max_scroll() > 0., |d| {
                        d.child(
                            div()
                                .id("newest")
                                .absolute()
                                .bottom(px(12.))
                                .right(px(18.))
                                .h(px(32.))
                                .px(px(12.))
                                .rounded(px(16.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .cursor_pointer()
                                .reachable()
                                .bg(rgb(c.accent))
                                .text_color(rgb(c.on_accent))
                                .font_weight(FontWeight::BOLD)
                                .text_size(px(12.5))
                                .child(icon("down", 14., c.on_accent))
                                .child(tr!("Newest"))
                                .on_click(cx.listener(|this, _, _, cx| this.jump_to_newest(cx))),
                        )
                    }),
            )
    }

    fn render_empty(&self) -> impl IntoElement {
        let c = colors();
        let addr = self.engine.addr;
        let (title, hint): (SharedString, Option<String>) = if self.ids.is_some() || self.filtering
        {
            (tr!("No flow fits this filter").into(), None)
        } else {
            (
                tr!("Nothing yet").into(),
                Some(tr!(
                    "Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/",
                    addr = addr.to_string(),
                    ca = self.data_dir.join("ca.pem").display().to_string()
                )),
            )
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .p(px(24.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .children(hint.map(|hint| div().text_color(rgb(c.muted)).child(hint)))
    }

    fn splitter(
        &self,
        id: &'static str,
        drag: Drag,
        vertical: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = colors();
        let handle = if vertical {
            div().w(px(6.)).h(px(56.))
        } else {
            div().h(px(6.)).w(px(56.))
        };
        let holder = div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0();
        let holder = if vertical {
            holder.w(px(10.)).h_full().cursor_col_resize()
        } else {
            holder.h(px(10.)).w_full().cursor_row_resize()
        };
        holder
            .child(handle.rounded(px(3.)).bg(rgb(c.splitter)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.drag = Some(drag);
                    cx.notify();
                }),
            )
    }

    /// The places the inspector can be dropped, over the panels.
    fn render_drop_zones(&self, hover: Place) -> impl IntoElement {
        let c = colors();
        let content = self.content.get();
        let width = f32::from(content.size.width);
        let height = f32::from(content.size.height);
        let left = if self.layout.sidebar_shown {
            self.layout.sidebar_w + 10.
        } else {
            0.
        };
        let zone = |place: Place, label: String| {
            let on = hover == place;
            let base = div()
                .absolute()
                .rounded(px(24.))
                .border_2()
                .flex()
                .items_center()
                .justify_center()
                .border_color(rgb(if on { c.accent } else { c.splitter }))
                .bg(rgba(with_alpha(c.accent, if on { 0x30 } else { 0x0c })))
                .child(
                    div()
                        .px(px(14.))
                        .py(px(8.))
                        .rounded_full()
                        .font_weight(FontWeight::BOLD)
                        .bg(rgb(if on { c.accent } else { c.surface }))
                        .text_color(rgb(if on { c.on_accent } else { c.text }))
                        .child(label),
                );
            match place {
                Place::Beside => base.top_0().bottom_0().right_0().w(px((width - left) / 3.)),
                Place::Below => base
                    .left(px(left))
                    .bottom_0()
                    .h(px(height / 2.))
                    .w(px((width - left) * 2. / 3. - 10.)),
            }
        };
        div()
            .absolute()
            .size_full()
            .bg(rgba(with_alpha(c.page, 0xd8)))
            .child(zone(Place::Beside, tr!("Beside the list")))
            .child(zone(Place::Below, tr!("Below the list")))
            .child(
                div()
                    .absolute()
                    .top(px(12.))
                    .left(px(left + 12.))
                    .px(px(14.))
                    .py(px(8.))
                    .rounded_full()
                    .font_weight(FontWeight::BOLD)
                    .bg(rgb(c.surface))
                    .text_color(rgb(c.text))
                    .child(tr!("Drop it on a place · Esc puts it back")),
            )
    }

    fn render_rail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let current = self.screen;
        let waiting = self.paused_count();
        let item = |id: &'static str, label: String, name: &'static str, screen: Screen| {
            let on = current == screen;
            let ink = if on { c.on_tonal } else { c.muted };
            div()
                .id(id)
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .cursor_pointer()
                .reachable()
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(ink))
                .child(
                    div()
                        .w(px(52.))
                        .h(px(30.))
                        .rounded(px(15.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(on, |d| d.bg(rgb(c.tonal)))
                        .child(icon(name, 18., ink)),
                )
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.set_screen(screen, cx)))
        };
        div()
            .w(px(76.))
            .flex_shrink_0()
            .h_full()
            .py(px(16.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(14.))
            .child(
                div()
                    .size(px(44.))
                    .mb(px(4.))
                    .rounded(px(15.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(c.accent))
                    .child(icon("logo", 24., c.on_accent)),
            )
            .child(item("rail-setup", tr!("Set up"), "setup", Screen::Setup))
            .child(item(
                "rail-traffic",
                tr!("Traffic"),
                "traffic",
                Screen::Traffic,
            ))
            .child(item("rail-rules", tr!("Rules"), "rules", Screen::Rules))
            .child(
                div()
                    .relative()
                    .child(item("rail-paused", tr!("Paused"), "pause", Screen::Paused))
                    .when(waiting > 0, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(-4.))
                                .right(px(2.))
                                .min_w(px(18.))
                                .h(px(18.))
                                .px(px(5.))
                                .rounded(px(9.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::BOLD)
                                .bg(rgb(c.warn))
                                .text_color(rgb(c.on_accent))
                                .child(waiting.to_string()),
                        )
                    }),
            )
            .child(item(
                "rail-compose",
                tr!("Compose"),
                "send",
                Screen::Compose,
            ))
            .child(item(
                "rail-sessions",
                tr!("Sessions"),
                "sessions",
                Screen::Sessions,
            ))
            .child(item(
                "rail-settings",
                tr!("Settings"),
                "settings",
                Screen::Settings,
            ))
    }

    fn render_ca(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let row = div().flex().flex_row().items_center().gap(px(6.));
        match self.system.ca {
            CaState::Trusted => row
                .child(icon("shield", 14., c.ok))
                .child(tr!("Certificate trusted on this computer"))
                .into_any_element(),
            CaState::Checking => row
                .child(tr!("Checking the certificate…"))
                .into_any_element(),
            CaState::Installing => row
                .child(tr!(
                    "Installing the certificate; the system may ask you to confirm…"
                ))
                .into_any_element(),
            CaState::Removing => row
                .child(tr!(
                    "Removing the certificate; the system may ask you to confirm…"
                ))
                .into_any_element(),
            CaState::NotTrusted => row
                .id("install-ca")
                .cursor_pointer()
                .reachable()
                .text_color(rgb(c.warn))
                .child(icon("shield-off", 14., c.warn))
                .child(tr!(
                    "HTTPS will fail until the certificate is trusted: set up"
                ))
                .on_click(cx.listener(|this, _, _, cx| this.set_screen(Screen::Setup, cx)))
                .into_any_element(),
        }
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let stats = &self.engine.shared.stats;
        let shed = stats.shed.load(Ordering::Relaxed);
        let dropped =
            stats.dropped.load(Ordering::Relaxed) + stats.rows_lost.load(Ordering::Relaxed);
        let low = self.engine.shared.low_disk.load(Ordering::Relaxed);
        let trimmed = stats.trimmed.load(Ordering::Relaxed);
        let mut notes: Vec<(String, u32)> = Vec::new();
        if let Some(problem) = &self.system.problem {
            notes.push((problem.clone(), c.error));
        }
        if let Some(problem) = &self.settings_problem {
            notes.push((
                tr!("settings.toml was not used: {problem}", problem = problem),
                c.error,
            ));
        }
        if let Some(problem) = &self.settings_left {
            notes.push((
                tr!("A setting was not saved: {problem}", problem = problem),
                c.warn,
            ));
        }
        if let Some(failure) = self.engine.failure() {
            notes.push((
                tr!("The proxy stopped: {failure}", failure = failure),
                c.error,
            ));
        }
        if low {
            notes.push((
                tr!("The disk is nearly full: bodies are not being kept"),
                c.warn,
            ));
        }
        if dropped > 0 {
            notes.push((
                tr_n!(
                    dropped,
                    "{count} flow not recorded",
                    "{count} flows not recorded",
                    count = group_digits(dropped)
                ),
                c.warn,
            ));
        }
        if shed > 0 {
            notes.push((
                tr_n!(
                    shed,
                    "{count} flow without bodies",
                    "{count} flows without bodies",
                    count = group_digits(shed)
                ),
                c.warn,
            ));
        }
        if let Some((note, failed)) = &self.sessions_page.files.note {
            notes.push((note.clone(), if *failed { c.error } else { c.muted }));
        }
        if trimmed > 0 {
            notes.push((
                tr_n!(
                    trimmed,
                    "{count} oldest flow dropped to stay under {size}",
                    "{count} oldest flows dropped to stay under {size}",
                    count = group_digits(trimmed),
                    size = crate::flows::size_text(self.engine.settings.session_max())
                ),
                c.muted,
            ));
        }
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(px(18.))
            .px(px(8.))
            .min_h(px(28.))
            .text_size(px(12.))
            .text_color(rgb(c.muted))
            .child(tr_n!(
                self.total(),
                "{count} flow",
                "{count} flows",
                count = group_digits(self.total() as u64)
            ))
            .when(self.total() > 0, |d| {
                d.child(
                    div()
                        .id("export-har")
                        .cursor_pointer()
                        .reachable()
                        .text_color(rgb(c.link))
                        .child(tr!("Export as HAR"))
                        .on_click(cx.listener(|this, _, _, cx| this.ask_export_shown(cx))),
                )
            })
            .child(self.render_ca(cx))
            .child(
                div()
                    .id("shortcuts-link")
                    .cursor_pointer()
                    .reachable()
                    .text_color(rgb(c.link))
                    .child(tr!("Shortcuts {keys}", keys = shortcuts::label("mod+/")))
                    .on_click(cx.listener(|this, _, _, cx| this.set_shortcuts(true, cx))),
            )
            .children(
                notes
                    .into_iter()
                    .map(|(text, color)| div().text_color(rgb(color)).child(text)),
            )
    }
}

impl Focusable for Traffic {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Traffic {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let setup =
            (self.screen == Screen::Setup).then(|| self.render_setup(cx).into_any_element());
        let sessions =
            (self.screen == Screen::Sessions).then(|| self.render_sessions(cx).into_any_element());
        let banner = self
            .render_viewing_banner(cx)
            .map(IntoElement::into_any_element);
        let rules =
            (self.screen == Screen::Rules).then(|| self.render_rules(cx).into_any_element());
        let paused =
            (self.screen == Screen::Paused).then(|| self.render_paused(cx).into_any_element());
        let compose =
            (self.screen == Screen::Compose).then(|| self.render_compose(cx).into_any_element());
        let settings = (self.screen == Screen::Settings)
            .then(|| self.render_settings(window, cx).into_any_element());
        let content_bounds = Rc::clone(&self.content);
        let place = self.layout.place;
        let shown = self.layout.inspector_shown;
        let drop_zones = match self.drag {
            Some(Drag::Inspector(hover)) => Some(self.render_drop_zones(hover).into_any_element()),
            _ => None,
        };
        let list = self.render_list(cx).into_any_element();
        let center = if shown {
            let inspector = self.render_inspector(cx).into_any_element();
            match place {
                Place::Beside => div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_row()
                    .child(list)
                    .child(self.splitter("split-inspector", Drag::InspectorSize, true, cx))
                    .child(
                        div()
                            .w(px(self.layout.inspector_w))
                            .flex_shrink_0()
                            .h_full()
                            .child(inspector),
                    ),
                Place::Below => div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(div().flex_1().min_h_0().w_full().flex().child(list))
                    .child(self.splitter("split-inspector", Drag::InspectorSize, false, cx))
                    .child(
                        div()
                            .h(px(self.layout.inspector_h))
                            .flex_shrink_0()
                            .w_full()
                            .child(inspector),
                    ),
            }
        } else {
            div().flex_1().min_w_0().h_full().flex().child(list)
        };

        let body = div()
            .relative()
            .track_focus(&self.focus)
            .key_context("Traffic")
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_hidden()
            .flex()
            .flex_row()
            .text_color(rgb(c.text))
            .font_family(crate::FONT)
            .text_size(px(13.))
            .child(self.render_rail(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .pt(px(14.))
                    .pr(px(14.))
                    .pb(px(10.))
                    .pl(px(4.))
                    .when_some(setup, |d, setup| d.child(setup))
                    .when_some(rules, |d, rules| d.child(rules))
                    .when_some(paused, |d, paused| d.child(paused))
                    .when_some(compose, |d, compose| d.child(compose))
                    .when_some(sessions, |d, sessions| d.child(sessions))
                    .when_some(settings, |d, settings| d.child(settings))
                    .when(self.screen == Screen::Traffic, |d| {
                        d.child(self.render_toolbar(cx)).children(banner).child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .relative()
                                .flex()
                                .flex_row()
                                .child(
                                    canvas(move |b, _, _| content_bounds.set(b), |_, _, _, _| {})
                                        .absolute()
                                        .size_full(),
                                )
                                .when(self.layout.sidebar_shown, |d| {
                                    d.child(self.render_sidebar(cx)).child(self.splitter(
                                        "split-sidebar",
                                        Drag::Sidebar,
                                        true,
                                        cx,
                                    ))
                                })
                                .child(center)
                                .children(drop_zones),
                        )
                    })
                    .child(self.render_footer(cx)),
            )
            .when(self.shortcuts_open, |d| {
                d.child(deferred(self.render_shortcuts(cx)).with_priority(2))
            });
        let switch = chrome::appearance_switch(self.layout.appearance).on_click(cx.listener(
            |this, _, window, cx| {
                let next = this.layout.appearance.next();
                this.set_appearance(next, window, cx);
            },
        ));
        chrome::window_chrome(body, rgb(c.page).into(), switch, window, cx)
    }
}

/// The path is never squeezed below this.
const PATH_MIN: f32 = 80.;
/// Room at the end of the header for the menu of columns, kept in each row too.
const MENU_W: f32 = 26.;
/// How far either side of a column's edge it can be taken hold of.
const EDGE_GRIP: f32 = 4.;

/// A column's cell, in the header or a row: as wide as it is set to, or taking
/// what is left.
fn cell(column: Column, width: f32) -> Div {
    let d = div();
    let d = if column.fills() {
        d.flex_1().min_w(px(PATH_MIN))
    } else if let Some(min) = column.gives_way_to() {
        d.w(px(width)).min_w(px(min.min(width)))
    } else {
        d.w(px(width)).flex_shrink_0()
    };
    if column.on_the_right() {
        d.flex().justify_end()
    } else {
        d
    }
}

impl Traffic {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let sort = self.filter.sort;
        let cells = self.shown_columns().into_iter().map(|(column, width)| {
            let sorted = sort.filter(|sort| sort.column == column);
            let label = div()
                .id(SharedString::from(format!("column-{}", column.key())))
                .h_full()
                .w_full()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .when(column.on_the_right(), |d| d.justify_end())
                // Text on the left keeps clear of the next column; numbers end at the edge.
                .when(!column.fills() && !column.on_the_right(), |d| d.pr(px(8.)))
                .cursor_pointer()
                .reachable()
                .when(sorted.is_some(), |d| d.text_color(rgb(c.text)))
                .child(div().min_w_0().truncate().child(column.label()))
                .children(
                    sorted
                        .map(|sort| icon(if sort.descending { "down" } else { "up" }, 12., c.text)),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.sort_by(column, cx)));
            let edge = (!column.fills()).then(|| {
                div()
                    .id(SharedString::from(format!("edge-{}", column.key())))
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .w(px(EDGE_GRIP * 2.))
                    .when(column.on_the_right(), |d| d.left(px(-EDGE_GRIP)))
                    .when(!column.on_the_right(), |d| d.right(px(-EDGE_GRIP)))
                    .cursor_col_resize()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.drag = Some(Drag::Column {
                                column,
                                from_x: f32::from(event.position.x),
                                from_w: this.layout.columns.width(column),
                            });
                            cx.notify();
                        }),
                    )
            });
            cell(column, width)
                .h_full()
                .relative()
                .child(label)
                .children(edge)
        });
        div()
            .h(px(36.))
            .flex_shrink_0()
            .flex()
            .flex_row()
            .items_center()
            .px(px(12.))
            .border_b_1()
            .border_color(rgb(c.line))
            .text_size(px(11.5))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(c.muted))
            .children(cells)
            .child(self.render_columns_button(cx))
    }

    /// The button at the end of the header, and the menu it opens.
    fn render_columns_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let open = self.columns_menu;
        div()
            .w(px(MENU_W))
            .h_full()
            .flex_shrink_0()
            .relative()
            .flex()
            .items_center()
            .justify_end()
            .child(
                div()
                    .id("columns-menu")
                    .size(px(22.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .reachable()
                    .when(open, |d| d.bg(rgb(c.tonal)))
                    .hover(|d| d.bg(rgb(c.tonal)))
                    .child(icon(
                        "columns",
                        14.,
                        if open { c.on_tonal } else { c.muted },
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_columns_menu(!open, cx))),
            )
            .when(open, |d| {
                d.child(
                    div().absolute().top(px(30.)).right_0().child(deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .snap_to_window_with_margin(px(8.))
                            .child(self.render_columns_menu(cx)),
                    )),
                )
            })
    }

    fn render_columns_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let columns = &self.layout.columns;
        let item = |id: SharedString, on: Option<bool>, text: String| {
            div()
                .id(id)
                .h(px(30.))
                .px(px(10.))
                .rounded(px(8.))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .reachable()
                .hover(|d| d.bg(rgb(c.tonal)))
                .child(
                    div()
                        .w(px(14.))
                        .flex_shrink_0()
                        .children(on.filter(|on| *on).map(|_| icon("check", 14., c.text))),
                )
                .child(text)
        };
        div()
            .id("columns-list")
            .occlude()
            .w(px(200.))
            .p(px(6.))
            .rounded(px(14.))
            .border_1()
            .border_color(rgb(c.line))
            .bg(rgb(c.surface))
            .shadow_lg()
            .flex()
            .flex_col()
            .text_size(px(13.))
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(c.text))
            .child(eyebrow(tr!("Columns")).px(px(10.)).py(px(4.)))
            .children(
                Column::ALL
                    .into_iter()
                    .filter(|c| !c.fills())
                    .map(|column| {
                        item(
                            format!("show-{}", column.key()).into(),
                            Some(columns.is_shown(column)),
                            column.name(),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_column(column, cx)))
                    }),
            )
            .child(div().h(px(1.)).my(px(4.)).bg(rgb(c.line)))
            .child(
                item("reset-columns".into(), None, tr!("Back to defaults"))
                    .when(columns.is_default(), |d| d.text_color(rgb(c.muted)))
                    .on_click(cx.listener(|this, _, _, cx| this.reset_columns(cx))),
            )
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.set_columns_menu(false, cx)))
    }
}

fn render_row(
    row: Row,
    sender: SharedString,
    columns: &[(Column, f32)],
    selected: bool,
) -> Stateful<Div> {
    let c = colors();
    // A gRPC call that failed is colored as the HTTP status that says so.
    let (ground, ink) = status_colors(row.grpc_failed.map(crate::grpc::as_http).or(row.status));
    let Row {
        id,
        number,
        method,
        status,
        host,
        path,
        graphql,
        note,
        size,
        time,
        slow,
        ..
    } = row;
    let status: SharedString = status.map_or("ERR".into(), |status| status.to_string().into());
    // A column of its own says it, so the path need not.
    let operation_shown = columns
        .iter()
        .any(|(column, _)| *column == Column::Operation);
    let cells = columns.iter().map(|&(column, width)| {
        let d = cell(column, width);
        match column {
            Column::Number => d
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(11.5))
                .text_color(rgb(c.muted))
                .truncate()
                .child(number.clone()),
            Column::Method => d
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .truncate()
                .child(method.clone()),
            Column::Status => d.child(
                div()
                    .flex()
                    .w_auto()
                    .max_w(px(44.))
                    .justify_center()
                    .px(px(6.))
                    .py(px(1.))
                    .rounded(px(8.))
                    .bg(rgb(ground))
                    .text_color(rgb(ink))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::BOLD)
                    .child(status.clone()),
            ),
            Column::App => d
                .pr(px(8.))
                .truncate()
                .text_size(px(12.))
                .text_color(rgb(c.muted))
                .child(sender.clone()),
            Column::Host => d.pr(px(8.)).truncate().child(host.clone()),
            Column::Path => d
                .flex()
                .flex_row()
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(12.))
                .children(
                    graphql
                        .clone()
                        .filter(|_| !operation_shown)
                        .map(|operation| {
                            div()
                                // The operation says more than `/graphql` does.
                                .flex_shrink_0()
                                .max_w(gpui::relative(0.8))
                                .mr(px(8.))
                                .px(px(6.))
                                .rounded(px(6.))
                                .truncate()
                                .bg(rgb(c.tonal))
                                .text_color(rgb(c.on_tonal))
                                .child(operation)
                        }),
                )
                .child(div().min_w_0().truncate().child(path.clone()))
                .children(note.clone().map(|note| {
                    div()
                        .min_w_0()
                        .max_w(gpui::relative(0.6))
                        .truncate()
                        .pl(px(10.))
                        .text_color(rgb(c.muted))
                        .child(note)
                })),
            Column::Operation => d
                .pr(px(8.))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(12.))
                .text_color(rgb(c.on_tonal))
                .truncate()
                .children(graphql.clone()),
            Column::Size => d.text_color(rgb(c.muted)).child(size.clone()),
            Column::Time => d
                .text_color(rgb(if slow { c.slow } else { c.muted }))
                .when(slow, |d| d.font_weight(FontWeight::BOLD))
                .child(time.clone()),
        }
    });
    div()
        .id(("flow", id as u64))
        .h(px(ROW_HEIGHT))
        .w_full()
        .px(px(6.))
        .py(px(1.))
        .child(
            div()
                .h_full()
                .px(px(6.))
                .rounded(px(10.))
                .flex()
                .flex_row()
                .items_center()
                .when(selected, |d| d.bg(rgb(c.selected_row)))
                .children(cells)
                .child(div().w(px(MENU_W)).flex_shrink_0()),
        )
}

fn pill_button(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(40.))
        .rounded(px(20.))
        .flex()
        .items_center()
        .cursor_pointer()
}
