//! A body in a panel. It keeps its own position, a line and a pixel offset into it,
//! and draws only the lines in view, because a body can have more lines than a
//! pixel offset in an `f32` can tell apart: `uniform_list`'s step grows to 8 px at
//! 6 million lines of 20 px.

mod find;

use find::found_style;
pub use find::{FOUND_MAX, FindCount};

use crate::scrollbar::Scrolled;
use crate::text_size::ui;
use crate::{
    body::{Body, Kind, Line, Token},
    colors::colors,
    flows::size_text,
    reading,
    remembered::Remembered,
    select::{self, COPY_MAX, Copied, Selection, Spot},
    syntax::Syntax,
    table::{self, Table},
    wrap::{self, Place},
};
use gpui::{
    AnyElement, Bounds, ClipboardItem, Context, FocusHandle, FontWeight, HighlightStyle, Hsla,
    ListHorizontalSizingBehavior, MouseButton, MouseDownEvent, Pixels, Point, Render,
    ScrollWheelEvent, SharedString, UniformListScrollHandle, Window, canvas, combine_highlights,
    div, prelude::*, px, rgb, uniform_list,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use tracewing_convert::Reading;
use tracewing_i18n::{tr, tr_n};

/// What a body view asks the window to open in a pane of its own.
pub enum ToOpen {
    /// A reading that holds a body, and the field or line it was read from.
    Reading(Reading, String),
    /// A line too long to be shown whole, by its number and its whole text.
    WholeLine { line: u64, text: String },
    /// A value a reader pressed, to be read in a pane of its own: the value
    /// as it stands, and the field it sat under.
    Value {
        text: String,
        field: Option<String>,
        /// Whether the reader asked outright -- a double press, or the menu
        /// -- which is answered even where the value reads as nothing.
        asked: bool,
    },
}

/// What opens one: a body view has no pane of its own, so it asks the window.
pub type Opens = Rc<dyn Fn(ToOpen, &mut gpui::App)>;

pub const LINE_HEIGHT: f32 = 20.;
/// How many lines' worth of "is this worth reading" to keep. A reader who has
/// scrolled past this many has not looked at the first of them.
const MARKS_MAX: usize = 4096;
/// How many lines' worth of what a rule reads to keep. The same reason, and
/// the same number: a reading is heavier than a `bool`, but `reading::small`
/// has already taken any body out of it, so it is a label and a line.
const RULED_MAX: usize = 4096;

const GUTTER_W: f32 = 52.;
/// Where an object or an array is folded and unfolded, after the number.
const FOLD_W: f32 = 20.;
/// The fold's arrow: twice the text, which made it too small to aim at.
const FOLD_TEXT: f32 = 24.;
const INDENT_W: f32 = 14.;
/// Past this the indent says what it is instead of pushing the text away.
const INDENT_MAX: u32 = 40;
const THUMB_MIN: f32 = 28.;
const BAR_W: f32 = crate::scrollbar::BAR_W;
/// The rounding of the well a body sits in, which the body reaches the bottom of.
const CORNER: f32 = 16.;
/// How often a window with a scan running looks at its progress.
const SCAN_POLL: Duration = Duration::from_millis(50);
/// Lines counted on either side of one whose rows are asked for.
const ROWS_READ_AROUND: u64 = 64;
/// Lines whose rows are kept counted, past which they are counted again.
const ROWS_KEPT: usize = 100_000;

/// An answer worked out once for a line and kept, with a bound on how many
/// are kept at a time: a body of a million lines asks as it scrolls.
///
/// The bound belongs to the type and not to the caller: this is the third
/// cache of this shape in the view, and `by_rule` went without one. It kept
/// a reading for every line a rule read, for as long as the rules stood, so
/// a body of hundreds of megabytes kept millions of them. Two callers out of
/// three remembering to check a constant is not a bound.
///
/// `max` must be well past what one draw inserts -- a screenful, or the
/// [`ROWS_READ_AROUND`] lines counted together -- or a draw throws away what
/// the next draw asks for and nothing is ever kept.
struct Kept<V> {
    kept: HashMap<u64, V>,
    max: usize,
}

impl<V> Kept<V> {
    fn new(max: usize) -> Self {
        Self {
            kept: HashMap::new(),
            max,
        }
    }

    fn get(&self, key: u64) -> Option<&V> {
        self.kept.get(&key)
    }

    /// Keeps `value` for `key`, never holding more than `max`. Past it the
    /// lot goes rather than the oldest: a reader who has scrolled that far
    /// is not looking at any of them, and choosing one costs more than
    /// working the next screenful out again.
    fn insert(&mut self, key: u64, value: V) {
        if self.kept.len() >= self.max {
            self.kept.clear();
        }
        self.kept.insert(key, value);
    }

    fn clear(&mut self) {
        self.kept.clear();
    }

    /// Only the test asks: the bound is kept inside `insert`, so no caller
    /// counts what is held.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.kept.len()
    }
}

/// Objects and arrays folded away, each from its first line to the line that
/// ends it. The first line stays in view; the rest, its closer too, do not.
/// None is inside another: folding the outer one lets the inner ones go.
#[derive(Debug, Default)]
pub struct Folds(BTreeMap<u64, u64>);

impl Folds {
    /// Rows in view for a body of `lines` lines.
    pub fn rows(&self, lines: u64) -> u64 {
        lines - self.0.iter().map(|(start, end)| end - start).sum::<u64>()
    }

    /// The line at `row`.
    pub fn line_of(&self, row: u64) -> u64 {
        let mut line = row;
        for (start, end) in &self.0 {
            if *start >= line {
                break;
            }
            line += end - start;
        }
        line
    }

    /// The row `line` is on, or the row of the fold that hides it.
    pub fn row_of(&self, line: u64) -> u64 {
        let mut row = line;
        for (start, end) in &self.0 {
            if *start >= line {
                break;
            }
            if line <= *end {
                return self.row_of(*start);
            }
            row -= end - start;
        }
        row
    }

    /// The line after `line` in view.
    pub fn next_line(&self, line: u64) -> u64 {
        self.0.get(&line).map_or(line + 1, |end| end + 1)
    }

    /// Where the fold from `line` ends, if it is folded.
    pub fn end_of(&self, line: u64) -> Option<u64> {
        self.0.get(&line).copied()
    }

    pub fn fold(&mut self, start: u64, end: u64) {
        let inside: Vec<u64> = self.0.range(start + 1..=end).map(|(s, _)| *s).collect();
        for line in inside {
            self.0.remove(&line);
        }
        self.0.insert(start, end);
    }

    pub fn unfold(&mut self, start: u64) -> bool {
        self.0.remove(&start).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Draws again while `body` is scanned, a frame only when there is something
/// new to show and none after. The scan stops if the view goes first.
fn watch_scan(body: Arc<Body>, cx: &mut Context<BodyView>) {
    cx.spawn(async move |this, cx| {
        while !body.is_scanned() {
            cx.background_executor().timer(SCAN_POLL).await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                body.stop();
                return;
            }
        }
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

/// What the gutter says of a line: nothing, that it opens an object or an
/// array, or that it is folded, with the closer's text and how many lines.
enum FoldMark {
    None,
    Opens,
    Folded(String, u64),
}

/// How a body is shown: laid out, as it was sent, or as its bytes. Only JSON
/// is laid out; any other text is shown as sent either way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyMode {
    #[default]
    Pretty,
    Raw,
    Hex,
    /// CSV and TSV as a table of cells.
    Table,
}

pub struct BodyView {
    body: Arc<Body>,
    /// The body read again as it was sent, once it is asked for.
    raw: Option<Arc<Body>>,
    /// The bytes as they came, when what is shown was made from them, as
    /// JSON is from MessagePack, and what they are called. Its dump is what
    /// Hex shows.
    original: Option<(Arc<Body>, SharedString)>,
    syntax: Syntax,
    /// The row at the top of the view, and how far it has scrolled out of it.
    top: u64,
    /// The same for text, which is wrapped: its rows are only known near
    /// where the view is.
    place: Place,
    /// Characters a wrapped row holds, from the width of the view.
    cols: usize,
    /// How many rows the lines read so far take at `cols`: a scroll asks of
    /// the same few lines again and again.
    row_counts: RefCell<Kept<usize>>,
    folds: Folds,
    mode: BodyMode,
    sub: f32,
    /// How much larger than the design the window draws: rows are this much
    /// taller, as the text in them is.
    scale: f32,
    dragging: bool,
    /// How far the lines are scrolled sideways, in pixels. Text that wraps
    /// never is.
    scroll_x: f32,
    /// The widest line drawn so far, indent and all, in pixels: how far the
    /// lines scroll sideways. It only grows, so scrolling down does not move
    /// what is in view sideways.
    widest: Cell<f32>,
    /// A character's width in the font the lines are drawn in, as last drawn.
    char_w: f32,
    /// The sideways thumb's drag: where the pointer and the lines were when
    /// it was pressed.
    dragging_x: Option<(f32, f32)>,
    /// Where the view was last drawn: the height sets the scroll range, and a drag
    /// on the bar is measured from its top.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    /// What is selected, by line and offset into what the line shows.
    select: Selection,
    /// The window's, which a press in the body takes so that its keys work.
    focus: FocusHandle,
    /// What the selection was read as, worked out when the right-click menu
    /// opened: a frame must not read the selection again on every draw.
    readings: Vec<Reading>,
    /// The readings the reader chose to see, by the piece each was read from,
    /// drawn after that piece's text. Emptied where the text changes under
    /// them, since a piece then means another line.
    drawn: BTreeMap<u64, Reading>,
    /// How fields are always read, shared with the inspector so that a rule
    /// remembered in either is kept by both.
    rules: Rc<RefCell<Remembered>>,
    /// The host this body came from, which rules are remembered against.
    host: SharedString,
    /// Whether values with a reading behind them are underlined, as Settings
    /// says. Shared with the inspector, so a change takes hold at once.
    marks_on: reading::MarksOn,
    /// What those rules read, by piece, and the rules' version they were read
    /// at: a frame must not read every line again on every draw. [`Kept`]
    /// bounds it: a million lines a rule reads would otherwise be a million
    /// readings held for as long as the rule stood.
    by_rule: RefCell<(u64, Kept<Option<Reading>>)>,
    /// Which pieces hold a value worth saying something about, by piece. The
    /// same reason, and the same answer kept.
    marks: RefCell<Kept<bool>>,
    /// The line the right-click menu was opened over, which `Open this line`
    /// opens. A line wider than the panel is cut by the panel, not by the
    /// reading, so it carries no note saying there is more.
    menu_line: Option<u64>,
    /// What to call to open a reading that holds a body of its own. A body
    /// view has no pane to put one in; the window does.
    opens: Option<Opens>,
    /// CSV or TSV read as a table, which [`BodyMode::Table`] shows.
    table: Option<Arc<Table>>,
    table_scroll: UniformListScrollHandle,
    /// The table's scroll bar, on the list's own handle.
    table_bar: Scrolled,
    /// The cell a right click was on, as row and column: what Copy row and
    /// Copy column take.
    table_menu_cell: Option<(usize, usize)>,
    /// A character's width in the table, at the design's size: measured in
    /// the code font as it draws.
    table_char_w: f32,
    /// What Find looks for here and has found, if it is open.
    find: Option<find::Found>,
    /// Which search is the latest: an older one still running stops.
    find_gen: u64,
}

impl BodyView {
    /// `content_type` is the body's, which says how to color it.
    pub fn new(
        body: Arc<Body>,
        content_type: Option<&str>,
        focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let syntax = Syntax::of(body.kind(), content_type, body.head());
        let table_scroll = UniformListScrollHandle::new();
        let table_bar = Scrolled::of(table_scroll.0.borrow().base_handle.clone());
        watch_scan(Arc::clone(&body), cx);
        Self {
            body,
            raw: None,
            original: None,
            syntax,
            top: 0,
            scale: 1.,
            place: Place::default(),
            cols: 80,
            row_counts: RefCell::new(Kept::new(ROWS_KEPT)),
            folds: Folds::default(),
            mode: BodyMode::Pretty,
            sub: 0.,
            dragging: false,
            scroll_x: 0.,
            widest: Cell::new(0.),
            char_w: 7.2,
            dragging_x: None,
            bounds: Rc::new(Cell::new(Bounds::default())),
            select: Selection::default(),
            focus,
            readings: Vec::new(),
            drawn: BTreeMap::new(),
            rules: Rc::new(RefCell::new(Remembered::default())),
            host: SharedString::default(),
            marks_on: reading::MarksOn::default(),
            by_rule: RefCell::new((0, Kept::new(RULED_MAX))),
            marks: RefCell::new(Kept::new(MARKS_MAX)),
            menu_line: None,
            opens: None,
            table: None,
            table_scroll,
            table_bar,
            table_menu_cell: None,
            table_char_w: 7.4,
            find: None,
            find_gen: 0,
        }
    }

    /// The body read as a table too, which it can then be shown as.
    pub fn set_table(&mut self, table: Arc<Table>, cx: &mut Context<Self>) {
        self.table = Some(table);
        cx.notify();
    }

    pub fn has_table(&self) -> bool {
        self.table.is_some()
    }

    pub fn table(&self) -> Option<&Arc<Table>> {
        self.table.as_ref()
    }

    /// How far the lines are scrolled sideways, in pixels.
    pub fn scroll_x(&self) -> f32 {
        self.scroll_x
    }

    /// Where the lines were last drawn, in the window.
    pub fn drawn_bounds(&self) -> Bounds<Pixels> {
        self.bounds.get()
    }

    pub fn selection(&self) -> &Selection {
        &self.select
    }

    /// The selected text, as [`Body::text`] gives it.
    pub fn selected_text(&self) -> Copied {
        match self.select.bounds() {
            Some((start, end)) if self.is_table() => self.table_text(start, end),
            Some((start, end)) => self.copy(start, end),
            None => Copied::Nothing,
        }
    }

    fn copy(&self, start: Spot, end: Spot) -> Copied {
        match self.shown().text_in(
            self.line_spot(start),
            self.line_spot(end),
            COPY_MAX,
            self.hex(),
        ) {
            Some(text) if text.is_empty() => Copied::Nothing,
            Some(text) => Copied::Text(text),
            None => Copied::TooLarge,
        }
    }

    /// The key the selection sits under, for the converters: the `created_at`
    /// of `"created_at": 1763000000`. A body is indexed by line and carries no
    /// path, so the name comes off the line itself.
    /// Takes the whole value under `point` and asks the window to read it
    /// beside the body. Nothing happens where the press was not in a value.
    fn read_value_at(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
        asked: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(spot) = self.select.spot_at(point) {
            self.read_value_at_spot(spot, asked, cx);
        }
    }

    /// The same from a spot rather than a point, which is what a press
    /// becomes and what a test can hand over.
    pub fn read_value_at_spot(&mut self, spot: Spot, asked: bool, cx: &mut Context<Self>) -> bool {
        // Asked for outright, the word under the press will do where it is
        // not in a value of its own: somebody pressing twice on a line of
        // plain text means that word.
        let found = self
            .value_at(spot)
            .or_else(|| asked.then(|| self.word_at(spot)).flatten());
        let Some((range, value, field)) = found else {
            return false;
        };
        self.select.select(
            Spot {
                piece: spot.piece,
                at: range.start,
            },
            Spot {
                piece: spot.piece,
                at: range.end,
            },
        );
        if let Some(opens) = &self.opens {
            let opens = opens.clone();
            let what = ToOpen::Value {
                text: value,
                field,
                asked,
            };
            cx.defer(move |cx| opens(what, cx));
        }
        cx.notify();
        true
    }

    /// The word under a spot, where it is not in a value of its own: what a
    /// reader who pressed twice on a line of plain text means.
    fn word_at(&self, spot: Spot) -> Option<(std::ops::Range<usize>, String, Option<String>)> {
        let (line, at) = self.line_spot(spot);
        let text = self.line_text(line)?;
        let range = crate::input::word_around(&text, at.min(text.len()));
        let word = text.get(range.clone())?.trim_matches('"').to_owned();
        (!word.trim().is_empty()).then_some((range, word, None))
    }

    /// The whole value the press landed in, and the field it sits under:
    /// `created_at` and `1794492800` of `  "created_at": 1794492800,`.
    ///
    /// A reader should not have to drag across a value exactly to have it
    /// read -- and dragging it exactly is work, since a word ends at every
    /// dot of a JWT and every dash of a date. The view already knows where
    /// the value is, since that is what it underlines.
    /// The line as it is drawn, which is what a press landed on.
    ///
    /// In hex that is the dump, not the file's line. `read_whole` gives back
    /// the file's line: a different string, of a different length, numbered
    /// in a different space -- a hex line is sixteen bytes and a text line is
    /// as long as it is. An offset taken from the dump lands anywhere at all
    /// in it, so a word read off it is a word nobody pressed, and the range
    /// it comes back with marks bytes nobody chose.
    fn line_text(&self, line: u64) -> Option<String> {
        match self.hex() {
            true => Some(self.shown().read_in(line..line + 1, true).pop()?.text),
            false => self.shown().read_whole(line..line + 1).pop(),
        }
    }

    fn value_at(&self, spot: Spot) -> Option<(std::ops::Range<usize>, String, Option<String>)> {
        if self.hex() {
            return None;
        }
        let (line, at) = self.line_spot(spot);
        let text = self.shown().read_whole(line..line + 1).pop()?;
        let (field, value) = reading::field_and_value(&text)?;
        let start = (value.as_ptr() as usize).checked_sub(text.as_ptr() as usize)?;
        let range = start..start + value.len();
        range
            .contains(&at)
            .then(|| (range.clone(), value.to_owned(), Some(field.to_owned())))
    }

    fn selected_field(&self) -> Option<String> {
        // A dump has no fields. Its text column holds whatever the bytes
        // spell, quotes and colons included, so the search below would find
        // `"session":` in a row of hex and call it the name of the value.
        if self.hex() {
            return None;
        }
        let (start, _) = self.select.bounds()?;
        let (line, at) = self.line_spot(start);
        let text = self.shown().read_whole(line..line + 1).pop()?;
        let before = &text[..select::floor(&text, at.min(text.len()))];
        let end = before.rfind("\":")?;
        let open = before[..end].rfind('"')?;
        Some(before[open + 1..end].to_owned())
    }

    /// The first and the last spot of what is shown: the body's text, or its
    /// table.
    fn ends(&self) -> Option<(Spot, Spot)> {
        if self.is_table() {
            self.table_ends()
        } else {
            self.text_ends()
        }
    }

    /// The first and the last spot of the body's text.
    fn text_ends(&self) -> Option<(Spot, Spot)> {
        let last = self.lines().checked_sub(1)?;
        let piece = if self.wrapping() {
            wrap::key(last, wrap::LAST_ROW)
        } else {
            last
        };
        Some((
            Spot::default(),
            Spot {
                piece,
                at: usize::MAX,
            },
        ))
    }

    /// The line a spot is in, and its offset into what the line shows. A
    /// wrapped row's spot is first made its line's.
    fn line_spot(&self, spot: Spot) -> (u64, usize) {
        if !self.wrapping() {
            return (spot.piece, spot.at);
        }
        let (line, row) = wrap::unkey(spot.piece);
        let base = self
            .shown()
            .read_whole(line..line + 1)
            .first()
            .map_or(0, |text| {
                wrap::row_starts(text, self.cols)
                    .get(row)
                    .copied()
                    .unwrap_or(text.len())
            });
        (line, base.saturating_add(spot.at))
    }

    /// Text is wrapped to the width of the view. JSON laid out, which is
    /// folded instead, and dumps are not.
    fn wrapping(&self) -> bool {
        !self.hex() && self.shown().kind() == Kind::Text
    }

    /// How many rows `line` takes, wrapped. Lines around it are counted
    /// with it, as they are read in one go and a scroll goes there next.
    fn rows_of(&self, line: u64) -> usize {
        if let Some(rows) = self.row_counts.borrow().get(line) {
            return *rows;
        }
        let body = self.shown();
        let lines = self.lines();
        let from = line.saturating_sub(ROWS_READ_AROUND);
        let to = (line + ROWS_READ_AROUND).min(lines);
        let mut counts = self.row_counts.borrow_mut();
        let mut found = 1;
        for (at, text) in (from..).zip(body.read_whole(from..to)) {
            let rows = wrap::row_starts(&text, self.cols).len();
            // The last line may still grow while the scan reads it.
            if at + 1 < lines || body.is_scanned() {
                counts.insert(at, rows);
            }
            if at == line {
                found = rows;
            }
        }
        found
    }

    /// The rows a view holds whole.
    fn rows_in_height(&self) -> u64 {
        (self.height() / self.line_h()).floor().max(1.) as u64
    }

    /// The highest place wrapped text can be shown from.
    fn last_place(&self) -> Place {
        Place::last_top(self.lines(), self.rows_in_height(), &mut |line| {
            self.rows_of(line)
        })
    }

    /// Scrolls wrapped text by `dy` pixels, a row at a time, no further than
    /// a full view from its end.
    fn scroll_wrapped(&mut self, dy: f64) {
        let lines = self.lines();
        let last = self.last_place();
        let line_height = f64::from(self.line_h());
        let mut place = self.place;
        let mut y = f64::from(self.sub) + dy;
        {
            let mut rows = |line| self.rows_of(line);
            while y >= line_height && place < last {
                let Some(next) = place.next(lines, &mut rows) else {
                    break;
                };
                place = next;
                y -= line_height;
            }
            while y < 0. {
                let Some(before) = place.back(&mut rows) else {
                    y = 0.;
                    break;
                };
                place = before;
                y += line_height;
            }
        }
        if place >= last {
            place = last;
            y = 0.;
        }
        self.place = place;
        self.sub = y.clamp(0., line_height - 1.) as f32;
    }

    /// Scrolls as the wheel does. For tests.
    #[doc(hidden)]
    pub fn scroll_by(&mut self, dy: f64, cx: &mut Context<Self>) {
        if self.wrapping() {
            self.scroll_wrapped(dy);
        } else {
            self.set_position(self.position() + dy);
        }
        cx.notify();
    }

    /// The line and the row of it at the top of wrapped text. For tests.
    #[doc(hidden)]
    pub fn place(&self) -> (u64, usize) {
        (self.place.line, self.place.row)
    }

    /// How far down wrapped text the view is, from 0 to 1.
    fn wrapped_share(&self) -> f64 {
        let mut rows = |line| self.rows_of(line);
        let max = self.last_place().fraction(&mut rows);
        if max <= 0. {
            0.
        } else {
            (self.place.fraction(&mut rows) / max).clamp(0., 1.)
        }
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        if let Some((start, end)) = self.ends() {
            self.select.select(start, end);
            cx.notify();
        }
    }

    /// Shuts the right-click menu, and forgets the readings it offered.
    fn close_menu(&mut self) {
        self.select.set_menu(None);
        self.readings = Vec::new();
        self.menu_line = None;
    }

    /// The selection read, as far as it is worth reading: past
    /// [`tracewing_convert::MAX_INPUT`] a selection is something to copy, not
    /// a value to make sense of.
    pub fn read_selection(&self) -> Vec<Reading> {
        let Some((start, end)) = self.select.bounds() else {
            return Vec::new();
        };
        let Some(text) = self.shown().text_in(
            self.line_spot(start),
            self.line_spot(end),
            tracewing_convert::MAX_INPUT,
            self.hex(),
        ) else {
            return Vec::new();
        };
        reading::of(&text, self.selected_field().as_deref())
    }

    /// Draws `reading` after the piece the selection starts in, which is what
    /// choosing it from the menu does.
    pub fn draw_reading(&mut self, reading: Reading, cx: &mut Context<Self>) {
        if let Some((start, _)) = self.select.bounds() {
            self.drawn.insert(start.piece, reading);
        }
        cx.notify();
    }

    /// The rules to read this body's fields by, and the host they are
    /// remembered against. Given by the inspector as the view is made.
    pub fn set_rules(
        &mut self,
        rules: Rc<RefCell<Remembered>>,
        host: impl Into<SharedString>,
        marks_on: reading::MarksOn,
    ) {
        self.rules = rules;
        self.host = host.into();
        self.marks_on = marks_on;
        self.by_rule.borrow_mut().1.clear();
    }

    /// What to call when a reading holds a body of its own.
    pub fn on_open_reading(&mut self, opens: Opens) {
        self.opens = Some(opens);
    }

    /// The rules, to remember one or to forget one from the menu.
    pub fn rules(&self) -> &Rc<RefCell<Remembered>> {
        &self.rules
    }

    /// Where the value on this line is worth underlining, so that a reader
    /// knows there is something here to read. Worked out once for each piece
    /// and kept, because a body of a million lines asks as it scrolls; past
    /// what a reader could have looked at, the answers are dropped.
    fn marked(&self, key: u64, line: &str) -> Option<std::ops::Range<usize>> {
        if !self.marks_on.get() {
            return None;
        }
        let (field, value) = reading::field_and_value(line)?;
        let at = (value.as_ptr() as usize).checked_sub(line.as_ptr() as usize)?;
        let known = self.marks.borrow().get(key).copied();
        let worth = match known {
            Some(worth) => worth,
            None => {
                let worth = reading::worth_marking(value, Some(field));
                self.marks.borrow_mut().insert(key, worth);
                worth
            }
        };
        worth.then_some(at..at + value.len())
    }

    /// What is drawn beside the piece `key` of a line that reads `line`: the
    /// reading chosen for it, else the one a remembered rule asks for.
    pub fn reading_at(&self, key: u64, line: &str) -> Option<Reading> {
        self.drawn
            .get(&key)
            .cloned()
            .or_else(|| self.ruled(key, line))
    }

    /// What a remembered rule reads of this line, worked out once for each
    /// line and kept until the rules change or [`RULED_MAX`] are held.
    fn ruled(&self, key: u64, line: &str) -> Option<Reading> {
        let (id, version) = {
            let rules = self.rules.borrow();
            if rules.is_empty() {
                return None;
            }
            let (field, _) = reading::field_and_value(line)?;
            (
                rules.reading_for(&self.host, field)?.to_owned(),
                rules.version(),
            )
        };
        {
            let mut kept = self.by_rule.borrow_mut();
            if kept.0 != version {
                kept.0 = version;
                kept.1.clear();
            } else if let Some(found) = kept.1.get(key) {
                return found.clone();
            }
        }
        let (field, value) = reading::field_and_value(line)?;
        let found = reading::read_as(&id, value, Some(field));
        self.by_rule.borrow_mut().1.insert(key, found.clone());
        found
    }

    /// Draws the next reading of the selection, after the one drawn there,
    /// and round again to none: what the keyboard does instead of the menu.
    pub fn cycle_reading(&mut self, cx: &mut Context<Self>) -> bool {
        let Some((start, _)) = self.select.bounds() else {
            return false;
        };
        let readings = self.read_selection();
        if readings.is_empty() {
            return false;
        }
        let now = self.drawn.get(&start.piece).map(|reading| reading.id);
        let next = match now {
            None => readings.first().cloned(),
            Some(id) => match readings.iter().position(|found| found.id == id) {
                Some(at) => readings.get(at + 1).cloned(),
                None => readings.first().cloned(),
            },
        };
        match next {
            Some(reading) => self.draw_reading(reading, cx),
            None => {
                self.drawn.remove(&start.piece);
                cx.notify();
            }
        }
        true
    }

    /// Opens the whole of a line the view had to cut, in a pane of its own.
    /// False where there is nowhere to open it or nothing to open.
    pub fn open_whole_line(&mut self, line: u64, cx: &mut gpui::App) -> bool {
        let Some(opens) = self.opens.clone() else {
            return false;
        };
        let Some(text) = self.shown().read_whole(line..line + 1).pop() else {
            return false;
        };
        opens(ToOpen::WholeLine { line, text }, cx);
        true
    }

    /// Takes away everything that was drawn beside a value.
    pub fn clear_readings(&mut self, cx: &mut Context<Self>) {
        self.drawn.clear();
        cx.notify();
    }

    /// How many readings are drawn.
    pub fn readings_drawn(&self) -> usize {
        self.drawn.len()
    }

    fn copy_all(&mut self, cx: &mut Context<Self>) {
        self.close_menu();
        // The body as it came, a table's too.
        if let Some((start, end)) = self.text_ends()
            && let Copied::Text(text) = self.copy(start, end)
        {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        cx.notify();
    }

    fn copy_selected(&mut self, cx: &mut Context<Self>) {
        self.close_menu();
        if let Copied::Text(text) = self.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        cx.notify();
    }

    /// The end of a selection dragged to `at`. Past the top or the bottom of
    /// the view, the view scrolls toward it as far as the mouse is past.
    fn drag_selection(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let bounds = self.bounds.get();
        let past = if at.y < bounds.top() {
            at.y - bounds.top()
        } else if at.y > bounds.bottom() {
            at.y - bounds.bottom()
        } else {
            px(0.)
        };
        if past != px(0.) {
            let past = f64::from(f32::from(past));
            if self.wrapping() {
                self.scroll_wrapped(past);
            } else {
                self.set_position(self.position() + past);
            }
        }
        if self.select.drag_to(at) || past != px(0.) {
            cx.notify();
        }
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    /// The line at the top of the view.
    pub fn top(&self) -> u64 {
        if self.wrapping() {
            self.place.line
        } else {
            self.folds.line_of(self.top)
        }
    }

    pub fn folds(&self) -> &Folds {
        &self.folds
    }

    /// Folds the object or array `line` opens, or unfolds it. The line at the
    /// top of the view stays there, or the fold that now hides it does.
    pub fn toggle_fold(&mut self, line: u64, cx: &mut Context<Self>) {
        let top_line = self.folds.line_of(self.top);
        if !self.folds.unfold(line) {
            // Not found yet when the scan has not reached it.
            let Some(end) = self.shown().closer(line) else {
                return;
            };
            self.folds.fold(line, end);
        }
        self.top = self.folds.row_of(top_line);
        self.set_position(self.position());
        cx.notify();
    }

    /// Every fold opened again.
    pub fn unfold_all(&mut self, cx: &mut Context<Self>) {
        let top_line = self.folds.line_of(self.top);
        self.folds = Folds::default();
        self.top = top_line;
        cx.notify();
    }

    /// Lines as shown: the body's, or its dump's.
    fn lines(&self) -> u64 {
        self.shown().lines_in(self.hex())
    }

    /// The body as read for the mode: as sent, or as it was told to be.
    fn shown(&self) -> &Arc<Body> {
        match (self.mode, &self.raw, &self.original) {
            (BodyMode::Raw, Some(raw), _) => raw,
            (BodyMode::Hex, _, Some((original, _))) => original,
            _ => &self.body,
        }
    }

    /// Shows JSON made from `original`, whose bytes Hex shows, and which is
    /// called `name`. Bytes as text would say nothing, so there is no Raw.
    pub fn set_original(
        &mut self,
        original: Arc<Body>,
        name: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let name = name.into();
        watch_scan(Arc::clone(&original), cx);
        self.original = Some((original, name));
        self.mode = self.fit(self.mode);
        cx.notify();
    }

    /// Whether the body can be shown as sent, as text.
    pub fn has_raw(&self) -> bool {
        self.original.is_none()
    }

    /// `mode`, or the nearest this body has.
    fn fit(&self, mode: BodyMode) -> BodyMode {
        match mode {
            BodyMode::Table if self.table.is_none() => self.fit(BodyMode::Pretty),
            BodyMode::Raw if !self.has_raw() => BodyMode::Pretty,
            // Made from bytes, what is shown is never as sent, laid out or not.
            BodyMode::Pretty if !self.has_pretty() && self.has_raw() => BodyMode::Raw,
            mode => mode,
        }
    }

    fn hex(&self) -> bool {
        self.mode == BodyMode::Hex
    }

    /// Whether the body as shown is read to its end.
    pub fn is_scanned(&self) -> bool {
        self.shown().is_scanned()
    }

    pub fn is_hex(&self) -> bool {
        self.hex()
    }

    /// Whether the body can be laid out, as JSON and XML can.
    pub fn has_pretty(&self) -> bool {
        self.body.kind().is_laid_out()
    }

    /// The mode in use. Text that cannot be laid out is as sent.
    pub fn mode(&self) -> BodyMode {
        self.fit(self.mode)
    }

    /// Shows the body laid out, as sent, or as a dump of its bytes. Folds,
    /// what is selected and the place in it belong to the other, so they go.
    pub fn set_mode(&mut self, mode: BodyMode, cx: &mut Context<Self>) {
        let mode = self.fit(mode);
        if self.mode() == mode {
            return;
        }
        if mode == BodyMode::Raw && self.has_pretty() && self.raw.is_none() {
            // Read again on its own, as the body was the first time.
            let Ok(raw) = self.body.as_sent() else {
                return;
            };
            watch_scan(Arc::clone(&raw), cx);
            self.raw = Some(raw);
        }
        self.mode = mode;
        self.scroll_x = 0.;
        self.widest.set(0.);
        self.drawn.clear();
        self.by_rule.borrow_mut().1.clear();
        self.marks.borrow_mut().clear();
        self.place = Place::default();
        self.row_counts.borrow_mut().clear();
        self.folds = Folds::default();
        self.select.clear();
        self.top = 0;
        self.sub = 0.;
        // What Find found was in the other mode's text.
        self.find_again(cx);
        cx.notify();
    }

    fn rows(&self) -> u64 {
        self.folds.rows(self.lines())
    }

    /// A row's height as drawn now.
    fn line_h(&self) -> f32 {
        LINE_HEIGHT * self.scale
    }

    fn height(&self) -> f32 {
        f32::from(self.bounds.get().size.height)
    }

    /// The largest position with a full view below it, in pixels.
    pub fn max_position(&self) -> f64 {
        (self.rows() as f64 * f64::from(self.line_h()) - f64::from(self.height())).max(0.)
    }

    /// Moves the top of the view to `y` pixels into the content, clamped.
    pub fn set_position(&mut self, y: f64) {
        let y = y.clamp(0., self.max_position());
        let line_height = f64::from(self.line_h());
        self.top = (y / line_height) as u64;
        self.sub = (y - self.top as f64 * line_height) as f32;
    }

    fn position(&self) -> f64 {
        self.top as f64 * f64::from(self.line_h()) + f64::from(self.sub)
    }

    fn on_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let moved = event.delta.pixel_delta(px(self.line_h()));
        let (mut across, mut down) = (f32::from(moved.x), f32::from(moved.y));
        // Shift turns a wheel that only goes up and down sideways.
        if event.modifiers.shift && across == 0. {
            (across, down) = (down, 0.);
        }
        if across != 0. && !self.wrapping() {
            self.scroll_x = (self.scroll_x - across).clamp(0., self.max_x());
        }
        if down != 0. {
            let delta = f64::from(down);
            if self.wrapping() {
                self.scroll_wrapped(-delta);
            } else {
                self.set_position(self.position() - delta);
            }
        }
        cx.notify();
    }

    /// How wide the lines' text shows, between the gutter and the bar.
    fn text_w(&self) -> f32 {
        f32::from(self.bounds.get().size.width) - BAR_W - (GUTTER_W + FOLD_W) * self.scale
    }

    /// How far the lines scroll sideways: past the widest, a little room.
    fn max_x(&self) -> f32 {
        if self.wrapping() {
            return 0.;
        }
        (self.widest.get() + 2. * self.char_w - self.text_w()).max(0.)
    }

    /// The sideways thumb dragged to `x`, or pressed there.
    /// Where the sideways bar starts, from the pane's left, and how long it
    /// is: past the rounded corner, up to the bar that goes up and down.
    fn across_track(&self) -> (f32, f32) {
        let start = CORNER * self.scale;
        (
            start,
            (f32::from(self.bounds.get().size.width) - BAR_W - start).max(1.),
        )
    }

    fn drag_x_to(&mut self, x: f32, cx: &mut Context<Self>) {
        let Some((x0, from)) = self.dragging_x else {
            return;
        };
        let (_, view) = self.across_track();
        if let Some((long, _)) = crate::scrollbar::thumb(view, self.max_x(), self.scroll_x) {
            let per = self.max_x() / (view - long).max(1.);
            self.scroll_x = (from + (x - x0) * per).clamp(0., self.max_x());
            cx.notify();
        }
    }

    fn drag_to(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let track = f64::from(self.height() - THUMB_MIN).max(1.);
        let inside = f32::from(y - self.bounds.get().origin.y);
        let at = ((f64::from(inside) - f64::from(THUMB_MIN) / 2.) / track).clamp(0., 1.);
        if self.wrapping() {
            let lines = self.lines();
            let mut rows = |line| self.rows_of(line);
            let max = self.last_place().fraction(&mut rows);
            let place = Place::at_fraction(at * max, lines, &mut rows);
            self.place = place;
            self.sub = 0.;
        } else {
            self.set_position(at * self.max_position());
        }
        cx.notify();
    }

    /// As many characters a row as the view is wide. The place in view stays
    /// on its line when the width changes.
    fn fit_cols(&mut self, window: &mut Window) {
        let char_w = mono_char_w(window, 12. * self.scale);
        let width =
            f32::from(self.bounds.get().size.width) - BAR_W - (GUTTER_W + FOLD_W + 8.) * self.scale;
        let cols = ((width / char_w).floor() as usize).max(8);
        if cols != self.cols {
            self.cols = cols;
            self.row_counts.borrow_mut().clear();
            self.place.row = self.place.row.min(self.rows_of(self.place.line) - 1);
        }
    }

    /// A line, or with text wrapped one row of it: `key` tells it from the
    /// rest for selecting, and only a line's first row has its number.
    /// `from` is where `shown` starts in its line: past 0 for a wrapped row.
    #[allow(clippy::too_many_arguments)]
    fn render_line(
        &self,
        line: u64,
        key: u64,
        number: Option<u64>,
        shown: Line,
        from: usize,
        mark: FoldMark,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let indent = if shown.depth > INDENT_MAX {
            INDENT_MAX as f32 * INDENT_W
        } else {
            shown.depth as f32 * INDENT_W
        };
        let number = number.map(|n| n.to_string()).unwrap_or_default();
        // What the reader chose to see here, or what a rule says this field
        // is always read as.
        let note = self.reading_at(key, &shown.text);
        let mut highlights: Vec<_> = if self.hex() {
            Vec::new()
        } else {
            self.syntax
                .runs(&shown.text)
                .into_iter()
                .map(|(range, token)| (range, highlight(token)))
                .collect()
        };
        // A value with something to say about it is underlined, dimly, until
        // it says it: then the reading itself stands beside it instead.
        if note.is_none()
            && !self.hex()
            && let Some(at) = self.marked(key, &shown.text)
        {
            highlights = combine_highlights(highlights, [(at, reading::underlined())]).collect();
        }
        // What Find found here, the one it is at brighter.
        for (range, now) in self.found_in(line, from, &shown.text) {
            highlights = combine_highlights(highlights, [(range, found_style(now))]).collect();
        }
        // Hex says each byte twice, as two digits and as the character it
        // stands for. What is chosen on one side is marked on the other, so
        // that a byte and its character are read together.
        if self.hex()
            && from == 0
            && let Some(chosen) = self.select.range_in(key, shown.text.len())
            && let Some(twin) = crate::body::hex_twin(&chosen, &shown.text)
        {
            let echoed = HighlightStyle {
                background_color: Some(rgb(colors().echoed).into()),
                ..Default::default()
            };
            highlights = combine_highlights(highlights, [(twin, echoed)]).collect();
        }
        let text: SharedString = shown.text.into();
        let folded = matches!(mark, FoldMark::Folded(..));
        let mut row = div()
            .h(ui(LINE_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .whitespace_nowrap()
            .overflow_hidden()
            .child(
                div()
                    .id(("number", key))
                    .w(ui(GUTTER_W))
                    .flex_shrink_0()
                    .pr_1()
                    .flex()
                    .justify_end()
                    .text_color(rgb(colors().faint))
                    .when(self.opens.is_some() && !number.is_empty(), |d| {
                        d.cursor_pointer()
                            .hover(|s| s.text_color(rgb(colors().link)))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_whole_line(line, cx);
                            }))
                    })
                    .child(number),
            )
            .child({
                let toggle = div()
                    .id(("fold", line))
                    .w(ui(FOLD_W))
                    .h(ui(LINE_HEIGHT))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(ui(FOLD_TEXT))
                    .line_height(ui(LINE_HEIGHT))
                    .text_color(rgb(colors().faint));
                match &mark {
                    FoldMark::None => toggle,
                    FoldMark::Opens | FoldMark::Folded(..) => toggle
                        .cursor_pointer()
                        .hover(|d| d.text_color(rgb(colors().text)))
                        .child(if matches!(mark, FoldMark::Opens) {
                            "▾"
                        } else {
                            "▸"
                        })
                        // A press here folds; it does not start a selection.
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_fold(line, cx))),
                }
            })
            .child({
                // How wide the line is, for how far the lines scroll sideways.
                let wide = indent * self.scale
                    + unicode_width::UnicodeWidthStr::width(text.as_ref()) as f32 * self.char_w;
                if wide > self.widest.get() {
                    self.widest.set(wide);
                }
                // The note below stays in view however long the text is. A fold's
                // note follows the text, as the closer it stands for would. The
                // text, indent and all, is moved sideways by laying it out further
                // left, so that a press still finds the character under it.
                div()
                    .when(!folded, |d| d.flex_1())
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .ml(px(-self.scroll_x))
                            .child(div().w(ui(indent)).flex_shrink_0())
                            .child(self.select.piece_at(key, text, highlights)),
                    )
            });
        if let FoldMark::Folded(closer, hidden) = mark {
            row = row.child(
                div()
                    .id(("folded", line))
                    .flex_shrink_0()
                    .ml_1()
                    .px_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(rgb(colors().chip))
                    .text_color(rgb(colors().muted))
                    .child(tr_n!(
                        hidden,
                        "… {count} line {closer}",
                        "… {count} lines {closer}",
                        count = crate::flows::group_digits(hidden),
                        closer = closer
                    ))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_fold(line, cx))),
            );
        }
        if shown.cut > 0 {
            // The rest of the line is not lost, only not drawn: a click opens
            // the whole of it in a pane, where text wraps.
            let opens = self.opens.clone();
            row = row.child(
                div()
                    .id(("cut", key))
                    .flex_shrink_0()
                    .pl_2()
                    .text_color(rgb(colors().faint))
                    .when(opens.is_some(), |d| {
                        d.cursor_pointer()
                            .hover(|s| s.text_color(rgb(colors().link)))
                    })
                    .child(tr!("… {size} more", size = size_text(shown.cut)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_whole_line(line, cx);
                    })),
            );
        }
        // What the reader asked to see of this piece: the time a number is,
        // the claims a token makes. A click on it takes it away again.
        if let Some(reading) = note {
            // A reading chosen here is taken away with a click. One a rule
            // asks for is not: it goes when the rule is forgotten.
            let chosen = self.drawn.contains_key(&key);
            row = row.child(
                div()
                    .id(("reading", key))
                    .flex_shrink_0()
                    .ml_1()
                    .px_1()
                    .rounded_sm()
                    .bg(rgb(colors().chip))
                    .text_color(rgb(colors().muted))
                    .child(reading.beside())
                    .when(chosen, |d| {
                        d.cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.drawn.remove(&key);
                                cx.notify();
                            }))
                    }),
            );
        }
        row
    }
}

impl BodyView {
    /// The body's size and lines, as shown.
    pub fn summary(&self) -> String {
        match &self.original {
            // The size is what was sent; the lines are the JSON's.
            Some((original, name)) if !self.hex() && self.body.is_scanned() => {
                let lines = self.body.lines();
                tr_n!(
                    lines,
                    "{size} {format} · {count} line of JSON",
                    "{size} {format} · {count} lines of JSON",
                    size = size_text(original.size()),
                    format = name.to_string(),
                    count = crate::flows::group_digits(lines)
                )
            }
            _ => summary_in(self.shown(), self.hex()),
        }
    }

    /// Wrapped text's rows from the top of the view, read a run of lines at
    /// a time.
    fn wrapped_rows_in_view(&self, in_view: u64, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let lines = self.lines();
        let mut shown = Vec::new();
        let mut line = self.place.line;
        let mut first_row = self.place.row;
        while line < lines && (shown.len() as u64) < in_view {
            let run = (in_view - shown.len() as u64).min(lines - line);
            // Lines cut from a longer one go on without a number of their own.
            let numbers = self.shown().numbers(line..line + run);
            for (text, number) in self
                .shown()
                .read_whole(line..line + run)
                .into_iter()
                .zip(numbers)
            {
                let starts = wrap::row_starts(&text, self.cols);
                for (row, from) in starts.iter().copied().enumerate().skip(first_row) {
                    if shown.len() as u64 >= in_view {
                        break;
                    }
                    let to = starts.get(row + 1).copied().unwrap_or(text.len());
                    let part = Line {
                        text: text[from..to].to_owned(),
                        depth: 0,
                        cut: 0,
                    };
                    shown.push(
                        self.render_line(
                            line,
                            wrap::key(line, row),
                            number.filter(|_| row == 0),
                            part,
                            from,
                            FoldMark::None,
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                first_row = 0;
                line += 1;
            }
        }
        shown
    }

    /// The rows from the top of the view, read a run of lines at a time: a
    /// fold is where one run ends and the next begins.
    fn rows_in_view(&self, in_view: u64, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let lines = self.lines();
        let mut shown = Vec::new();
        let mut line = self.folds.line_of(self.top);
        while line < lines && (shown.len() as u64) < in_view {
            let start = line;
            let mut end = start;
            while end + 1 < lines
                && self.folds.end_of(end).is_none()
                && end + 1 - start < in_view - shown.len() as u64
            {
                end += 1;
            }
            // One more, to tell whether the run's last line opens something.
            let read = self
                .shown()
                .read_in(start..(end + 2).min(lines), self.hex());
            let next_depths: Vec<Option<u32>> = read
                .iter()
                .skip(1)
                .map(|l| Some(l.depth))
                .chain(std::iter::once(None))
                .collect();
            for (i, text) in read
                .into_iter()
                .take((end - start + 1) as usize)
                .enumerate()
            {
                let at = start + i as u64;
                let mark = match self.folds.end_of(at) {
                    Some(closer) => {
                        let closed = self
                            .shown()
                            .read(closer..closer + 1)
                            .pop()
                            .map(|l| l.text)
                            .unwrap_or_default();
                        FoldMark::Folded(closed, closer - at)
                    }
                    None if next_depths[i] == Some(text.depth + 1) => FoldMark::Opens,
                    None => FoldMark::None,
                };
                shown.push(
                    self.render_line(at, at, Some(at + 1), text, 0, mark, cx)
                        .into_any_element(),
                );
            }
            line = self.folds.next_line(end);
        }
        shown
    }
}

impl Render for BodyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.table.clone() {
            Some(table) if self.mode() == BodyMode::Table => {
                // As wide as the code font's letters, whichever it is.
                let scale = crate::text_size::scale(window);
                self.scale = scale;
                self.table_char_w = mono_char_w(window, 12. * scale) / scale;
                self.render_table(table, cx).into_any_element()
            }
            _ => self.render_text(window, cx).into_any_element(),
        }
    }
}

/// A character's width in the code font at `size` px, as the window draws it.
fn mono_char_w(window: &Window, size: f32) -> f32 {
    let text_system = window.text_system();
    let font = text_system.resolve_font(&gpui::font(crate::mono_font()));
    text_system
        .advance(font, px(size), 'm')
        .map_or(size * 0.6, |s| f32::from(s.width))
        .max(1.)
}

impl BodyView {
    fn render_table(&mut self, table: Arc<Table>, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let rows = table.rows();
        let scroll = self.table_scroll.clone();
        let me = cx.entity().downgrade();
        let follow = select::follow_drag(&self.select, {
            let me = me.clone();
            move |at, _, cx| {
                let _ = me.update(cx, |this, cx| {
                    if this.select.drag_to(at) {
                        cx.notify();
                    }
                });
            }
        });
        let menu = self.select.menu().map(|at| self.render_table_menu(at, cx));
        self.select.begin();
        div()
            .size_full()
            .flex()
            .flex_col()
            .font_family(crate::mono_font())
            .font_features(gpui::FontFeatures::disable_ligatures())
            .text_size(ui(12.))
            .when(table.cut, |d| {
                d.child(
                    div()
                        .px(ui(10.))
                        .pb(ui(4.))
                        .text_color(rgb(c.muted))
                        .font_family(crate::ui_font())
                        .child(tr!(
                            "The first {count} rows are shown.",
                            count = crate::flows::group_digits(table::ROWS_MAX as u64)
                        )),
                )
            })
            .child(
                div()
                    .id("table")
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .cursor(gpui::CursorStyle::IBeam)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, window, cx| {
                            window.focus(&this.focus, cx);
                            this.select
                                .press(e.position, e.click_count, e.modifiers.shift);
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, e: &MouseDownEvent, window, cx| {
                            window.focus(&this.focus, cx);
                            this.table_menu_cell = this
                                .select
                                .spot_at(e.position)
                                .map(|spot| this.cell_of(spot.piece));
                            this.select.set_menu(Some(e.position));
                            cx.notify();
                        }),
                    )
                    .flex()
                    .flex_col()
                    // The head stays at the top while the rows go under it, and
                    // goes sideways with them.
                    .child(
                        div().flex_shrink_0().overflow_hidden().child(
                            div()
                                .ml(self.table_bar.handle.offset().x)
                                .children(self.render_table_rows(0..rows.min(1))),
                        ),
                    )
                    .child(
                        div().flex_1().min_h_0().child(
                            self.table_bar.around(
                                uniform_list(
                                    "table-rows",
                                    rows.saturating_sub(1),
                                    cx.processor(|this, range: std::ops::Range<usize>, _, _| {
                                        this.render_table_rows(range.start + 1..range.end + 1)
                                    }),
                                )
                                .size_full()
                                .with_horizontal_sizing_behavior(
                                    ListHorizontalSizingBehavior::Unconstrained,
                                )
                                .track_scroll(&scroll),
                            ),
                        ),
                    )
                    .children(follow)
                    .children(menu),
            )
    }

    /// What a right click on the table offers.
    fn render_table_menu(&self, at: Point<Pixels>, cx: &mut Context<Self>) -> AnyElement {
        let me = cx.entity().downgrade();
        let (copy, all, row, column, close) = (me.clone(), me.clone(), me.clone(), me.clone(), me);
        let mut items = select::copy_items(
            !self.select.is_empty(),
            self.body.size() <= COPY_MAX as u64,
            move |_, cx| {
                let _ = copy.update(cx, |this, cx| this.copy_selected(cx));
            },
            move |_, cx| {
                let _ = all.update(cx, |this, cx| this.copy_all(cx));
            },
        );
        let on = self.table_menu_cell.is_some();
        items.push(select::Item::new(
            "copy-row",
            tr!("Copy row"),
            on.then(|| {
                Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                    let _ = row.update(cx, |this, cx| this.copy_table_row(cx));
                }) as select::Act
            }),
        ));
        items.push(select::Item::new(
            "copy-column",
            tr!("Copy column"),
            on.then(|| {
                Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                    let _ = column.update(cx, |this, cx| this.copy_table_column(cx));
                }) as select::Act
            }),
        ));
        select::render_menu(at, items, move |_, cx| {
            let _ = close.update(cx, |this, cx| {
                this.close_menu();
                cx.notify();
            });
        })
    }

    fn render_table_rows(&self, range: std::ops::Range<usize>) -> Vec<AnyElement> {
        let c = colors();
        let Some(table) = &self.table else {
            return Vec::new();
        };
        let digits = table.rows().to_string().len();
        let number_w = digits as f32 * self.table_char_w + 16.;
        range
            .map(|row| {
                let cells = table.cells(row);
                let head = row == 0;
                div()
                    .id(("table-row", row))
                    .h(ui(LINE_HEIGHT))
                    .flex()
                    .flex_row()
                    .items_center()
                    .when(head, |d| d.font_weight(FontWeight::BOLD).bg(rgb(c.chip)))
                    .when(!head && row % 2 == 0, |d| d.bg(rgb(c.code)))
                    .child(
                        div()
                            .w(ui(number_w))
                            .flex_shrink_0()
                            .pr(ui(8.))
                            .flex()
                            .justify_end()
                            .text_color(rgb(c.faint))
                            .when(!head, |d| d.child(row.to_string())),
                    )
                    .children(table.widths.iter().enumerate().map(|(i, width)| {
                        let shown = cells
                            .get(i)
                            .map(|cell| table::shown(cell))
                            .unwrap_or_default();
                        div()
                            .w(ui(*width as f32 * self.table_char_w + 14.))
                            .h_full()
                            .flex_shrink_0()
                            .px(ui(7.))
                            .flex()
                            .items_center()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .border_l_1()
                            .border_color(rgb(c.line))
                            .child({
                                let key = self.cell_key(row, i);
                                let found = self
                                    .found_in_cell(key, &shown)
                                    .into_iter()
                                    .map(|(range, now)| (range, found_style(now)))
                                    .collect();
                                self.select.piece_at(key, shown.into(), found)
                            })
                    }))
                    .into_any_element()
            })
            .collect()
    }

    /// The table's scroll bar, and where the table is scrolled to.
    pub fn table_bar(&self) -> &Scrolled {
        &self.table_bar
    }

    /// Whether the body is shown as its table.
    fn is_table(&self) -> bool {
        self.table.is_some() && self.mode() == BodyMode::Table
    }

    /// How many columns the table has, at least one.
    fn table_cols(&self) -> usize {
        self.table.as_ref().map_or(1, |t| t.widths.len().max(1))
    }

    /// A cell's piece of the selection: row by row, a column at a time, so a
    /// selection runs through the cells as it does through text.
    fn cell_key(&self, row: usize, col: usize) -> u64 {
        (row * self.table_cols() + col) as u64
    }

    /// The row and the column of the cell a piece is.
    fn cell_of(&self, key: u64) -> (usize, usize) {
        let cols = self.table_cols();
        (key as usize / cols, key as usize % cols)
    }

    fn table_ends(&self) -> Option<(Spot, Spot)> {
        let last = self.table.as_ref()?.rows().checked_sub(1)?;
        Some((
            Spot::default(),
            Spot {
                piece: self.cell_key(last, self.table_cols() - 1),
                at: usize::MAX,
            },
        ))
    }

    /// The cells from `start` to `end`, as TSV: a tab between cells, a line
    /// between rows. A cell selected whole is copied whole, as it came; one
    /// selected in part, as much of it as was shown and selected.
    fn table_text(&self, start: Spot, end: Spot) -> Copied {
        let Some(table) = &self.table else {
            return Copied::Nothing;
        };
        let cols = self.table_cols();
        let (first, last) = (self.cell_of(start.piece), self.cell_of(end.piece));
        let last_row = last.0.min(table.rows().saturating_sub(1));
        let mut text = String::new();
        for row in first.0..=last_row {
            let cells = table.cells(row);
            let from_col = if row == first.0 { first.1 } else { 0 };
            let to_col = if row == last.0 { last.1 } else { cols - 1 };
            if row > first.0 {
                text.push('\n');
            }
            for col in from_col..=to_col {
                if col > from_col {
                    text.push('\t');
                }
                let cell = cells.get(col).map(String::as_str).unwrap_or("");
                let shown = table::shown(cell);
                let from = if (row, col) == first { start.at } else { 0 };
                let to = if (row, col) == last {
                    end.at
                } else {
                    usize::MAX
                };
                if from == 0 && to >= shown.len() {
                    text.push_str(cell);
                } else {
                    let to = select::floor(&shown, to.min(shown.len()));
                    let from = select::floor(&shown, from.min(to));
                    text.push_str(&shown[from..to]);
                }
            }
            if text.len() > COPY_MAX {
                return Copied::TooLarge;
            }
        }
        if text.is_empty() {
            Copied::Nothing
        } else {
            Copied::Text(text)
        }
    }

    /// The row a right click was on, as TSV.
    fn copy_table_row(&mut self, cx: &mut Context<Self>) {
        self.close_menu();
        if let (Some(table), Some((row, _))) = (&self.table, self.table_menu_cell) {
            cx.write_to_clipboard(ClipboardItem::new_string(table.cells(row).join("\t")));
        }
        cx.notify();
    }

    /// The column a right click was on, its name first, a line a row.
    fn copy_table_column(&mut self, cx: &mut Context<Self>) {
        self.close_menu();
        if let (Some(table), Some((_, col))) = (&self.table, self.table_menu_cell) {
            let mut text = String::new();
            for row in 0..table.rows() {
                if row > 0 {
                    text.push('\n');
                }
                text.push_str(table.cells(row).get(col).map_or("", String::as_str));
                if text.len() > COPY_MAX {
                    break;
                }
            }
            if text.len() <= COPY_MAX {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }
        cx.notify();
    }

    fn render_text(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        // Enough for any size of the panel; what does not fit is clipped.
        self.scale = crate::text_size::scale(window);
        let in_view = (f32::from(window.viewport_size().height) / self.line_h()).ceil() as u64 + 1;
        let list_height = self.height();
        let lines = self.lines();
        let wrapping = self.wrapping();
        if wrapping {
            self.fit_cols(window);
            self.scroll_x = 0.;
        } else {
            self.char_w = mono_char_w(window, 12. * self.scale);
        }
        let (rows, thumb, share) = if wrapping {
            // Every line's rows are not known: as many as the line in view
            // has, for each.
            let rows_at_top = self.rows_of(self.place.line) as u64;
            (
                self.wrapped_rows_in_view(in_view, cx),
                thumb_size(
                    list_height,
                    lines.saturating_mul(rows_at_top),
                    self.line_h(),
                ),
                self.wrapped_share(),
            )
        } else {
            let max = self.max_position();
            (
                self.rows_in_view(in_view, cx),
                thumb_size(list_height, self.folds.rows(lines), self.line_h()),
                if max > 0. { self.position() / max } else { 0. },
            )
        };
        let thumb_top = share as f32 * (list_height - thumb);
        // Sideways, once the lines in view have said how wide they are.
        self.scroll_x = self.scroll_x.min(self.max_x());
        let (across_start, across_view) = self.across_track();
        let across = crate::scrollbar::thumb(across_view, self.max_x(), self.scroll_x);
        let bounds = Rc::clone(&self.bounds);
        let me = cx.entity().downgrade();
        let follow = select::follow_drag(&self.select, {
            let me = me.clone();
            move |at, _, cx| {
                let _ = me.update(cx, |this, cx| this.drag_selection(at, cx));
            }
        });
        let menu = self.select.menu().map(|at| {
            let (copy, all, close, unfold) = (me.clone(), me.clone(), me.clone(), me.clone());
            let mut items = select::copy_items(
                !self.select.is_empty(),
                self.body.size() <= COPY_MAX as u64,
                move |_, cx| {
                    let _ = copy.update(cx, |this, cx| this.copy_selected(cx));
                },
                move |_, cx| {
                    let _ = all.update(cx, |this, cx| this.copy_all(cx));
                },
            );
            // Read what was dragged across, whatever it turns out to be:
            // the way to read a part of a line, or a value the view does
            // not see as one.
            if let Some(opens) = self.opens.clone()
                && let Copied::Text(chosen) = self.select.selected_text()
            {
                let field = self.selected_field();
                items.push(select::Item::new(
                    "read-this",
                    tr!("Read this"),
                    Some(Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                        opens(
                            ToOpen::Value {
                                text: chosen.clone(),
                                field: field.clone(),
                                asked: true,
                            },
                            cx,
                        );
                    }) as select::Act),
                ));
            }
            // What the selection turned out to be: a time, a token, something
            // packed. Choosing one draws it after the value it was read from;
            // `Copy reading` takes the whole of it instead.
            let pick = me.clone();
            items.extend(reading::items(&self.readings, move |reading, _, cx| {
                let mut opened = None;
                let _ = pick.update(cx, |this, cx| {
                    // What holds a body of its own is opened as one; the
                    // rest is drawn beside the value.
                    let holds_body =
                        matches!(reading.shown, tracewing_convert::Shown::Bytes { .. });
                    match (holds_body, this.opens.clone()) {
                        (true, Some(opens)) => {
                            let from = this.selected_field().unwrap_or_default();
                            opened = Some((opens, reading, from));
                        }
                        _ => this.draw_reading(reading, cx),
                    }
                    this.close_menu();
                });
                if let Some((opens, reading, from)) = opened {
                    opens(ToOpen::Reading(reading, from), cx);
                }
            }));
            let copied = me.clone();
            items.extend(reading::copy_item(&self.readings, move |_, cx| {
                let _ = copied.update(cx, |this, cx| {
                    this.close_menu();
                    cx.notify();
                });
            }));
            // Any line at all can be opened whole, not only one the reading
            // had to cut: a line wider than the panel is cut by the panel,
            // and says nothing about it.
            if let Some(line) = self.menu_line.filter(|_| self.opens.is_some()) {
                let open = me.clone();
                items.push(select::Item::new(
                    "open-line",
                    tr!("Open this line"),
                    Some(Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                        let _ = open.update(cx, |this, cx| {
                            this.close_menu();
                            this.open_whole_line(line, cx);
                        });
                    }) as select::Act),
                ));
            }
            // Always, and never again: a rule for the field this value sits
            // under, on the host this body came from.
            if let Some(field) = self.selected_field() {
                let known = self
                    .rules
                    .borrow()
                    .reading_for(&self.host, &field)
                    .is_some();
                let chosen = self
                    .select
                    .bounds()
                    .and_then(|(start, _)| self.drawn.get(&start.piece))
                    .or_else(|| self.readings.first())
                    .map(|reading| reading.id);
                let rule_me = me.clone();
                items.push(if known {
                    select::Item::new(
                        "forget-reading",
                        tr!("Forget how {field} is read", field = field.as_str()),
                        Some(Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                            let _ = rule_me.update(cx, |this, cx| {
                                let host = this.host.clone();
                                let _ = this.rules.borrow_mut().forget(&host, &field);
                                this.by_rule.borrow_mut().1.clear();
                                this.close_menu();
                                cx.notify();
                            });
                        }) as select::Act),
                    )
                } else {
                    select::Item::new(
                        "remember-reading",
                        tr!("Always read {field} this way", field = field.as_str()),
                        chosen.map(|id| {
                            Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                                let _ = rule_me.update(cx, |this, cx| {
                                    let host = this.host.clone();
                                    let _ = this.rules.borrow_mut().remember(&host, &field, id);
                                    this.by_rule.borrow_mut().1.clear();
                                    this.close_menu();
                                    cx.notify();
                                });
                            }) as select::Act
                        }),
                    )
                });
            }
            if !self.drawn.is_empty() {
                let clear = me.clone();
                items.push(select::Item::new(
                    "clear-readings",
                    tr!("Clear what was read"),
                    Some(Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                        let _ = clear.update(cx, |this, cx| {
                            this.clear_readings(cx);
                            this.close_menu();
                        });
                    }) as select::Act),
                ));
            }
            if !self.folds.is_empty() {
                items.push(select::Item::new(
                    "unfold-all",
                    tr!("Unfold all"),
                    Some(Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                        let _ = unfold.update(cx, |this, cx| {
                            this.close_menu();
                            this.unfold_all(cx);
                        });
                    }) as select::Act),
                ));
            }
            select::render_menu(at, items, move |_, cx| {
                let _ = close.update(cx, |this, cx| {
                    this.close_menu();
                    cx.notify();
                });
            })
        });
        self.select.begin();

        div()
            .id("body-list")
            .size_full()
            .relative()
            .overflow_hidden()
            // Rounded as the well it sits in, whose bottom it reaches.
            .rounded_b(ui(CORNER))
            .bg(rgb(c.code))
            .text_color(rgb(c.text))
            .font_family(crate::mono_font())
            .font_features(gpui::FontFeatures::disable_ligatures())
            .text_size(ui(12.))
            .cursor(gpui::CursorStyle::IBeam)
            .on_scroll_wheel(cx.listener(Self::on_wheel))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.select
                        .press(e.position, e.click_count, e.modifiers.shift);
                    this.readings = Vec::new();
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    this.select.set_menu(Some(e.position));
                    this.readings = this.read_selection();
                    this.menu_line = this
                        .select
                        .spot_at(e.position)
                        .map(|spot| this.line_spot(spot).0);
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging {
                    this.drag_to(event.position.y, cx);
                }
                if this.dragging_x.is_some() {
                    this.drag_x_to(f32::from(event.position.x), cx);
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, e: &gpui::MouseUpEvent, _, cx| {
                    this.dragging = false;
                    if this.dragging_x.take().is_some() {
                        cx.notify();
                    }
                    // A press that went nowhere, inside a value: the whole
                    // value is taken and read, so that nobody has to drag
                    // across a JWT exactly to have it read. A drag is left
                    // alone -- what was dragged is what was meant.
                    // Pressed twice, the reader is asking rather than
                    // passing through, and the answer comes whatever the
                    // value turns out to be.
                    match e.click_count {
                        1 if this.select.is_empty() => this.read_value_at(e.position, false, cx),
                        2 => this.read_value_at(e.position, true, cx),
                        _ => {}
                    }
                }),
            )
            .child(
                canvas(
                    move |b, window, _| {
                        // What is drawn from it was drawn from the old size.
                        if bounds.replace(b).size != b.size {
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top(px(-self.sub))
                    .left_0()
                    .right(px(BAR_W))
                    .children(rows),
            )
            .children(follow)
            .children(menu)
            .child(
                div()
                    .id("body-bar")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(px(BAR_W))
                    .cursor(gpui::CursorStyle::Arrow)
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                            // The bar's, not the start of a selection.
                            cx.stop_propagation();
                            this.dragging = true;
                            this.drag_to(event.position.y, cx);
                        }),
                    )
                    // None when every line fits, as the other bars.
                    .when(thumb < list_height, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(thumb_top))
                                .h(px(thumb))
                                .left(px((BAR_W - crate::scrollbar::THUMB_W) / 2.))
                                .w(px(crate::scrollbar::THUMB_W))
                                .rounded(px(crate::scrollbar::THUMB_W / 2.))
                                .bg(rgb(c.splitter)),
                        )
                    }),
            )
            .when_some(across, |d, (long, at)| {
                let held = self.dragging_x.is_some();
                d.child(
                    div()
                        .id("body-bar-x")
                        .absolute()
                        .left(px(across_start))
                        .right(px(BAR_W))
                        .bottom_0()
                        .h(px(BAR_W))
                        .cursor(gpui::CursorStyle::Arrow)
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                // The bar's, not the start of a selection.
                                cx.stop_propagation();
                                let x = f32::from(event.position.x - this.bounds.get().left())
                                    - across_start;
                                // A press off the thumb brings its middle there first.
                                if x < at || x > at + long {
                                    let track = (across_view - long).max(1.);
                                    let max = this.max_x();
                                    this.scroll_x =
                                        (max * ((x - long / 2.).max(0.) / track)).clamp(0., max);
                                }
                                this.dragging_x =
                                    Some((f32::from(event.position.x), this.scroll_x));
                                cx.notify();
                            }),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(at))
                                .w(px(long))
                                // On the pane's bottom edge.
                                .bottom_0()
                                .h(px(crate::scrollbar::THUMB_W))
                                .rounded(px(crate::scrollbar::THUMB_W / 2.))
                                .bg(rgb(if held { c.muted } else { c.splitter })),
                        ),
                )
            })
    }
}

fn thumb_size(list_height: f32, lines: u64, line_h: f32) -> f32 {
    let content = lines as f32 * line_h;
    if content <= list_height {
        list_height
    } else {
        (list_height * list_height / content).max(THUMB_MIN)
    }
}

pub(crate) fn highlight(token: Token) -> HighlightStyle {
    let c = colors();
    let color = match token {
        Token::Key | Token::Tag => c.json_key,
        Token::Str => c.json_string,
        Token::Number => c.json_number,
        Token::Literal | Token::Attr => c.json_literal,
        Token::Punct => c.json_punct,
        Token::Comment => c.code_comment,
        Token::Keyword => c.code_keyword,
    };
    HighlightStyle {
        color: Some(Hsla::from(rgb(color))),
        ..Default::default()
    }
}

/// `2.1 MB · 40,000 lines`, or how far the reading has got.
pub fn summary(body: &Body) -> String {
    summary_in(body, false)
}

/// As [`summary`], counting the lines of a dump with `hex`.
pub fn summary_in(body: &Body, hex: bool) -> String {
    // The text's own lines, not those it was cut into.
    let lines = if hex {
        body.lines_in(true)
    } else {
        body.sent_lines()
    };
    if hex || body.is_scanned() {
        tr_n!(
            lines,
            "{size} · {count} line",
            "{size} · {count} lines",
            size = size_text(body.size()),
            count = crate::flows::group_digits(lines)
        )
    } else {
        tr!(
            "{size} · reading {percent}%",
            size = size_text(body.size()),
            percent = format!(
                "{:.0}",
                body.scanned() as f64 * 100. / body.size().max(1) as f64
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bound is the point: `Kept` is what the three per-line caches in
    /// the view are, so a caller cannot hold a million answers by leaving a
    /// check out. This covers the type; the fields' types are what put
    /// `marked`, `ruled` and `rows_of` behind it.
    #[test]
    fn what_is_kept_is_bounded() {
        let mut kept: Kept<u64> = Kept::new(8);
        for key in 0..100 {
            kept.insert(key, key * 2);
            assert!(kept.len() <= 8, "{} kept at {key}", kept.len());
            assert_eq!(kept.get(key), Some(&(key * 2)), "the newest is there");
        }
        // The lot goes, not the oldest: this is a cache to refill, not an
        // LRU, and a reader who scrolled past them is not looking at them.
        let mut kept: Kept<u64> = Kept::new(2);
        kept.insert(1, 1);
        kept.insert(2, 2);
        kept.insert(3, 3);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept.get(1), None);
        assert_eq!(kept.get(2), None);
        kept.clear();
        assert_eq!(kept.len(), 0);
        assert_eq!(kept.get(3), None);
    }

    #[test]
    fn folds_hide_lines_and_rows_find_them() {
        let mut folds = Folds::default();
        folds.fold(2, 5);
        folds.fold(10, 12);
        // 0 1 2 [3 4 5] 6 7 8 9 10 [11 12] 13
        assert_eq!(folds.rows(14), 9);
        let lines: Vec<u64> = (0..9).map(|row| folds.line_of(row)).collect();
        assert_eq!(lines, [0, 1, 2, 6, 7, 8, 9, 10, 13]);
        for (row, line) in lines.iter().enumerate() {
            assert_eq!(folds.row_of(*line), row as u64);
        }
        assert_eq!(folds.row_of(4), 2, "hidden: the fold's row");
        assert_eq!(folds.next_line(2), 6);
        assert_eq!(folds.next_line(6), 7);
    }

    #[test]
    fn folding_the_outer_lets_the_inner_go() {
        let mut folds = Folds::default();
        folds.fold(3, 4);
        folds.fold(6, 8);
        folds.fold(1, 7);
        assert_eq!(folds.0.into_iter().collect::<Vec<_>>(), [(1, 7)]);
        let mut folds = Folds::default();
        folds.fold(3, 4);
        assert!(folds.unfold(3));
        assert!(!folds.unfold(3));
        assert!(folds.is_empty());
    }
}
