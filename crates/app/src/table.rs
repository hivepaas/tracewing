//! CSV and TSV as a table. Where each record starts is found once, in the
//! background, from the body's file mapped as the viewer maps it; a
//! record's cells are read only when it is drawn.

use memmap2::Mmap;
use std::{fs::File, path::Path, sync::Arc};

/// The most records a table holds; the rest are left out, and that is said.
pub const ROWS_MAX: usize = 1_000_000;
/// The most of a cell drawn.
pub const CELL_SHOWN: usize = 200;
/// Records looked at for how wide each column is.
const WIDTH_SAMPLE: usize = 500;
/// A column's width, in characters, at least and at most.
const WIDTH_MIN: usize = 3;
const WIDTH_MAX: usize = 48;

/// What separates a body's cells, if it is a table: `,` for CSV, a tab for TSV.
pub fn separator_of(content_type: Option<&str>) -> Option<u8> {
    let essence = content_type?.split(';').next()?.trim().to_ascii_lowercase();
    match essence.as_str() {
        "text/csv" | "application/csv" | "text/comma-separated-values" => Some(b','),
        "text/tab-separated-values" | "text/tsv" => Some(b'\t'),
        _ => None,
    }
}

enum Data {
    Map(Mmap),
    Bytes(Vec<u8>),
}

impl Data {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Map(map) => map,
            Self::Bytes(bytes) => bytes,
        }
    }
}

pub struct Table {
    data: Data,
    /// Where each record starts, and after the last, where it ends.
    starts: Vec<usize>,
    separator: u8,
    /// Cells in the widest record looked at.
    pub columns: usize,
    /// Each column's width, in characters.
    pub widths: Vec<usize>,
    /// It had more than [`ROWS_MAX`] records.
    pub cut: bool,
}

impl Table {
    /// The table in the file at `path`, or `None` if it holds nothing.
    pub fn open(path: &Path, separator: u8) -> Option<Arc<Self>> {
        let file = File::open(path).ok()?;
        // SAFETY: the viewer's own copy of the body, which nothing writes
        // while it is shown.
        let map = unsafe { Mmap::map(&file) }.ok()?;
        Self::new(Data::Map(map), separator)
    }

    pub fn from_bytes(bytes: Vec<u8>, separator: u8) -> Option<Arc<Self>> {
        Self::new(Data::Bytes(bytes), separator)
    }

    fn new(data: Data, separator: u8) -> Option<Arc<Self>> {
        let (starts, cut) = record_starts(data.bytes(), separator == b',');
        if starts.len() < 2 {
            return None;
        }
        let mut table = Self {
            data,
            starts,
            separator,
            columns: 0,
            widths: Vec::new(),
            cut,
        };
        for row in 0..table.rows().min(WIDTH_SAMPLE) {
            let cells = table.cells(row);
            if cells.len() > table.widths.len() {
                table.widths.resize(cells.len(), WIDTH_MIN);
            }
            for (width, cell) in table.widths.iter_mut().zip(&cells) {
                *width = (*width).max(cell.chars().count().min(WIDTH_MAX));
            }
        }
        table.columns = table.widths.len();
        Some(Arc::new(table))
    }

    /// Records, the first, which names the columns, among them.
    pub fn rows(&self) -> usize {
        self.starts.len() - 1
    }

    /// The cells of record `row`, unquoted.
    pub fn cells(&self, row: usize) -> Vec<String> {
        let (Some(&from), Some(&to)) = (self.starts.get(row), self.starts.get(row + 1)) else {
            return Vec::new();
        };
        let mut record = &self.data.bytes()[from..to];
        while let [rest @ .., b'\n' | b'\r'] = record {
            record = rest;
        }
        split_record(record, self.separator, self.separator == b',')
    }
}

/// Where each record starts, and where the last ends. With `quoted`, a line
/// break between quotes is in a cell, not between records. True as well if
/// there were more than [`ROWS_MAX`].
fn record_starts(data: &[u8], quoted: bool) -> (Vec<usize>, bool) {
    let mut starts = vec![0];
    let mut in_quotes = false;
    for (at, &byte) in data.iter().enumerate() {
        match byte {
            b'"' if quoted => in_quotes = !in_quotes,
            b'\n' if !in_quotes => {
                if starts.len() > ROWS_MAX {
                    return (starts, true);
                }
                starts.push(at + 1);
            }
            _ => {}
        }
    }
    // A last record with no line break after it.
    if starts.last() != Some(&data.len()) {
        starts.push(data.len());
    }
    (starts, false)
}

/// A record's cells. With `quoted`, as CSV writes them: a cell in quotes may
/// hold the separator, a line break, and a quote written twice.
fn split_record(record: &[u8], separator: u8, quoted: bool) -> Vec<String> {
    let text = String::from_utf8_lossy(record);
    if !quoted {
        return text
            .split(separator as char)
            .map(|cell| cell.to_owned())
            .collect();
    }
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            c if c == separator as char && !in_quotes => cells.push(std::mem::take(&mut cell)),
            c => cell.push(c),
        }
    }
    cells.push(cell);
    cells
}

/// A cell as drawn: on one line, and cut if long.
pub fn shown(cell: &str) -> String {
    let line: String = cell
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    match line.char_indices().nth(CELL_SHOWN) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_known_by_their_type() {
        assert_eq!(separator_of(Some("text/csv; charset=utf-8")), Some(b','));
        assert_eq!(separator_of(Some("Text/TSV")), Some(b'\t'));
        assert_eq!(separator_of(Some("text/tab-separated-values")), Some(b'\t'));
        assert_eq!(separator_of(Some("text/plain")), None);
        assert_eq!(separator_of(None), None);
    }

    #[test]
    fn csv_cells_are_unquoted() {
        let csv = "id,name,note\r\n\
                   1,An,\"Red, blue\"\r\n\
                   2,\"Said \"\"hi\"\"\",\"two\nlines\"\n\
                   3,,\n\
                   4,Dũng,last";
        let table = Table::from_bytes(csv.as_bytes().to_vec(), b',').unwrap();
        assert_eq!(table.rows(), 5);
        assert_eq!(table.cells(0), ["id", "name", "note"]);
        assert_eq!(table.cells(1), ["1", "An", "Red, blue"]);
        assert_eq!(table.cells(2), ["2", "Said \"hi\"", "two\nlines"]);
        assert_eq!(table.cells(3), ["3", "", ""]);
        assert_eq!(table.cells(4), ["4", "Dũng", "last"]);
        assert!(table.cells(5).is_empty());
        assert_eq!(table.columns, 3);
        assert_eq!(table.widths, [3, 9, 9]);
        assert!(!table.cut);
    }

    #[test]
    fn tsv_cells_are_split_at_tabs_alone() {
        let tsv = "a\tb\n\"x,y\"\t\"q\n";
        let table = Table::from_bytes(tsv.as_bytes().to_vec(), b'\t').unwrap();
        assert_eq!(table.rows(), 2);
        assert_eq!(table.cells(1), ["\"x,y\"", "\"q"]);
    }

    #[test]
    fn nothing_is_no_table() {
        assert!(Table::from_bytes(Vec::new(), b',').is_none());
        let one = Table::from_bytes(b"only,a,head".to_vec(), b',').unwrap();
        assert_eq!(one.rows(), 1);
    }

    #[test]
    fn long_cells_are_cut_and_kept_on_a_line() {
        assert_eq!(shown("two\nlines"), "two lines");
        let long = "x".repeat(CELL_SHOWN + 5);
        assert_eq!(shown(&long).chars().count(), CELL_SHOWN + 1);
    }
}
