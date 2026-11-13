//! Find, in a body: what is shown searched in the background a chunk at a
//! time, as text or as a table's cells, what it found highlighted as it is
//! drawn, and a step from one to the next bringing it into view.

use std::{ops::Range, sync::Arc};

use gpui::{AppContext as _, Context, ScrollStrategy, point, px};
use regex::Regex;

use super::{BodyView, INDENT_MAX, INDENT_W};
use crate::{body::Body, table, table::Table, wrap};

/// The most a search finds: past it the count says so, and stops.
pub const FOUND_MAX: usize = 10_000;
/// Lines, or a table's rows, searched between two looks at the window.
const CHUNK: u64 = 4_000;

/// Something found: in a line, or in a table's cell (its piece of the
/// selection), and where in its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub at: u64,
    pub range: Range<usize>,
}

/// What Find looks for in a body, what it found so far, and which it is at.
pub struct Found {
    pub re: Regex,
    pub hits: Vec<Hit>,
    /// The whole body has been searched, or [`FOUND_MAX`] reached.
    pub done: bool,
    pub now: Option<usize>,
}

/// How far Find has got: the one it is at, how many, and whether that is all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindCount {
    pub now: Option<usize>,
    pub found: usize,
    pub done: bool,
    pub capped: bool,
}

pub(super) use crate::find::found_style;

/// Searches `from` on, a chunk: the hits, where the next chunk starts, and
/// whether there is no more.
fn search_chunk(
    body: &Body,
    table: Option<&Table>,
    cols: usize,
    hex: bool,
    re: &Regex,
    from: u64,
) -> (Vec<Hit>, u64, bool) {
    let mut hits = Vec::new();
    if let Some(table) = table {
        let rows = table.rows() as u64;
        let to = rows.min(from + CHUNK);
        for row in from..to {
            for (col, cell) in table.cells(row as usize).iter().enumerate() {
                let shown = table::shown(cell);
                hits.extend(crate::find::found(re, &shown).map(|range| Hit {
                    at: row * cols as u64 + col as u64,
                    range,
                }));
            }
        }
        return (hits, to, to >= rows);
    }
    let dump = hex || body.kind() == crate::body::Kind::Binary;
    let lines = body.lines_in(hex);
    let to = lines.min(from + CHUNK);
    let texts: Vec<String> = if dump {
        body.read_in(from..to, true)
            .into_iter()
            .map(|line| line.text)
            .collect()
    } else {
        body.read_whole(from..to)
    };
    for (i, text) in texts.iter().enumerate() {
        hits.extend(crate::find::found(re, text).map(|range| Hit {
            at: from + i as u64,
            range,
        }));
    }
    (hits, to, to >= lines)
}

impl BodyView {
    /// Looks for `re` in what is shown, from the start; `None` stops looking
    /// and takes the highlights away.
    pub fn set_find(&mut self, re: Option<Regex>, cx: &mut Context<Self>) {
        self.find_gen += 1;
        let Some(re) = re else {
            self.find = None;
            cx.notify();
            return;
        };
        self.find = Some(Found {
            re: re.clone(),
            hits: Vec::new(),
            done: false,
            now: None,
        });
        let generation = self.find_gen;
        let body = Arc::clone(self.shown());
        let table = self.is_table().then(|| self.table.clone()).flatten();
        let cols = self.table_cols();
        let hex = self.hex();
        cx.spawn(async move |this, cx| {
            let mut from = 0;
            loop {
                let (body, table, re) = (Arc::clone(&body), table.clone(), re.clone());
                let (hits, next, end) = cx
                    .background_spawn(async move {
                        search_chunk(&body, table.as_deref(), cols, hex, &re, from)
                    })
                    .await;
                let go_on = this
                    .update(cx, |view, cx| {
                        if view.find_gen != generation {
                            return false;
                        }
                        let Some(found) = view.find.as_mut() else {
                            return false;
                        };
                        let first = found.hits.is_empty() && !hits.is_empty();
                        found.hits.extend(hits);
                        found.hits.truncate(FOUND_MAX);
                        found.done = end || found.hits.len() >= FOUND_MAX;
                        let done = found.done;
                        // As it is typed: the first found is shown at once.
                        if first {
                            found.now = Some(0);
                            view.show_found(0);
                        }
                        cx.notify();
                        !done
                    })
                    .unwrap_or(false);
                if !go_on {
                    break;
                }
                from = next;
            }
        })
        .detach();
        cx.notify();
    }

    /// The same search again, over what is shown now: another mode, or the
    /// table.
    pub(super) fn find_again(&mut self, cx: &mut Context<Self>) {
        if let Some(re) = self.find.as_ref().map(|found| found.re.clone()) {
            self.set_find(Some(re), cx);
        }
    }

    pub fn find_count(&self) -> Option<FindCount> {
        self.find.as_ref().map(|found| FindCount {
            now: found.now,
            found: found.hits.len(),
            done: found.done,
            capped: found.hits.len() >= FOUND_MAX,
        })
    }

    /// The next found, or the one before, round again at either end.
    pub fn find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(found) = self.find.as_mut() else {
            return;
        };
        let count = found.hits.len();
        if count == 0 {
            return;
        }
        let now = match (found.now, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(i), true) => (i + 1) % count,
            (Some(i), false) => (i + count - 1) % count,
        };
        found.now = Some(now);
        self.show_found(now);
        cx.notify();
    }

    /// What was found in `line`, from `from` for `len` bytes of it (a wrapped
    /// row), as ranges in that piece, each with whether it is the one Find
    /// is at.
    pub(super) fn found_in(&self, line: u64, from: usize, text: &str) -> Vec<(Range<usize>, bool)> {
        let Some(found) = &self.find else {
            return Vec::new();
        };
        let first = found.hits.partition_point(|hit| hit.at < line);
        found.hits[first..]
            .iter()
            .enumerate()
            .take_while(|(_, hit)| hit.at == line)
            .filter_map(|(i, hit)| {
                let start = hit.range.start.max(from) - from;
                let end = hit.range.end.saturating_sub(from).min(text.len());
                (start < end && text.is_char_boundary(start) && text.is_char_boundary(end))
                    .then(|| (start..end, found.now == Some(first + i)))
            })
            .collect()
    }

    /// The same for a table's cell, its piece `key`.
    pub(super) fn found_in_cell(&self, key: u64, text: &str) -> Vec<(Range<usize>, bool)> {
        self.found_in(key, 0, text)
    }

    /// Brings the `i`th found into view: a fold that hides it opened, the
    /// view scrolled down and sideways to it.
    pub(super) fn show_found(&mut self, i: usize) {
        let Some(hit) = self.find.as_ref().and_then(|f| f.hits.get(i)).cloned() else {
            return;
        };
        if self.is_table() {
            self.show_found_cell(&hit);
            return;
        }
        let line = hit.at;
        let text = if self.hex() || self.shown().kind() == crate::body::Kind::Binary {
            self.shown()
                .read_in(line..line + 1, true)
                .pop()
                .map(|l| l.text)
                .unwrap_or_default()
        } else {
            self.shown()
                .read_whole(line..line + 1)
                .pop()
                .unwrap_or_default()
        };
        if self.wrapping() {
            let starts = wrap::row_starts(&text, self.cols);
            let row = starts
                .partition_point(|start| *start <= hit.range.start)
                .saturating_sub(1);
            self.place = wrap::Place { line, row };
            self.sub = 0.;
            // A few rows of what comes before, above it.
            self.scroll_wrapped(-f64::from(self.height() / 3.));
            return;
        }
        // Open what hides it.
        let hiding: Vec<u64> = self
            .folds
            .0
            .range(..line)
            .filter(|(_, end)| **end >= line)
            .map(|(start, _)| *start)
            .collect();
        for start in hiding {
            self.folds.unfold(start);
        }
        let line_h = f64::from(self.line_h());
        let row = self.folds.row_of(line) as f64;
        self.set_position(row * line_h - f64::from(self.height()) / 2. + line_h / 2.);
        // Sideways, as far as it takes to see it whole.
        let depth = self
            .shown()
            .read_in(line..line + 1, self.hex())
            .first()
            .map_or(0, |l| l.depth);
        let indent = depth.min(INDENT_MAX) as f32 * INDENT_W * self.scale;
        let wide = |s: &str| unicode_width::UnicodeWidthStr::width(s) as f32 * self.char_w;
        let (start, end) = (
            crate::select::floor(&text, hit.range.start),
            crate::select::floor(&text, hit.range.end),
        );
        let x0 = indent + wide(&text[..start]);
        let x1 = indent + wide(&text[..end]);
        if x1 > self.widest.get() {
            self.widest.set(x1);
        }
        let view = self.text_w();
        let margin = 2. * self.char_w;
        if x0 < self.scroll_x {
            self.scroll_x = (x0 - margin).max(0.);
        } else if x1 > self.scroll_x + view {
            self.scroll_x = (x1 - view + margin).min(self.max_x());
        }
    }

    /// A table's cell found: its row scrolled to, and its column sideways.
    fn show_found_cell(&mut self, hit: &Hit) {
        let (row, col) = self.cell_of(hit.at);
        // The head stays at the top; the list holds the rows after it.
        if row > 0 {
            self.table_scroll
                .scroll_to_item(row - 1, ScrollStrategy::Center);
        }
        let Some(table) = &self.table else {
            return;
        };
        let scale = self.scale;
        let digits = table.rows().to_string().len();
        let number_w = digits as f32 * self.table_char_w + 16.;
        let width = |i: usize| {
            table
                .widths
                .get(i)
                .map_or(0., |w| *w as f32 * self.table_char_w + 14.)
        };
        let x0 = (number_w + (0..col).map(width).sum::<f32>()) * scale;
        let x1 = x0 + width(col) * scale;
        let handle = &self.table_bar.handle;
        let view = f32::from(handle.bounds().size.width);
        let now = -f32::from(handle.offset().x);
        let to = if x0 < now {
            x0 - 20.
        } else if x1 > now + view {
            x1 - view + 20.
        } else {
            return;
        };
        let max = f32::from(handle.max_offset().x);
        let y = handle.offset().y;
        handle.set_offset(point(px(-to.clamp(0., max.max(0.))), y));
    }
}
