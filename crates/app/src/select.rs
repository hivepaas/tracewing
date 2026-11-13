//! Text that can be selected with the mouse and copied, in views that are not
//! text fields. Each piece of text a view draws says where it was drawn, so a
//! press, a drag or a double click finds the piece and the character under the
//! mouse; the selection then runs from one spot to another, across pieces.

use crate::text_size::ui;
use crate::{colors::colors, input::word_around};
use gpui::{
    AnyElement, App, Bounds, ElementId, GlobalElementId, HighlightStyle, InspectorElementId,
    LayoutId, MouseButton, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, StyledText,
    TextLayout, Window, anchored, canvas, combine_highlights, deferred, div, prelude::*, rgb,
};
use std::{cell::RefCell, collections::BTreeMap, ops::Range, rc::Rc};
use tracewing_i18n::tr;

/// The most a copy takes: past it, a copy is refused rather than cut.
pub const COPY_MAX: usize = 64 << 20;

/// A place in a view's text: a piece, and a byte offset into it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spot {
    pub piece: u64,
    pub at: usize,
}

/// What a copy gave.
#[derive(Debug, PartialEq, Eq)]
pub enum Copied {
    Text(String),
    Nothing,
    TooLarge,
}

#[derive(Default)]
struct Inner {
    anchor: Option<Spot>,
    head: Spot,
    selecting: bool,
    /// Each piece of a view drawn whole, in order: its text, and what goes
    /// before it when it is copied after the one before.
    pieces: Vec<(SharedString, &'static str)>,
    /// The field each piece is the value of, where the view knows one: the
    /// `Set-Cookie` of its value, the `page` of its query field. It turns a
    /// guess about a value into a reading.
    fields: Vec<Option<SharedString>>,
    /// Where pieces were drawn in the last frame.
    drawn: BTreeMap<u64, TextLayout>,
    /// The menu, open where it was asked for.
    menu: Option<Point<Pixels>>,
    /// What Find looks for in the pieces, and the one it is at: its piece
    /// and where in it.
    find: Option<regex::Regex>,
    find_at: Option<(u64, usize)>,
    /// The one Find is at moved, and wants to be scrolled to.
    find_scroll: bool,
}

/// One view's selection. Clones share it.
#[derive(Clone, Default)]
pub struct Selection(Rc<RefCell<Inner>>);

impl Selection {
    /// Called as a view of pieces starts to draw: it says them all again.
    pub fn begin(&self) {
        let mut inner = self.0.borrow_mut();
        inner.pieces.clear();
        inner.fields.clear();
        inner.drawn.clear();
    }

    /// Forgets what was selected, for a view that shows something else now.
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        inner.anchor = None;
        inner.selecting = false;
        inner.menu = None;
    }

    /// The next piece of a view drawn whole. `before` goes between it and the
    /// piece before it when both are copied: a line break, or ": " after a name.
    pub fn piece(
        &self,
        text: impl Into<SharedString>,
        before: &'static str,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
    ) -> SelectableText {
        self.piece_of(None, text, before, highlights)
    }

    /// The same, for the value of a field the view can name.
    pub fn piece_under(
        &self,
        field: impl Into<SharedString>,
        text: impl Into<SharedString>,
        before: &'static str,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
    ) -> SelectableText {
        self.piece_of(Some(field.into()), text, before, highlights)
    }

    fn piece_of(
        &self,
        field: Option<SharedString>,
        text: impl Into<SharedString>,
        before: &'static str,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
    ) -> SelectableText {
        let text = text.into();
        let key = {
            let mut inner = self.0.borrow_mut();
            inner.pieces.push((text.clone(), before));
            inner.fields.push(field);
            inner.pieces.len() as u64 - 1
        };
        self.piece_at(key, text, highlights)
    }

    /// The field the piece `key` is the value of, as the view said when it
    /// drew it.
    pub fn field_of(&self, key: u64) -> Option<SharedString> {
        self.0.borrow().fields.get(key as usize)?.clone()
    }

    /// The whole of the piece `key`, however much of it is selected.
    pub fn text_of(&self, key: u64) -> Option<SharedString> {
        Some(self.0.borrow().pieces.get(key as usize)?.0.clone())
    }

    /// A piece of a view whose text is kept elsewhere, such as a body's line.
    pub fn piece_at(
        &self,
        key: u64,
        text: SharedString,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
    ) -> SelectableText {
        // What Find found in it, the one it is at brighter.
        let found: Vec<(Range<usize>, HighlightStyle)> = {
            let inner = self.0.borrow();
            match &inner.find {
                Some(re) => crate::find::found(re, &text)
                    .map(|range| {
                        let now = inner.find_at == Some((key, range.start));
                        (range, crate::find::found_style(now))
                    })
                    .collect(),
                None => Vec::new(),
            }
        };
        let highlights: Vec<_> = if found.is_empty() {
            highlights
        } else {
            combine_highlights(highlights, found).collect()
        };
        let selected = self.range_in(key, text.len()).filter(|r| !r.is_empty());
        let highlights: Vec<_> = match selected {
            Some(range) => {
                let range = floor(&text, range.start)..floor(&text, range.end);
                let style = HighlightStyle {
                    background_color: Some(rgb(colors().soft).into()),
                    ..Default::default()
                };
                combine_highlights(highlights, [(range, style)]).collect()
            }
            None => highlights,
        };
        SelectableText {
            key,
            text: StyledText::new(text).with_highlights(highlights),
            selection: self.clone(),
        }
    }

    /// Looks for `re` in the pieces of the view, from the first; `None` stops.
    pub fn set_find(&self, re: Option<regex::Regex>) {
        let mut inner = self.0.borrow_mut();
        inner.find = re;
        inner.find_at = None;
        inner.find_scroll = true;
    }

    /// What Find found in the pieces as the view said them, in order: each
    /// piece, and where in it.
    fn found(&self) -> Vec<(u64, Range<usize>)> {
        let inner = self.0.borrow();
        let Some(re) = &inner.find else {
            return Vec::new();
        };
        inner
            .pieces
            .iter()
            .enumerate()
            .flat_map(|(key, (text, _))| {
                crate::find::found(re, text)
                    .map(|range| (key as u64, range))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Which found Find is at, of how many: the first, when it was at none or
    /// at one no longer there. `None` when Find is not looking.
    pub fn find_count(&self) -> Option<(Option<usize>, usize)> {
        self.0.borrow().find.as_ref()?;
        let hits = self.found();
        let mut inner = self.0.borrow_mut();
        let Some((key, start)) = hits.first().map(|(key, range)| (*key, range.start)) else {
            inner.find_at = None;
            return Some((None, 0));
        };
        let now = inner
            .find_at
            .and_then(|(k, s)| hits.iter().position(|(hk, r)| *hk == k && r.start == s));
        let now = now.unwrap_or_else(|| {
            inner.find_at = Some((key, start));
            inner.find_scroll = true;
            0
        });
        Some((Some(now), hits.len()))
    }

    /// The next found, or the one before, round again at either end.
    pub fn find_step(&self, forward: bool) {
        let hits = self.found();
        if hits.is_empty() {
            return;
        }
        let mut inner = self.0.borrow_mut();
        let now = inner
            .find_at
            .and_then(|(k, s)| hits.iter().position(|(hk, r)| *hk == k && r.start == s));
        let count = hits.len();
        let next = match (now, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(i), true) => (i + 1) % count,
            (Some(i), false) => (i + count - 1) % count,
        };
        inner.find_at = Some((hits[next].0, hits[next].1.start));
        inner.find_scroll = true;
    }

    /// The piece Find is at, once, when it has moved and should be scrolled to.
    pub fn take_find_scroll(&self) -> Option<u64> {
        let mut inner = self.0.borrow_mut();
        if !std::mem::take(&mut inner.find_scroll) {
            return None;
        }
        inner.find_at.map(|(key, _)| key)
    }

    /// The selection, start first, if there is one.
    pub fn bounds(&self) -> Option<(Spot, Spot)> {
        let inner = self.0.borrow();
        let anchor = inner.anchor?;
        let head = inner.head;
        (anchor != head).then(|| (anchor.min(head), anchor.max(head)))
    }

    pub fn is_empty(&self) -> bool {
        self.bounds().is_none()
    }

    /// What of piece `key`, `len` bytes long, is selected.
    pub fn range_in(&self, key: u64, len: usize) -> Option<Range<usize>> {
        let (start, end) = self.bounds()?;
        range_of(key, len, start, end)
    }

    /// Where piece `key` was last drawn.
    pub fn drawn_at(&self, key: u64) -> Option<Bounds<Pixels>> {
        self.0.borrow().drawn.get(&key).map(TextLayout::bounds)
    }

    pub fn is_selecting(&self) -> bool {
        self.0.borrow().selecting
    }

    /// The spot under `point`, from where pieces were last drawn. Between rows
    /// it is the end of the row above; beside a piece, its nearer end.
    pub fn spot_at(&self, point: Point<Pixels>) -> Option<Spot> {
        let inner = self.0.borrow();
        let drawn: Vec<(u64, &TextLayout, Bounds<Pixels>)> = inner
            .drawn
            .iter()
            .map(|(key, layout)| (*key, layout, layout.bounds()))
            .collect();
        let (first_key, _, _) = drawn.first()?;
        let in_row: Vec<_> = drawn
            .iter()
            .filter(|(_, _, b)| b.top() <= point.y && point.y <= b.bottom())
            .collect();
        let at = |layout: &TextLayout, p: Point<Pixels>| match layout.index_for_position(p) {
            Ok(i) | Err(i) => i,
        };
        if !in_row.is_empty() {
            // The piece the pointer is in, or past the end of: the nearest that
            // starts at or before it, whatever order the view said its pieces
            // in (a value said before its name, as JSON-RPC's are). Before
            // them all, the first on the row.
            let (key, layout, bounds) = in_row
                .iter()
                .filter(|(_, _, b)| b.left() <= point.x)
                .max_by(|a, b| {
                    f32::from(a.2.left())
                        .total_cmp(&f32::from(b.2.left()))
                        .then(a.0.cmp(&b.0))
                })
                .or_else(|| {
                    in_row
                        .iter()
                        .min_by(|a, b| f32::from(a.2.left()).total_cmp(&f32::from(b.2.left())))
                })
                .expect("the row has a piece");
            let p = if point.x > bounds.right() {
                // Past a wrapped piece's end, on its line.
                Point::new(bounds.right(), point.y)
            } else {
                point
            };
            return Some(Spot {
                piece: *key,
                at: at(layout, p),
            });
        }
        match drawn.iter().rev().find(|(_, _, b)| b.bottom() < point.y) {
            Some((key, layout, _)) => Some(Spot {
                piece: *key,
                at: layout.len(),
            }),
            None => Some(Spot {
                piece: *first_key,
                at: 0,
            }),
        }
    }

    /// A press: a click puts the start here, with shift it moves the end, a
    /// second click takes the word and a third the piece.
    pub fn press(&self, point: Point<Pixels>, clicks: usize, shift: bool) {
        let Some(spot) = self.spot_at(point) else {
            return;
        };
        let text = self.0.borrow().drawn.get(&spot.piece).map(|l| l.text());
        let mut inner = self.0.borrow_mut();
        inner.menu = None;
        if clicks >= 2 && !shift {
            let text = text.unwrap_or_default();
            let range = if clicks == 2 {
                word_around(&text, spot.at)
            } else {
                0..text.len()
            };
            inner.anchor = Some(Spot {
                piece: spot.piece,
                at: range.start,
            });
            inner.head = Spot {
                piece: spot.piece,
                at: range.end,
            };
            inner.selecting = false;
            return;
        }
        if !(shift && inner.anchor.is_some()) {
            inner.anchor = Some(spot);
        }
        inner.head = spot;
        inner.selecting = true;
    }

    /// The mouse moved while the button is down: the end follows it.
    pub fn drag_to(&self, point: Point<Pixels>) -> bool {
        if !self.is_selecting() {
            return false;
        }
        let Some(spot) = self.spot_at(point) else {
            return false;
        };
        let mut inner = self.0.borrow_mut();
        let moved = inner.head != spot;
        inner.head = spot;
        moved
    }

    pub fn release(&self) {
        self.0.borrow_mut().selecting = false;
    }

    /// Selects from `start` to `end`, as select all does.
    pub fn select(&self, start: Spot, end: Spot) {
        let mut inner = self.0.borrow_mut();
        inner.anchor = Some(start);
        inner.head = end;
        inner.selecting = false;
    }

    /// All the pieces of a view drawn whole.
    pub fn select_all(&self) {
        let last = {
            let inner = self.0.borrow();
            inner
                .pieces
                .last()
                .map(|(text, _)| (inner.pieces.len() as u64 - 1, text.len()))
        };
        if let Some((piece, at)) = last {
            self.select(Spot::default(), Spot { piece, at });
        }
    }

    /// The selected text of a view drawn whole, `before` between pieces.
    pub fn selected_text(&self) -> Copied {
        let Some((start, end)) = self.bounds() else {
            return Copied::Nothing;
        };
        let inner = self.0.borrow();
        let mut out = String::new();
        for (key, (text, before)) in inner.pieces.iter().enumerate() {
            let key = key as u64;
            let Some(range) = range_of(key, text.len(), start, end) else {
                continue;
            };
            if key > start.piece {
                out.push_str(before);
            }
            out.push_str(&text[floor(text, range.start)..floor(text, range.end)]);
            if out.len() > COPY_MAX {
                return Copied::TooLarge;
            }
        }
        Copied::Text(out)
    }

    /// Everything a view drawn whole says.
    pub fn all_text(&self) -> Copied {
        let inner = self.0.borrow();
        let mut out = String::new();
        for (key, (text, before)) in inner.pieces.iter().enumerate() {
            if key > 0 {
                out.push_str(before);
            }
            out.push_str(text);
        }
        if out.is_empty() {
            Copied::Nothing
        } else {
            Copied::Text(out)
        }
    }

    pub fn menu(&self) -> Option<Point<Pixels>> {
        self.0.borrow().menu
    }

    pub fn set_menu(&self, at: Option<Point<Pixels>>) {
        let mut inner = self.0.borrow_mut();
        inner.menu = at;
        inner.selecting = false;
    }
}

/// The part of a piece between `start` and `end`.
fn range_of(key: u64, len: usize, start: Spot, end: Spot) -> Option<Range<usize>> {
    if key < start.piece || key > end.piece {
        return None;
    }
    let from = if key == start.piece { start.at } else { 0 };
    let to = if key == end.piece { end.at } else { len };
    Some(from.min(len)..to.min(len))
}

/// `at`, moved back to the start of the character it is in.
pub fn floor(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// While a selection is dragged, the mouse anywhere in the window is told to
/// `on_move`, which moves the selection's end, and the button let go anywhere
/// ends it: a quick drag leaves the view before it says where it is.
pub fn follow_drag(
    selection: &Selection,
    on_move: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static,
) -> Option<AnyElement> {
    if !selection.is_selecting() {
        return None;
    }
    let selection = selection.clone();
    let on_move = Rc::new(on_move);
    Some(
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let on_move = Rc::clone(&on_move);
                window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                    if phase == gpui::DispatchPhase::Capture {
                        on_move(e.position, window, cx);
                    }
                });
                let on_up = selection.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, _| {
                    if phase == gpui::DispatchPhase::Capture {
                        on_up.release();
                    }
                });
            },
        )
        .absolute()
        .size_0()
        .into_any_element(),
    )
}

/// What a menu item does when it is chosen.
pub type Act = Rc<dyn Fn(&mut Window, &mut App)>;

/// An item of a menu: its label, and what it does, if it can be chosen.
pub struct Item {
    pub id: &'static str,
    pub label: String,
    pub act: Option<Act>,
}

impl Item {
    pub fn new(id: &'static str, label: String, act: Option<Act>) -> Self {
        Self { id, label, act }
    }
}

/// The menu a right click opens over selected text: copy what is selected,
/// or everything, which is not offered when it is more than a copy takes.
pub fn copy_menu(
    at: Point<Pixels>,
    can_copy: bool,
    all_fits: bool,
    on_copy: impl Fn(&mut Window, &mut App) + 'static,
    on_copy_all: impl Fn(&mut Window, &mut App) + 'static,
    on_close: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    render_menu(
        at,
        copy_items(can_copy, all_fits, on_copy, on_copy_all),
        on_close,
    )
}

/// The copy menu's items, for a menu that has more.
pub fn copy_items(
    can_copy: bool,
    all_fits: bool,
    on_copy: impl Fn(&mut Window, &mut App) + 'static,
    on_copy_all: impl Fn(&mut Window, &mut App) + 'static,
) -> Vec<Item> {
    let all = if all_fits {
        tr!("Copy all")
    } else {
        tr!("Too large to copy all")
    };
    vec![
        Item::new(
            "copy-selection",
            tr!("Copy"),
            can_copy.then(|| Rc::new(on_copy) as Act),
        ),
        Item::new(
            "copy-all",
            all,
            all_fits.then(|| Rc::new(on_copy_all) as Act),
        ),
    ]
}

/// A menu at `at`, over everything. A press outside it closes it.
pub fn render_menu(
    at: Point<Pixels>,
    items: Vec<Item>,
    on_close: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    let c = colors();
    deferred(
        anchored().position(at).child(
            div()
                .id("copy-menu")
                .min_w(ui(160.))
                .p(ui(4.))
                .flex()
                .flex_col()
                .rounded(ui(10.))
                .bg(rgb(c.surface))
                .border_1()
                .border_color(rgb(c.splitter))
                .shadow_lg()
                .font_family(crate::ui_font())
                .text_size(ui(13.))
                .on_mouse_down_out(move |_, window, cx| on_close(window, cx))
                .children(items.into_iter().enumerate().map(|(nth, item)| {
                    let enabled = item.act.is_some();
                    div()
                        .id((item.id, nth))
                        .px(ui(12.))
                        .py(ui(5.))
                        .rounded(ui(8.))
                        .text_color(rgb(if enabled { c.text } else { c.faint }))
                        .when(enabled, |d| d.cursor_pointer().hover(|s| s.bg(rgb(c.chip))))
                        .child(item.label)
                        .when_some(item.act, |d, act| {
                            d.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                cx.stop_propagation();
                                act(window, cx)
                            })
                        })
                })),
        ),
    )
    .with_priority(2)
    .into_any_element()
}

/// A piece of text that says where it was drawn.
pub struct SelectableText {
    key: u64,
    text: StyledText,
    selection: Selection,
}

impl IntoElement for SelectableText {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for SelectableText {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.text.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text
            .prepaint(id, inspector_id, bounds, state, window, cx);
        self.selection
            .0
            .borrow_mut()
            .drawn
            .insert(self.key, self.text.layout().clone());
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text
            .paint(id, inspector_id, bounds, state, prepaint, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(piece: u64, at: usize) -> Spot {
        Spot { piece, at }
    }

    fn headers() -> Selection {
        let s = Selection::default();
        for (name, value) in [("Host", "example.com"), ("Accept", "*/*")] {
            s.piece(name, "\n", Vec::new());
            s.piece(value, ": ", Vec::new());
        }
        s
    }

    #[test]
    fn a_copy_puts_back_what_goes_between_pieces() {
        let s = headers();
        s.select(spot(1, 0), spot(3, 3));
        assert_eq!(
            s.selected_text(),
            Copied::Text("example.com\nAccept: */*".into())
        );
        // Backwards is the same.
        s.select(spot(2, 2), spot(1, 7));
        assert_eq!(s.selected_text(), Copied::Text(".com\nAc".into()));
        assert_eq!(
            s.all_text(),
            Copied::Text("Host: example.com\nAccept: */*".into())
        );
        s.select_all();
        assert_eq!(s.selected_text(), s.all_text());
    }

    #[test]
    fn nothing_selected_copies_nothing() {
        let s = headers();
        assert_eq!(s.selected_text(), Copied::Nothing);
        s.select(spot(1, 4), spot(1, 4));
        assert!(s.is_empty());
        s.clear();
        assert_eq!(s.bounds(), None);
    }

    #[test]
    fn only_the_selected_part_of_a_piece_is_shaded() {
        let s = headers();
        s.select(spot(1, 4), spot(3, 1));
        assert_eq!(s.range_in(0, 4), None);
        assert_eq!(s.range_in(1, 11), Some(4..11));
        assert_eq!(s.range_in(2, 6), Some(0..6));
        assert_eq!(s.range_in(3, 3), Some(0..1));
    }

    #[test]
    fn a_spot_inside_a_character_starts_it() {
        assert_eq!(floor("tiếng", 3), 2);
        assert_eq!(word_around("a-token value", 4), 2..7);
    }
}
