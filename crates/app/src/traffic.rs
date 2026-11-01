//! The main screen: traffic as it happens. The toolbar, the hosts, the list, and
//! the inspector beside it or below it, which can be moved by dragging, from the
//! toolbar, or from the keyboard. Looks at the session a few times a second; all
//! that can take long is done on a worker thread, so a flood or a million rows
//! never holds up a frame.

use crate::sessions_page::{SessionsPage, when_text};
use crate::{
    chrome,
    colors::{self, colors, with_alpha},
    engine::Engine,
    filter::{Filter, Kind},
    flows::{Flows, Row, group_digits},
    input::{Changed, TextInput},
    inspector::InspectorState,
    layout::{Appearance, INSPECTOR_MIN, LIST_MIN, Layout, Place, SIDEBAR_MAX, SIDEBAR_MIN},
    placement,
    settings_page::SettingsPage,
    system::{CaState, ProxySwitch, SystemState},
    widgets::{eyebrow, icon, status_colors},
};
use gpui::{
    Bounds, ClickEvent, Context, Div, Entity, FocusHandle, Focusable, FontWeight, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render,
    ScrollStrategy, ScrollWheelEvent, SharedString, Stateful, UniformListScrollHandle, Window,
    WindowAppearance, canvas, div, point, prelude::*, px, rgb, rgba, uniform_list,
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
    Filter { generation: u64, filter: Filter },
    Hosts { from_id: i64 },
}

enum Done {
    Filter {
        generation: u64,
        ids: Vec<i64>,
        took: Duration,
    },
    Hosts {
        hosts: Vec<(SharedString, u64)>,
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
            let hosts = host_counts(conn, from_id).unwrap_or_default();
            let took = started.elapsed();
            let _ = done.send(Done::Hosts { hosts, took });
        }
    }
}

fn host_counts(conn: &Connection, from_id: i64) -> rusqlite::Result<Vec<(SharedString, u64)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT host, count(*) AS n FROM flows WHERE id >= ?1 GROUP BY host ORDER BY n DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![from_id, HOSTS_SHOWN], |r| {
        Ok((
            SharedString::from(r.get::<_, String>(0)?),
            r.get::<_, i64>(1)? as u64,
        ))
    })?;
    rows.collect()
}

/// What the mouse is holding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
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
    /// The system's own appearance is dark.
    system_dark: bool,
    pub(crate) data_dir: PathBuf,
    pub screen: Screen,
    pub system: SystemState,
    pub settings_page: Option<SettingsPage>,
    /// Why the settings file was not used, if it was not.
    pub settings_problem: Option<String>,
    layout_path: PathBuf,
    pub(crate) layout: Layout,
    flows: Option<Flows>,
    jobs: Sender<Job>,
    done: Receiver<Done>,
    filter_input: Entity<TextInput>,
    filter: Filter,
    /// With a filter on, the flows that fit. Without one, every id in `bounds`.
    ids: Option<Arc<Vec<i64>>>,
    generation: u64,
    filter_due: Instant,
    filter_seen: i64,
    filtering: bool,
    bounds: Option<(i64, i64)>,
    hosts: Vec<(SharedString, u64)>,
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
    focus: FocusHandle,
    last_count: usize,
    rows_missing: bool,
    places_asked: usize,
    bounds_change: u64,
    /// Who last reached the test page, as the window has seen it.
    pub check_visit: Option<CheckVisit>,
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

        let filter_input = cx.new(|cx| TextInput::new("host:, status:4xx, or text in bodies", cx));
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
        cx.on_app_quit(|this, _| {
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
        cx.defer_in(window, |this, _, cx| this.start_ca_check(cx));
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
            system_dark,
            data_dir: data_dir.to_owned(),
            screen: Screen::Traffic,
            system: SystemState::default(),
            settings_page: None,
            settings_problem: None,
            layout,
            layout_path,
            flows: None,
            jobs: job_tx,
            done: done_rx,
            filter_input,
            filter: Filter::default(),
            ids: None,
            generation: 0,
            filter_due: now,
            filter_seen: 0,
            filtering: false,
            bounds: None,
            hosts: Vec::new(),
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
        // Once left, it does not open by itself again.
        if self.screen == Screen::Setup && !self.layout.setup_seen {
            self.layout.setup_seen = true;
            self.save_layout();
        }
        self.screen = screen;
        if screen == Screen::Sessions {
            self.refresh_sessions(cx);
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
            Err(e) => self.system.problem = Some(format!("The search index: {e:#}")),
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
                        .child(format!("An earlier session: {name}")),
                )
                .child(div().child(if self.catching_up.is_some() {
                    "Read-only. Search covers more as its index is finished. Recording goes on."
                } else {
                    "Read-only. Recording goes on."
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
                        .bg(rgb(c.accent))
                        .text_color(rgb(c.on_accent))
                        .font_weight(FontWeight::BOLD)
                        .child("Back to live")
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
                Done::Hosts { hosts, took } => {
                    self.hosts = hosts;
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
        let visit = self.engine.check.last();
        if visit != self.check_visit {
            self.check_visit = visit;
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
            filter,
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
        // so its answer starts at the top.
        self.follow = self.viewing.is_none();
        self.last_count = usize::MAX;
        if self.viewing.is_some() {
            self.scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(0.)));
        }
        cx.notify();
    }

    pub fn set_filter_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let mut filter = Filter::parse(text);
        filter.kind = self.filter.kind;
        filter.exact_host = self.filter.exact_host.clone();
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

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.cleared_to = self.bounds.map_or(0, |(_, last)| last + 1);
        self.hosts.clear();
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

    fn save_layout(&self) {
        self.layout.save(&self.layout_path);
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

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        let secondary = keys.modifiers.secondary();
        match keys.key.as_str() {
            "escape" if self.drag.is_some() => {
                self.drag = None;
                cx.notify();
            }
            "f" if secondary => {
                let handle = self.filter_input.read(cx).focus_handle(cx);
                window.focus(&handle, cx);
            }
            "\\" if secondary => {
                let place = match self.layout.place {
                    Place::Beside => Place::Below,
                    Place::Below => Place::Beside,
                };
                self.set_place(place, cx);
            }
            "b" if secondary => self.toggle_sidebar(cx),
            "i" if secondary => self.toggle_inspector(cx),
            "up" if !self.filter_has_focus(window, cx) => self.move_selection(-1, cx),
            "down" if !self.filter_has_focus(window, cx) => self.move_selection(1, cx),
            _ => {}
        }
    }

    fn filter_has_focus(&self, window: &Window, cx: &Context<Self>) -> bool {
        self.filter_input
            .read(cx)
            .focus_handle(cx)
            .is_focused(window)
    }

    /// Exactly one element per place asked for, always. `uniform_list` measures the
    /// first to learn the height of them all: given none it takes the height for
    /// zero, thinks every row is in view and asks for all of them, which at a million
    /// rows ran the machine out of memory.
    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<Stateful<Div>> {
        self.places_asked = range.len();
        let ids = self.ids_at(range.clone());
        let selected = self.selected;
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
                    render_row(row, selected == Some(id)).on_click(cx.listener(
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
            format!(
                "{} of {}",
                group_digits(self.count() as u64),
                group_digits(self.total() as u64)
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
                    .child(if recording { "Recording" } else { "Paused" })
                    .on_click(cx.listener(|this, _, _, cx| {
                        let on = !this.engine.is_recording();
                        this.engine.set_recording(on);
                        cx.notify();
                    })),
            )
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
            ProxySwitch::Off => (format!("System proxy off · {addr}"), false),
            ProxySwitch::Working => ("System proxy…".to_owned(), false),
            ProxySwitch::On(_) => (format!("System proxy on · {addr}"), true),
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
                    .id(kind.label())
                    .h(px(32.))
                    .px(px(12.))
                    .rounded(px(16.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
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
        let total = self.total() as u64;
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
            .child(eyebrow("Hosts").px(px(10.)).py(px(4.)))
            .child(
                line(
                    "all-hosts".into(),
                    "All hosts".into(),
                    total,
                    chosen.is_none(),
                )
                .on_click(cx.listener(|this, _, _, cx| this.set_host(None, cx))),
            )
            .children(self.hosts.iter().map(|(host, count)| {
                let on = chosen.as_deref() == Some(host.as_ref());
                let name = host.to_string();
                line(format!("host-{host}").into(), host.clone(), *count, on).on_click(cx.listener(
                    move |this, _, _, cx| {
                        let next = (this.filter.exact_host.as_deref() != Some(name.as_str()))
                            .then(|| name.clone());
                        this.set_host(next, cx);
                    },
                ))
            }))
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
            .child(render_header())
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
                                .bg(rgb(c.accent))
                                .text_color(rgb(c.on_accent))
                                .font_weight(FontWeight::BOLD)
                                .text_size(px(12.5))
                                .child(icon("down", 14., c.on_accent))
                                .child("Newest")
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
            ("No flow fits this filter".into(), None)
        } else {
            (
                "Nothing yet".into(),
                Some(format!(
                    "Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {} https://example.com/",
                    self.data_dir.join("ca.pem").display()
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
        let zone = |place: Place, label: &'static str| {
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
            .child(zone(Place::Beside, "Beside the list"))
            .child(zone(Place::Below, "Below the list"))
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
                    .child("Drop it on a place · Esc puts it back"),
            )
    }

    fn render_rail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let current = self.screen;
        let item = |id: &'static str, label: &'static str, name: &'static str, screen: Screen| {
            let on = current == screen;
            let ink = if on { c.on_tonal } else { c.muted };
            div()
                .id(id)
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .cursor_pointer()
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
            .child(item("rail-setup", "Set up", "setup", Screen::Setup))
            .child(item("rail-traffic", "Traffic", "traffic", Screen::Traffic))
            .child(item(
                "rail-sessions",
                "Sessions",
                "sessions",
                Screen::Sessions,
            ))
            .child(item(
                "rail-settings",
                "Settings",
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
                .child("Certificate trusted on this computer")
                .into_any_element(),
            CaState::Checking => row.child("Checking the certificate…").into_any_element(),
            CaState::Installing => row
                .child("Installing the certificate; the system may ask you to confirm…")
                .into_any_element(),
            CaState::NotTrusted => row
                .id("install-ca")
                .cursor_pointer()
                .text_color(rgb(c.warn))
                .child(icon("shield-off", 14., c.warn))
                .child("HTTPS will fail until the certificate is trusted: set up")
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
            notes.push((format!("settings.toml was not used: {problem}"), c.error));
        }
        if let Some(failure) = self.engine.failure() {
            notes.push((format!("The proxy stopped: {failure}"), c.error));
        }
        if low {
            notes.push((
                "The disk is nearly full: bodies are not being kept".to_owned(),
                c.warn,
            ));
        }
        if dropped > 0 {
            notes.push((
                format!("{} flows not recorded", group_digits(dropped)),
                c.warn,
            ));
        }
        if shed > 0 {
            notes.push((
                format!("{} flows without bodies", group_digits(shed)),
                c.warn,
            ));
        }
        if trimmed > 0 {
            notes.push((
                format!(
                    "{} oldest flows dropped to stay under {}",
                    group_digits(trimmed),
                    crate::flows::size_text(self.engine.settings.session_max())
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
            .child(match self.total() {
                1 => "1 flow".to_owned(),
                total => format!("{} flows", group_digits(total as u64)),
            })
            .child(self.render_ca(cx))
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
            );
        let switch = chrome::appearance_switch(self.layout.appearance).on_click(cx.listener(
            |this, _, window, cx| {
                let next = this.layout.appearance.next();
                this.set_appearance(next, window, cx);
            },
        ));
        chrome::window_chrome(body, rgb(c.page).into(), switch, window, cx)
    }
}

const NUMBER_W: f32 = 76.;
const METHOD_W: f32 = 66.;
const STATUS_W: f32 = 60.;
const HOST_W: f32 = 170.;
const SIZE_W: f32 = 74.;
const TIME_W: f32 = 70.;

fn render_header() -> impl IntoElement {
    let c = colors();
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
        .child(div().w(px(NUMBER_W)).flex_shrink_0().child("#"))
        .child(div().w(px(METHOD_W)).flex_shrink_0().child("Method"))
        .child(div().w(px(STATUS_W)).flex_shrink_0().child("Status"))
        .child(div().w(px(HOST_W)).flex_shrink_0().child("Host"))
        .child(div().flex_1().min_w_0().child("Path"))
        .child(
            div()
                .w(px(SIZE_W))
                .flex_shrink_0()
                .flex()
                .justify_end()
                .child("Size"),
        )
        .child(
            div()
                .w(px(TIME_W))
                .flex_shrink_0()
                .flex()
                .justify_end()
                .child("Time"),
        )
}

fn render_row(row: Row, selected: bool) -> Stateful<Div> {
    let c = colors();
    let (ground, ink) = status_colors(row.status);
    let status: SharedString = row
        .status
        .map_or("ERR".into(), |status| status.to_string().into());
    div()
        .id(("flow", row.id as u64))
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
                .child(
                    div()
                        .w(px(NUMBER_W))
                        .flex_shrink_0()
                        .font_family(crate::MONO_FONT)
                        .text_size(px(11.5))
                        .text_color(rgb(c.muted))
                        .child(row.number),
                )
                .child(
                    div()
                        .w(px(METHOD_W))
                        .flex_shrink_0()
                        .text_size(px(12.))
                        .font_weight(FontWeight::BOLD)
                        .child(row.method),
                )
                .child(
                    div().w(px(STATUS_W)).flex_shrink_0().child(
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
                            .child(status),
                    ),
                )
                .child(
                    div()
                        .w(px(HOST_W))
                        .flex_shrink_0()
                        .pr(px(8.))
                        .truncate()
                        .child(row.host),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_row()
                        .font_family(crate::MONO_FONT)
                        .text_size(px(12.))
                        .child(div().min_w_0().truncate().child(row.path))
                        .children(row.note.map(|note| {
                            div()
                                .flex_shrink_0()
                                .max_w(px(220.))
                                .truncate()
                                .pl(px(10.))
                                .text_color(rgb(c.muted))
                                .child(note)
                        })),
                )
                .child(
                    div()
                        .w(px(SIZE_W))
                        .flex_shrink_0()
                        .flex()
                        .justify_end()
                        .text_color(rgb(c.muted))
                        .child(row.size),
                )
                .child(
                    div()
                        .w(px(TIME_W))
                        .flex_shrink_0()
                        .flex()
                        .justify_end()
                        .text_color(rgb(if row.slow { c.slow } else { c.muted }))
                        .when(row.slow, |d| d.font_weight(FontWeight::BOLD))
                        .child(row.time),
                ),
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
