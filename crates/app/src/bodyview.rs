//! A body in a panel. It keeps its own position, a line and a pixel offset into it,
//! and draws only the lines in view, because a body can have more lines than a
//! pixel offset in an `f32` can tell apart: `uniform_list`'s step grows to 8 px at
//! 6 million lines of 20 px.

use crate::{
    body::{Body, Kind, Line, Token},
    colors::colors,
    flows::size_text,
    select::{self, COPY_MAX, Copied, Selection, Spot},
    syntax::Syntax,
    wrap::{self, Place},
};
use gpui::{
    Bounds, ClipboardItem, Context, FocusHandle, HighlightStyle, Hsla, MouseButton, MouseDownEvent,
    Pixels, Point, Render, ScrollWheelEvent, SharedString, Window, canvas, div, prelude::*, px,
    rgb,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use tracewing_i18n::{tr, tr_n};

pub const LINE_HEIGHT: f32 = 20.;
const GUTTER_W: f32 = 52.;
/// Where an object or an array is folded and unfolded, after the number.
const FOLD_W: f32 = 16.;
const INDENT_W: f32 = 14.;
/// Past this the indent says what it is instead of pushing the text away.
const INDENT_MAX: u32 = 40;
const THUMB_MIN: f32 = 28.;
const BAR_W: f32 = 10.;
/// How often a window with a scan running looks at its progress.
const SCAN_POLL: Duration = Duration::from_millis(50);
/// Lines counted on either side of one whose rows are asked for.
const ROWS_READ_AROUND: u64 = 64;
/// Lines whose rows are kept counted, past which they are counted again.
const ROWS_KEPT: usize = 100_000;

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
}

pub struct BodyView {
    body: Arc<Body>,
    /// The body read again as it was sent, once it is asked for.
    raw: Option<Arc<Body>>,
    /// The bytes as they came, when what is shown was made from them, as
    /// JSON is from MessagePack. Its dump is what Hex shows.
    original: Option<Arc<Body>>,
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
    row_counts: RefCell<HashMap<u64, usize>>,
    folds: Folds,
    mode: BodyMode,
    sub: f32,
    dragging: bool,
    /// Where the view was last drawn: the height sets the scroll range, and a drag
    /// on the bar is measured from its top.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    /// What is selected, by line and offset into what the line shows.
    select: Selection,
    /// The window's, which a press in the body takes so that its keys work.
    focus: FocusHandle,
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
        watch_scan(Arc::clone(&body), cx);
        Self {
            body,
            raw: None,
            original: None,
            syntax,
            top: 0,
            place: Place::default(),
            cols: 80,
            row_counts: RefCell::default(),
            folds: Folds::default(),
            mode: BodyMode::Pretty,
            sub: 0.,
            dragging: false,
            bounds: Rc::new(Cell::new(Bounds::default())),
            select: Selection::default(),
            focus,
        }
    }

    pub fn selection(&self) -> &Selection {
        &self.select
    }

    /// The selected text, as [`Body::text`] gives it.
    pub fn selected_text(&self) -> Copied {
        match self.select.bounds() {
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

    /// The first and the last spot of the body.
    fn ends(&self) -> Option<(Spot, Spot)> {
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
        if let Some(rows) = self.row_counts.borrow().get(&line) {
            return *rows;
        }
        let body = self.shown();
        let lines = self.lines();
        let from = line.saturating_sub(ROWS_READ_AROUND);
        let to = (line + ROWS_READ_AROUND).min(lines);
        let mut counts = self.row_counts.borrow_mut();
        if counts.len() > ROWS_KEPT {
            counts.clear();
        }
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
        (self.height() / LINE_HEIGHT).floor().max(1.) as u64
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
        let line_height = f64::from(LINE_HEIGHT);
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

    fn copy_all(&mut self, cx: &mut Context<Self>) {
        self.select.set_menu(None);
        if let Some((start, end)) = self.ends()
            && let Copied::Text(text) = self.copy(start, end)
        {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        cx.notify();
    }

    fn copy_selected(&mut self, cx: &mut Context<Self>) {
        self.select.set_menu(None);
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
            (BodyMode::Hex, _, Some(original)) => original,
            _ => &self.body,
        }
    }

    /// Shows JSON made from `original`, whose bytes Hex shows. Bytes as
    /// text would say nothing, so there is no Raw.
    pub fn set_original(&mut self, original: Arc<Body>, cx: &mut Context<Self>) {
        watch_scan(Arc::clone(&original), cx);
        self.original = Some(original);
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
        self.place = Place::default();
        self.row_counts.borrow_mut().clear();
        self.folds = Folds::default();
        self.select.clear();
        self.top = 0;
        self.sub = 0.;
        cx.notify();
    }

    fn rows(&self) -> u64 {
        self.folds.rows(self.lines())
    }

    fn height(&self) -> f32 {
        f32::from(self.bounds.get().size.height)
    }

    /// The largest position with a full view below it, in pixels.
    pub fn max_position(&self) -> f64 {
        (self.rows() as f64 * f64::from(LINE_HEIGHT) - f64::from(self.height())).max(0.)
    }

    /// Moves the top of the view to `y` pixels into the content, clamped.
    pub fn set_position(&mut self, y: f64) {
        let y = y.clamp(0., self.max_position());
        let line_height = f64::from(LINE_HEIGHT);
        self.top = (y / line_height) as u64;
        self.sub = (y - self.top as f64 * line_height) as f32;
    }

    fn position(&self) -> f64 {
        self.top as f64 * f64::from(LINE_HEIGHT) + f64::from(self.sub)
    }

    fn on_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = f64::from(f32::from(event.delta.pixel_delta(px(LINE_HEIGHT)).y));
        if self.wrapping() {
            self.scroll_wrapped(-delta);
        } else {
            self.set_position(self.position() - delta);
        }
        cx.notify();
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
        let text_system = window.text_system();
        let font = text_system.resolve_font(&gpui::font(crate::MONO_FONT));
        let char_w = text_system
            .advance(font, px(12.), 'm')
            .map_or(7.2, |size| f32::from(size.width))
            .max(1.);
        let width = f32::from(self.bounds.get().size.width) - BAR_W - GUTTER_W - FOLD_W - 8.;
        let cols = ((width / char_w).floor() as usize).max(8);
        if cols != self.cols {
            self.cols = cols;
            self.row_counts.borrow_mut().clear();
            self.place.row = self.place.row.min(self.rows_of(self.place.line) - 1);
        }
    }

    /// A line, or with text wrapped one row of it: `key` tells it from the
    /// rest for selecting, and only a line's first row has its number.
    fn render_line(
        &self,
        line: u64,
        key: u64,
        numbered: bool,
        shown: Line,
        mark: FoldMark,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let indent = if shown.depth > INDENT_MAX {
            INDENT_MAX as f32 * INDENT_W
        } else {
            shown.depth as f32 * INDENT_W
        };
        let number = if numbered {
            (line + 1).to_string()
        } else {
            String::new()
        };
        let highlights: Vec<_> = if self.hex() {
            Vec::new()
        } else {
            self.syntax
                .runs(&shown.text)
                .into_iter()
                .map(|(range, token)| (range, highlight(token)))
                .collect()
        };
        let text: SharedString = shown.text.into();
        let folded = matches!(mark, FoldMark::Folded(..));
        let mut row = div()
            .h(px(LINE_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .whitespace_nowrap()
            .overflow_hidden()
            .child(
                div()
                    .w(px(GUTTER_W))
                    .flex_shrink_0()
                    .pr_1()
                    .flex()
                    .justify_end()
                    .text_color(rgb(colors().faint))
                    .child(number),
            )
            .child({
                let toggle = div()
                    .id(("fold", line))
                    .w(px(FOLD_W))
                    .flex_shrink_0()
                    .flex()
                    .justify_center()
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
            .child(div().w(px(indent)).flex_shrink_0())
            .child(
                // The note below stays in view however long the text is. A fold's
                // note follows the text, as the closer it stands for would.
                div()
                    .when(!folded, |d| d.flex_1())
                    .min_w_0()
                    .overflow_hidden()
                    .child(self.select.piece_at(key, text, highlights)),
            );
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
            row = row.child(
                div()
                    .flex_shrink_0()
                    .pl_2()
                    .text_color(rgb(colors().faint))
                    .child(tr!("… {size} more", size = size_text(shown.cut))),
            );
        }
        row
    }
}

impl BodyView {
    /// The body's size and lines, as shown.
    pub fn summary(&self) -> String {
        summary_in(self.shown(), self.hex())
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
            for text in self.shown().read_whole(line..line + run) {
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
                            row == 0,
                            part,
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
                    self.render_line(at, at, true, text, mark, cx)
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
        let c = colors();
        // Enough for any size of the panel; what does not fit is clipped.
        let in_view = (f32::from(window.viewport_size().height) / LINE_HEIGHT).ceil() as u64 + 1;
        let list_height = self.height();
        let lines = self.lines();
        let wrapping = self.wrapping();
        if wrapping {
            self.fit_cols(window);
        }
        let (rows, thumb, share) = if wrapping {
            // Every line's rows are not known: as many as the line in view
            // has, for each.
            let rows_at_top = self.rows_of(self.place.line) as u64;
            (
                self.wrapped_rows_in_view(in_view, cx),
                thumb_size(list_height, lines.saturating_mul(rows_at_top)),
                self.wrapped_share(),
            )
        } else {
            let max = self.max_position();
            (
                self.rows_in_view(in_view, cx),
                thumb_size(list_height, self.folds.rows(lines)),
                if max > 0. { self.position() / max } else { 0. },
            )
        };
        let thumb_top = share as f32 * (list_height - thumb);
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
            if !self.folds.is_empty() {
                items.push(select::Item::new(
                    "unfold-all",
                    tr!("Unfold all"),
                    Some(Rc::new(move |_: &mut Window, cx: &mut gpui::App| {
                        let _ = unfold.update(cx, |this, cx| {
                            this.select.set_menu(None);
                            this.unfold_all(cx);
                        });
                    }) as select::Act),
                ));
            }
            select::render_menu(at, items, move |_, cx| {
                let _ = close.update(cx, |this, cx| {
                    this.select.set_menu(None);
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
            .bg(rgb(c.code))
            .text_color(rgb(c.text))
            .font_family(crate::MONO_FONT)
            .font_features(gpui::FontFeatures::disable_ligatures())
            .text_size(px(12.))
            .cursor(gpui::CursorStyle::IBeam)
            .on_scroll_wheel(cx.listener(Self::on_wheel))
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
                    this.select.set_menu(Some(e.position));
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging {
                    this.drag_to(event.position.y, cx);
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
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
                    .child(
                        div()
                            .absolute()
                            .top(px(thumb_top))
                            .h(px(thumb))
                            .left(px(2.))
                            .right(px(2.))
                            .rounded_sm()
                            .bg(rgb(c.splitter)),
                    ),
            )
    }
}

fn thumb_size(list_height: f32, lines: u64) -> f32 {
    let content = lines as f32 * LINE_HEIGHT;
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
    let lines = body.lines_in(hex);
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
