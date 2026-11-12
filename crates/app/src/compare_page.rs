//! The compare screen: two flows side by side, and what differs between
//! them. Picked with Ctrl (⌘ on a Mac) and a click on a second row, or with
//! "Compare with…" in the inspector and a click on the other.

use crate::{
    colors::colors,
    compare::{self, BodyText, Change, Line, Node, Pair, Row},
    detail::{Detail, Side, message_of},
    flows::{size_text, time_text},
    traffic::{Screen, Traffic},
    widgets::{Reachable, eyebrow},
};
use gpui::{
    AnyElement, Context, FontWeight, HighlightStyle, Hsla, SharedString, StyledText,
    UniformListScrollHandle, div, prelude::*, px, rgb, uniform_list,
};
use std::{collections::HashSet, ops::Range, path::Path, sync::Arc};
use tracewing_i18n::{tr, tr_n};
use tracewing_store::{schema::Library, session::BodyReader};

const LINE_H: f32 = 20.;
const NUMBER_W: f32 = 52.;
/// The most of a line drawn.
const LINE_SHOWN: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparePart {
    Body,
    Headers,
    Query,
}

/// Both bodies of one side, and their lines lined up.
#[derive(Debug)]
pub struct BodyDiff {
    pub a: BodyText,
    pub b: BodyText,
    pub lines: Vec<Line>,
    pub changed: usize,
    /// The fields that differ, when both are JSON or read as JSON.
    pub tree: Option<Vec<Node>>,
    pub fields: usize,
}

#[derive(Debug)]
pub enum BodyLoad {
    Loading,
    Ready(Arc<BodyDiff>),
    /// Why they cannot be compared.
    Not(SharedString),
}

pub struct ComparePage {
    pub a: Detail,
    pub b: Detail,
    pub side: Side,
    pub part: ComparePart,
    /// The request's bodies, then the response's.
    pub bodies: [BodyLoad; 2],
    /// The folds opened on each side, by where they start.
    open: [HashSet<usize>; 2],
    rows: [Vec<Row>; 2],
    /// Fields rather than lines, where both bodies are JSON.
    pub tree: bool,
    /// The objects and arrays closed in each tree, by place.
    closed: [HashSet<usize>; 2],
    /// The nodes each tree draws.
    nodes: [Vec<usize>; 2],
    scroll: UniformListScrollHandle,
}

fn at(side: Side) -> usize {
    match side {
        Side::Request => 0,
        Side::Response => 1,
    }
}

impl ComparePage {
    pub fn body(&self, side: Side) -> &BodyLoad {
        &self.bodies[at(side)]
    }

    /// The rows the body of `side` draws.
    pub fn rows(&self, side: Side) -> &[Row] {
        &self.rows[at(side)]
    }

    /// The nodes the tree of `side` draws, by place in its tree.
    pub fn nodes(&self, side: Side) -> &[usize] {
        &self.nodes[at(side)]
    }

    /// Whether `side`'s bodies are shown by field.
    pub fn shows_tree(&self, side: Side) -> bool {
        self.tree && matches!(self.body(side), BodyLoad::Ready(diff) if diff.tree.is_some())
    }

    fn fill_rows(&mut self, side: Side) {
        let i = at(side);
        (self.rows[i], self.nodes[i]) = match &self.bodies[i] {
            BodyLoad::Ready(diff) => (
                compare::rows(&diff.lines, &self.open[i]),
                diff.tree
                    .as_deref()
                    .map(|tree| compare::shown_nodes(tree, &self.closed[i]))
                    .unwrap_or_default(),
            ),
            _ => Default::default(),
        };
    }

    pub fn headers(&self, side: Side) -> Vec<Pair> {
        let of = |d: &Detail| d.headers(side).map(<[_]>::to_vec).unwrap_or_default();
        compare::pairs(&of(&self.a), &of(&self.b), true)
    }

    pub fn trailers(&self) -> Vec<Pair> {
        compare::pairs(
            self.a.trailers(Side::Response),
            self.b.trailers(Side::Response),
            true,
        )
    }

    pub fn query(&self) -> Vec<Pair> {
        compare::pairs(
            &compare::query_of(&self.a.url),
            &compare::query_of(&self.b.url),
            false,
        )
    }

    /// How many things differ in `part` of `side`, if that is known yet.
    pub fn differences(&self, side: Side, part: ComparePart) -> Option<usize> {
        let differ = |pairs: Vec<Pair>| pairs.iter().filter(|p| p.change() != Change::Same).count();
        match part {
            ComparePart::Headers => {
                let trailers = if side == Side::Response {
                    differ(self.trailers())
                } else {
                    0
                };
                Some(differ(self.headers(side)) + trailers)
            }
            ComparePart::Query => Some(differ(self.query())),
            ComparePart::Body => match self.body(side) {
                BodyLoad::Ready(diff) if self.shows_tree(side) => Some(diff.fields),
                BodyLoad::Ready(diff) => Some(diff.changed),
                _ => None,
            },
        }
    }
}

/// A body as lines, or `None` if it was not kept.
fn read_body(session_dir: &Path, protos: &Library, d: &Detail, side: Side) -> Option<BodyText> {
    let facts = d.body(side);
    if !facts.stored.exists() {
        return (facts.bytes == 0).then(BodyText::default);
    }
    let mut data = BodyReader::new(session_dir).unpacked(&facts.stored, compare::BODY_MAX + 1)?;
    let cut = data.len() > compare::BODY_MAX || facts.truncated;
    data.truncate(compare::BODY_MAX);
    let content_type = match side {
        Side::Request => d.req_content_type.as_deref(),
        Side::Response => d.content_type.as_deref(),
    };
    let message = message_of(protos, &d.url, content_type, side);
    Some(compare::body_text(
        &data,
        content_type,
        message.as_ref(),
        cut,
    ))
}

fn body_diff(session_dir: &Path, protos: &Library, a: &Detail, b: &Detail, side: Side) -> BodyLoad {
    match (
        read_body(session_dir, protos, a, side),
        read_body(session_dir, protos, b, side),
    ) {
        (Some(mut a), Some(mut b)) => {
            let lines = compare::diff_lines(&a.lines, &b.lines);
            let changed = compare::changed_count(&lines);
            // What the bodies hold is needed no longer once compared.
            let tree = match (a.json.take(), b.json.take()) {
                (Some(ja), Some(jb)) => Some(compare::diff_tree(&ja, &jb)),
                _ => None,
            };
            let fields = tree.as_deref().map_or(0, compare::fields_changed);
            BodyLoad::Ready(Arc::new(BodyDiff {
                a,
                b,
                lines,
                changed,
                tree,
                fields,
            }))
        }
        _ => BodyLoad::Not(tr!("A body was not kept, so the two cannot be compared.").into()),
    }
}

/// The last flow before `d` with the same method, host and path, its query
/// aside: the call made before, to be compared with.
pub fn previous_call(flows: &rusqlite::Connection, d: &Detail) -> Option<i64> {
    let (_, path) = crate::detail::host_and_path(&d.url);
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    // By the index on host, newest first; a path is itself or itself and a query.
    flows
        .query_row(
            "SELECT id FROM flows
             WHERE host = ?1 AND id < ?2 AND method = ?3
               AND substr(path, 1, length(?4)) = ?4
               AND (length(path) = length(?4) OR substr(path, length(?4) + 1, 1) = '?')
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![d.host, d.id, d.method, path],
            |r| r.get(0),
        )
        .ok()
}

impl Traffic {
    /// Shows flows `a` and `b` side by side.
    pub fn compare(&mut self, a: i64, b: i64, cx: &mut Context<Self>) {
        self.compare_pick = None;
        let Some(flows) = self.flows() else {
            return;
        };
        let session_dir = self.session_dir().to_path_buf();
        let (Ok(da), Ok(db)) = (
            Detail::load(flows.conn(), &session_dir, a),
            Detail::load(flows.conn(), &session_dir, b),
        ) else {
            return;
        };
        // Compared again, as by Swap, the side and part looked at stay.
        let side = self
            .compare_page
            .as_ref()
            .map_or(self.inspector.side, |page| page.side);
        let part = match self.compare_page.as_ref().map(|p| p.part) {
            Some(ComparePart::Query) if side == Side::Response => ComparePart::Body,
            Some(part) => part,
            None => ComparePart::Body,
        };
        self.compare_page = Some(ComparePage {
            a: da.clone(),
            b: db.clone(),
            side,
            part,
            bodies: [BodyLoad::Loading, BodyLoad::Loading],
            open: Default::default(),
            rows: Default::default(),
            tree: self.compare_page.as_ref().is_none_or(|page| page.tree),
            closed: Default::default(),
            nodes: Default::default(),
            scroll: UniformListScrollHandle::new(),
        });
        self.set_screen(Screen::Compare, cx);
        let protos = Arc::clone(&self.protos);
        cx.spawn(async move |this, cx| {
            let bodies = cx
                .background_spawn(async move {
                    [Side::Request, Side::Response]
                        .map(|side| body_diff(&session_dir, &protos, &da, &db, side))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let Some(page) = this.compare_page.as_mut() else {
                    return;
                };
                if (page.a.id, page.b.id) != (a, b) {
                    return;
                }
                page.bodies = bodies;
                page.fill_rows(Side::Request);
                page.fill_rows(Side::Response);
                cx.notify();
            });
        })
        .detach();
    }

    /// Starts choosing a flow to compare the one selected with.
    pub fn start_compare(&mut self, cx: &mut Context<Self>) {
        self.compare_pick = self.selected;
        cx.notify();
    }

    pub fn cancel_compare(&mut self, cx: &mut Context<Self>) {
        self.compare_pick = None;
        cx.notify();
    }

    /// What a click on row `id` does when comparing: true if it compared.
    pub fn compare_on_click(&mut self, id: i64, secondary: bool, cx: &mut Context<Self>) -> bool {
        let with = match self.compare_pick {
            Some(pick) => Some(pick),
            None if secondary => self.selected,
            None => None,
        };
        match with {
            Some(with) if with != id => {
                self.compare(with, id, cx);
                true
            }
            _ => false,
        }
    }

    pub fn swap_compared(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = &self.compare_page {
            let (a, b) = (page.a.id, page.b.id);
            self.compare(b, a, cx);
        }
    }

    pub fn set_compare_side(&mut self, side: Side, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.compare_page {
            page.side = side;
            if side == Side::Response && page.part == ComparePart::Query {
                page.part = ComparePart::Body;
            }
            page.scroll = UniformListScrollHandle::new();
        }
        cx.notify();
    }

    pub fn set_compare_part(&mut self, part: ComparePart, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.compare_page {
            page.part = part;
        }
        cx.notify();
    }

    /// Fields, or lines.
    pub fn set_compare_tree(&mut self, tree: bool, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.compare_page {
            page.tree = tree;
            page.scroll = UniformListScrollHandle::new();
        }
        cx.notify();
    }

    /// Closes the object or array at `node` in the tree shown, or opens it.
    pub fn toggle_compare_node(&mut self, node: usize, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.compare_page {
            let side = page.side;
            let closed = &mut page.closed[self::at(side)];
            if !closed.remove(&node) {
                closed.insert(node);
            }
            page.fill_rows(side);
        }
        cx.notify();
    }

    fn render_compare_nodes(
        &mut self,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = colors();
        let Some(page) = &self.compare_page else {
            return Vec::new();
        };
        let side = page.side;
        let BodyLoad::Ready(diff) = page.body(side) else {
            return Vec::new();
        };
        let Some(tree) = diff.tree.as_deref() else {
            return Vec::new();
        };
        let closed = &page.closed[at(side)];
        let value = |text: Option<&String>, ground: Option<u32>| {
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .px(px(8.))
                .flex()
                .items_center()
                .overflow_hidden()
                .whitespace_nowrap()
                .when_some(ground, |d, ground| d.bg(rgb(ground)))
                .when(text.is_none(), |d| d.bg(rgb(c.track)))
                .children(text.cloned())
        };
        page.nodes(side)
            [range.start.min(page.nodes(side).len())..range.end.min(page.nodes(side).len())]
            .iter()
            .map(|&i| {
                let node = &tree[i];
                // The indent inside, so that it does not widen the column.
                let key = div()
                    .h_full()
                    .pl(px(8. + node.depth as f32 * 16.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(
                        div()
                            .w(px(16.))
                            .flex_shrink_0()
                            .flex()
                            .justify_center()
                            .text_size(px(20.))
                            .line_height(px(LINE_H))
                            .text_color(rgb(c.faint))
                            .when(node.inside.is_some(), |d| {
                                d.child(if closed.contains(&i) { "▸" } else { "▾" })
                            }),
                    )
                    .child(div().text_color(rgb(c.json_key)).child(node.key.clone()));
                let key = div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .child(key);
                let row = div()
                    .id(("compare-node", i))
                    .h(px(LINE_H))
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(2.))
                    .child(key);
                match node.inside {
                    Some(count) => row
                        .cursor_pointer()
                        .hover(|d| d.bg(rgb(c.hover)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .px(px(8.))
                                .flex()
                                .items_center()
                                .text_color(rgb(c.muted))
                                .child(tr_n!(
                                    count,
                                    "{count} field differs",
                                    "{count} fields differ",
                                    count = crate::flows::group_digits(count as u64)
                                )),
                        )
                        .child(div().flex_1())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.toggle_compare_node(i, cx)),
                        )
                        .into_any_element(),
                    None => {
                        let left = matches!(node.change, Change::Removed | Change::Changed)
                            .then_some(c.error_ground);
                        let right = matches!(node.change, Change::Added | Change::Changed)
                            .then_some(c.ok_ground);
                        row.child(value(node.a.as_ref(), left))
                            .child(value(node.b.as_ref(), right))
                            .into_any_element()
                    }
                }
            })
            .collect()
    }

    /// Unfolds the same lines from `at` in the body shown.
    pub fn open_compare_fold(&mut self, at: usize, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.compare_page {
            let side = page.side;
            page.open[self::at(side)].insert(at);
            page.fill_rows(side);
        }
        cx.notify();
    }

    fn render_compare_rows(
        &mut self,
        range: Range<usize>,
        _cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = colors();
        let Some(page) = &self.compare_page else {
            return Vec::new();
        };
        let side = page.side;
        let BodyLoad::Ready(diff) = page.body(side) else {
            return Vec::new();
        };
        // What changed in a line, on its own ground, darker than the line's.
        let marked = |color: u32| HighlightStyle {
            background_color: Some({
                let mut color = Hsla::from(rgb(color));
                color.a = 0.28;
                color
            }),
            ..Default::default()
        };
        let shown = |line: &str, spans: &[Range<usize>], color: u32| -> StyledText {
            let cut = line
                .char_indices()
                .nth(LINE_SHOWN)
                .map_or(line.len(), |(cut, _)| cut);
            let mut text = line[..cut].to_owned();
            if cut < line.len() {
                text.push('…');
            }
            let highlights: Vec<(Range<usize>, HighlightStyle)> = spans
                .iter()
                .filter(|span| span.start < cut)
                .map(|span| (span.start..span.end.min(cut), marked(color)))
                .collect();
            StyledText::new(text).with_highlights(highlights)
        };
        let half = |number: Option<usize>,
                    text: Option<&String>,
                    ground: Option<u32>,
                    spans: &[Range<usize>],
                    strong: u32| {
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_row()
                .items_center()
                .overflow_hidden()
                .when_some(ground, |d, ground| d.bg(rgb(ground)))
                .when(text.is_none(), |d| d.bg(rgb(c.track)))
                .child(
                    div()
                        .w(px(NUMBER_W))
                        .flex_shrink_0()
                        .pr(px(8.))
                        .flex()
                        .justify_end()
                        .text_color(rgb(c.faint))
                        .child(number.map(|n| (n + 1).to_string()).unwrap_or_default()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .children(text.map(|t| shown(t, spans, strong))),
                )
        };
        let rows = page.rows(side);
        range
            .filter_map(|i| rows.get(i).map(|row| (i, *row)))
            .map(|(i, row)| match row {
                Row::Line(line) => {
                    let left = matches!(line.change, Change::Removed | Change::Changed)
                        .then_some(c.error_ground);
                    let right = matches!(line.change, Change::Added | Change::Changed)
                        .then_some(c.ok_ground);
                    let text_a = line.a.and_then(|n| diff.a.lines.get(n));
                    let text_b = line.b.and_then(|n| diff.b.lines.get(n));
                    let (spans_a, spans_b) = match (line.change, text_a, text_b) {
                        (Change::Changed, Some(a), Some(b)) => compare::changed_spans(a, b),
                        _ => Default::default(),
                    };
                    div()
                        .id(("compare-line", i))
                        .h(px(LINE_H))
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(2.))
                        .child(half(line.a, text_a, left, &spans_a, c.error))
                        .child(half(line.b, text_b, right, &spans_b, c.ok))
                        .into_any_element()
                }
                Row::Folded { at, count } => div()
                    .id(("compare-fold", i))
                    .h(px(LINE_H))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id(("compare-fold-open", i))
                            .px(px(10.))
                            .rounded(px(10.))
                            .cursor_pointer()
                            .reachable()
                            .bg(rgb(c.chip))
                            .text_color(rgb(c.muted))
                            .hover(|d| d.text_color(rgb(c.text)))
                            .child(tr_n!(
                                count,
                                "… {count} line the same",
                                "… {count} lines the same",
                                count = crate::flows::group_digits(count as u64)
                            ))
                            .on_click(
                                _cx.listener(move |this, _, _, cx| this.open_compare_fold(at, cx)),
                            ),
                    )
                    .into_any_element(),
            })
            .collect()
    }

    pub(crate) fn render_compare(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let Some(page) = &self.compare_page else {
            return div().id("compare").child(tr!("Nothing to compare."));
        };
        let (a, b) = (&page.a, &page.b);
        let chip = |id: &'static str, label: String, on: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(px(30.))
                .px(px(14.))
                .rounded(px(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
                .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };

        // What each flow is, a line each, the cells that differ marked.
        let status = |d: &Detail| match (d.status, &d.error) {
            (Some(status), _) => match d.grpc_status.filter(|s| *s != 0) {
                Some(grpc) => format!("{status} · gRPC {}", crate::grpc::code_text(grpc)),
                None => status.to_string(),
            },
            (None, Some(error)) => error.clone(),
            (None, None) => "…".to_owned(),
        };
        let facts: Vec<(String, String, String)> = vec![
            (tr!("Method"), a.method.clone(), b.method.clone()),
            (tr!("URL"), a.url.clone(), b.url.clone()),
            (tr!("Status"), status(a), status(b)),
            (tr!("Protocol"), a.protocols(), b.protocols()),
            (
                tr!("Type"),
                a.content_type.clone().unwrap_or_default(),
                b.content_type.clone().unwrap_or_default(),
            ),
            (
                tr!("Sizes"),
                format!(
                    "{} ↑ · {} ↓",
                    size_text(a.request.bytes),
                    size_text(a.response.bytes)
                ),
                format!(
                    "{} ↑ · {} ↓",
                    size_text(b.request.bytes),
                    size_text(b.response.bytes)
                ),
            ),
            (
                tr!("Time"),
                a.duration_us.map(time_text).unwrap_or_default(),
                b.duration_us.map(time_text).unwrap_or_default(),
            ),
            (
                tr!("App"),
                a.app.clone().unwrap_or_default(),
                b.app.clone().unwrap_or_default(),
            ),
        ];
        let cell = |text: String, ground: Option<u32>| {
            div()
                .flex_1()
                .min_w_0()
                .px(px(8.))
                .py(px(2.))
                .rounded(px(6.))
                .when_some(ground, |d, ground| d.bg(rgb(ground)))
                .child(text)
        };
        let table_head = || {
            div()
                .flex()
                .flex_row()
                .gap(px(6.))
                .child(div().w(px(160.)).flex_shrink_0())
                .child(cell(format!("#{}", a.id), None).font_weight(FontWeight::BOLD))
                .child(cell(format!("#{}", b.id), None).font_weight(FontWeight::BOLD))
        };
        let overview = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(table_head())
            .children(
                facts
                    .into_iter()
                    .filter(|(_, va, vb)| !(va.is_empty() && vb.is_empty()))
                    .map(|(name, va, vb)| {
                        let differ = va != vb;
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(6.))
                            .when(!differ, |d| d.text_color(rgb(c.muted)))
                            .child(
                                div()
                                    .w(px(160.))
                                    .flex_shrink_0()
                                    .py(px(2.))
                                    .text_color(rgb(c.muted))
                                    .child(name),
                            )
                            .child(cell(va, differ.then_some(c.error_ground)))
                            .child(cell(vb, differ.then_some(c.ok_ground)))
                    }),
            );

        let side = page.side;
        let mut parts = vec![ComparePart::Body, ComparePart::Headers];
        if side == Side::Request {
            parts.push(ComparePart::Query);
        }
        let part_name = |part: ComparePart| match part {
            ComparePart::Body => tr!("Body"),
            ComparePart::Headers => tr!("Headers"),
            ComparePart::Query => tr!("Query"),
        };
        let tabs = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                chip("compare-request", tr!("Request"), side == Side::Request).on_click(
                    cx.listener(|this, _, _, cx| this.set_compare_side(Side::Request, cx)),
                ),
            )
            .child(
                chip("compare-response", tr!("Response"), side == Side::Response).on_click(
                    cx.listener(|this, _, _, cx| this.set_compare_side(Side::Response, cx)),
                ),
            )
            .child(div().w(px(12.)))
            .children(parts.into_iter().map(|part| {
                let label = match page.differences(side, part) {
                    Some(0) => part_name(part),
                    Some(n) => format!("{} {n}", part_name(part)),
                    None => part_name(part),
                };
                let id: &'static str = match part {
                    ComparePart::Body => "compare-body",
                    ComparePart::Headers => "compare-headers",
                    ComparePart::Query => "compare-query",
                };
                chip(id, label, page.part == part)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_compare_part(part, cx)))
            }));

        let pairs_table = |pairs: Vec<Pair>| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(12.))
                .child(table_head())
                .children(pairs.into_iter().map(|pair| {
                    let change = pair.change();
                    let left = matches!(change, Change::Removed | Change::Changed)
                        .then_some(c.error_ground);
                    let right =
                        matches!(change, Change::Added | Change::Changed).then_some(c.ok_ground);
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(6.))
                        .when(change == Change::Same, |d| d.text_color(rgb(c.muted)))
                        .child(
                            div()
                                .w(px(160.))
                                .flex_shrink_0()
                                .py(px(3.))
                                .overflow_hidden()
                                .child(pair.name),
                        )
                        .child(cell(pair.a.unwrap_or_default(), left))
                        .child(cell(pair.b.unwrap_or_default(), right))
                }))
        };
        let note = |text: String| div().py(px(8.)).text_color(rgb(c.muted)).child(text);

        let content: AnyElement = match page.part {
            ComparePart::Headers => {
                let headers = page.headers(side);
                let trailers = (side == Side::Response)
                    .then(|| page.trailers())
                    .filter(|t| !t.is_empty());
                div()
                    .id("compare-pairs")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(if headers.is_empty() {
                        note(tr!("Neither has headers kept."))
                    } else {
                        pairs_table(headers)
                    })
                    .children(trailers.map(|trailers| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(eyebrow(tr!("Trailers")))
                            .child(pairs_table(trailers))
                    }))
                    .into_any_element()
            }
            ComparePart::Query => {
                let query = page.query();
                div()
                    .id("compare-pairs")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(if query.is_empty() {
                        note(tr!("Neither URL has a query."))
                    } else {
                        pairs_table(query)
                    })
                    .into_any_element()
            }
            ComparePart::Body => match page.body(side) {
                BodyLoad::Loading => note(tr!("Comparing…")).into_any_element(),
                BodyLoad::Not(why) => note(why.to_string()).into_any_element(),
                BodyLoad::Ready(diff) => {
                    let kind = |text: &BodyText| {
                        if text.kind.is_empty() {
                            tr!("No body")
                        } else if text.cut {
                            tr!(
                                "{kind}, its first {size} compared",
                                kind = text.kind.clone(),
                                size = size_text(compare::BODY_MAX as u64)
                            )
                        } else {
                            text.kind.clone()
                        }
                    };
                    let tree = page.shows_tree(side);
                    let said = if diff.a.kind.is_empty() && diff.b.kind.is_empty() {
                        tr!("Neither has a body.")
                    } else if diff.changed == 0 {
                        tr!("The bodies are the same.")
                    } else if tree {
                        let mut said = tr_n!(
                            diff.fields,
                            "{count} field differs.",
                            "{count} fields differ.",
                            count = crate::flows::group_digits(diff.fields as u64)
                        );
                        if diff
                            .tree
                            .as_ref()
                            .is_some_and(|t| t.len() >= compare::TREE_MAX)
                        {
                            said.push(' ');
                            said.push_str(&tr!(
                                "The first {count} are listed.",
                                count = crate::flows::group_digits(compare::TREE_MAX as u64)
                            ));
                        }
                        said
                    } else {
                        tr_n!(
                            diff.changed,
                            "{count} line differs.",
                            "{count} lines differ.",
                            count = crate::flows::group_digits(diff.changed as u64)
                        )
                    };
                    let count = if tree {
                        page.nodes(side).len()
                    } else {
                        page.rows(side).len()
                    };
                    let scroll = page.scroll.clone();
                    let views = diff.tree.is_some().then(|| {
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(4.))
                            .child(
                                chip("compare-tree", tr!("Fields"), tree)
                                    .h(px(26.))
                                    .text_size(px(12.))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_compare_tree(true, cx)
                                    })),
                            )
                            .child(
                                chip("compare-lines-view", tr!("Lines"), !tree)
                                    .h(px(26.))
                                    .text_size(px(12.))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_compare_tree(false, cx)
                                    })),
                            )
                    });
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(rgb(c.muted))
                                .child(div().flex_1().child(kind(&diff.a)))
                                .child(div().flex_1().child(kind(&diff.b))),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(10.))
                                .child(div().flex_1().font_weight(FontWeight::BOLD).child(said))
                                .children(views),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .flex()
                                .rounded(px(12.))
                                .overflow_hidden()
                                .bg(rgb(c.code))
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .text_size(px(12.))
                                .child(if tree {
                                    uniform_list(
                                        "compare-fields",
                                        count,
                                        cx.processor(|this, range: Range<usize>, _, cx| {
                                            this.render_compare_nodes(range, cx)
                                        }),
                                    )
                                    .flex_1()
                                    .track_scroll(&scroll)
                                } else {
                                    uniform_list(
                                        "compare-lines",
                                        count,
                                        cx.processor(|this, range: Range<usize>, _, cx| {
                                            this.render_compare_rows(range, cx)
                                        }),
                                    )
                                    .flex_1()
                                    .track_scroll(&scroll)
                                }),
                        )
                        .into_any_element()
                }
            },
        };

        div()
            .id("compare")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr!("Compare")),
                            )
                            .child(div().text_color(rgb(c.muted)).child(tr!(
                                "#{a} beside #{b}: what only #{a} has is red, what only #{b} has is green.",
                                a = a.id,
                                b = b.id
                            ))),
                    )
                    .child(
                        chip("compare-swap", tr!("Swap"), false)
                            .on_click(cx.listener(|this, _, _, cx| this.swap_compared(cx))),
                    )
                    .child(
                        chip("compare-back", tr!("Back to Traffic"), false).on_click(
                            cx.listener(|this, _, _, cx| this.set_screen(Screen::Traffic, cx)),
                        ),
                    ),
            )
            .child(overview)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .p(px(14.))
                    .rounded(px(22.))
                    .bg(rgb(c.surface))
                    .child(tabs)
                    .child(content),
            )
    }
}
