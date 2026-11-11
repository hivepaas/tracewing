//! Text wrapped to the width of a body view: a line in rows of so many
//! characters. A line of text is at most `body::TEXT_LINE_MAX` bytes, so it
//! has at most that many rows, and a row is told apart from the others of its
//! line by a key that holds both.

/// Bits of a row key that say which row of its line it is.
const ROW_BITS: u32 = 12;
const ROW_MASK: u64 = (1 << ROW_BITS) - 1;

/// Where each row of `text` begins, in bytes, at `cols` characters a row. An
/// empty line still has its one row.
pub fn row_starts(text: &str, cols: usize) -> Vec<usize> {
    let cols = cols.max(1);
    let mut starts = vec![0];
    for (n, (at, _)) in text.char_indices().enumerate() {
        if n > 0 && n % cols == 0 {
            starts.push(at);
        }
    }
    starts
}

/// The key of a row: its line, and which row of it.
pub fn key(line: u64, row: usize) -> u64 {
    (line << ROW_BITS) | (row as u64).min(ROW_MASK)
}

/// The line and the row a key stands for.
pub fn unkey(key: u64) -> (u64, usize) {
    (key >> ROW_BITS, (key & ROW_MASK) as usize)
}

/// The last row a key can name, for a selection that runs to a line's end.
pub const LAST_ROW: usize = ROW_MASK as usize;

/// A place in wrapped text: a line, and a row of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub line: u64,
    pub row: usize,
}

impl Place {
    /// The row after this one, if there is one. `rows` tells how many rows a
    /// line has.
    pub fn next(self, lines: u64, rows: &mut impl FnMut(u64) -> usize) -> Option<Self> {
        if self.row + 1 < rows(self.line) {
            Some(Self {
                row: self.row + 1,
                ..self
            })
        } else if self.line + 1 < lines {
            Some(Self {
                line: self.line + 1,
                row: 0,
            })
        } else {
            None
        }
    }

    /// The row before this one, if there is one.
    pub fn back(self, rows: &mut impl FnMut(u64) -> usize) -> Option<Self> {
        if self.row > 0 {
            Some(Self {
                row: self.row - 1,
                ..self
            })
        } else if self.line > 0 {
            let line = self.line - 1;
            Some(Self {
                line,
                row: rows(line).saturating_sub(1),
            })
        } else {
            None
        }
    }

    /// The highest place the view can start at and still be full: `in_view`
    /// rows back from the last.
    pub fn last_top(lines: u64, in_view: u64, rows: &mut impl FnMut(u64) -> usize) -> Self {
        let Some(last) = lines.checked_sub(1) else {
            return Self::default();
        };
        let mut place = Self {
            line: last,
            row: rows(last).saturating_sub(1),
        };
        for _ in 1..in_view {
            match place.back(rows) {
                Some(before) => place = before,
                None => break,
            }
        }
        place
    }

    /// How far into the text this place is, in lines and a share of one: what
    /// the scroll bar shows when the rows of every line are not known.
    pub fn fraction(self, rows: &mut impl FnMut(u64) -> usize) -> f64 {
        self.line as f64 + self.row as f64 / rows(self.line).max(1) as f64
    }

    /// The place `fraction` lines into the text, as [`Self::fraction`] tells.
    pub fn at_fraction(fraction: f64, lines: u64, rows: &mut impl FnMut(u64) -> usize) -> Self {
        let Some(last) = lines.checked_sub(1) else {
            return Self::default();
        };
        let line = (fraction.max(0.) as u64).min(last);
        let share = (fraction - line as f64).clamp(0., 1.);
        let count = rows(line).max(1);
        Self {
            line,
            // A little over, so that a third of three rows is the second.
            row: ((share * count as f64 + 1e-9) as usize).min(count - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_begin_every_so_many_characters() {
        assert_eq!(row_starts("", 4), [0]);
        assert_eq!(row_starts("abcd", 4), [0]);
        assert_eq!(row_starts("abcdefghi", 4), [0, 4, 8]);
        // Characters, not bytes: "ệ" is three.
        assert_eq!(row_starts("ệệệệệ", 2), [0, 6, 12]);
    }

    #[test]
    fn a_key_keeps_its_line_and_row_in_order() {
        assert_eq!(unkey(key(7, 3)), (7, 3));
        assert!(key(7, 3) < key(7, 4));
        assert!(key(7, LAST_ROW) < key(8, 0));
    }

    #[test]
    fn places_step_over_rows_and_lines() {
        // Three lines of 1, 3 and 2 rows.
        let mut rows = |line: u64| [1, 3, 2][line as usize];
        let start = Place::default();
        let mut seen = vec![start];
        let mut place = start;
        while let Some(next) = place.next(3, &mut rows) {
            seen.push(next);
            place = next;
        }
        assert_eq!(seen.len(), 6);
        assert_eq!(place, Place { line: 2, row: 1 });
        let mut back = 0;
        while let Some(before) = place.back(&mut rows) {
            place = before;
            back += 1;
        }
        assert_eq!((back, place), (5, start));
        assert_eq!(Place::last_top(3, 3, &mut rows), Place { line: 1, row: 2 });
        assert_eq!(Place::last_top(3, 99, &mut rows), start);
        let middle = Place { line: 1, row: 1 };
        let fraction = middle.fraction(&mut rows);
        assert_eq!(Place::at_fraction(fraction, 3, &mut rows), middle);
    }
}
