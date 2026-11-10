//! The list's columns: which are shown, how wide, and what the list is sorted by.
//! What is shown and how wide is kept with the layout; the sort is not, so a new
//! start shows flows in the order they came.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tracewing_i18n::tr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    Number,
    Method,
    Status,
    App,
    Host,
    Path,
    /// A GraphQL request's operation.
    Operation,
    Size,
    Time,
}

/// The narrowest a column is made by dragging, and the widest.
pub const WIDTH_MIN: f32 = 40.;
pub const WIDTH_MAX: f32 = 640.;

impl Column {
    pub const ALL: [Self; 9] = [
        Self::Number,
        Self::Method,
        Self::Status,
        Self::App,
        Self::Host,
        Self::Path,
        Self::Operation,
        Self::Size,
        Self::Time,
    ];

    /// In the header.
    pub fn label(self) -> String {
        match self {
            Self::Number => "#".to_owned(),
            Self::Method => tr!("Method"),
            Self::Status => tr!("Status"),
            Self::App => tr!("App"),
            Self::Host => tr!("Host"),
            Self::Path => tr!("Path"),
            Self::Operation => tr!("Operation"),
            Self::Size => tr!("Size"),
            Self::Time => tr!("Time"),
        }
    }

    /// In the menu, where `#` would say little.
    pub fn name(self) -> String {
        match self {
            Self::Number => tr!("Number"),
            other => other.label(),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Method => "method",
            Self::Status => "status",
            Self::App => "app",
            Self::Host => "host",
            Self::Path => "path",
            Self::Operation => "operation",
            Self::Size => "size",
            Self::Time => "time",
        }
    }

    pub fn default_width(self) -> f32 {
        match self {
            Self::Number => 76.,
            Self::Method => 66.,
            Self::Status => 60.,
            Self::App => 104.,
            Self::Host => 170.,
            // It takes what is left.
            Self::Path => 0.,
            Self::Operation => 150.,
            Self::Size => 74.,
            Self::Time => 70.,
        }
    }

    /// Hidden until asked for: most traffic is not GraphQL, and the path says
    /// the operation as well while this is hidden.
    pub fn opt_in(self) -> bool {
        self == Self::Operation
    }

    /// The path takes the room the others leave; it is always there.
    pub fn fills(self) -> bool {
        self == Self::Path
    }

    /// Numbers line up on the right, and their columns grow to the left.
    pub fn on_the_right(self) -> bool {
        matches!(self, Self::Size | Self::Time)
    }

    /// In a narrow list these give way, down to this, before the path does.
    pub fn gives_way_to(self) -> Option<f32> {
        match self {
            Self::Number => Some(52.),
            Self::App => Some(56.),
            Self::Host => Some(96.),
            Self::Operation => Some(56.),
            _ => None,
        }
    }

    /// What the list is sorted by for this column. Flows that have none, as a
    /// failed one has no status, come first going up.
    pub fn order_by(self) -> &'static str {
        match self {
            Self::Number => "id",
            Self::Method => "method",
            Self::Status => "status",
            Self::App => "app",
            Self::Host => "host",
            Self::Path => "path",
            Self::Operation => "graphql",
            Self::Size => "resp_bytes",
            Self::Time => "duration_us",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sort {
    pub column: Column,
    pub descending: bool,
}

impl Sort {
    /// A click on a column's header: up, then down, then as the flows came.
    pub fn after_click(now: Option<Self>, column: Column) -> Option<Self> {
        match now {
            Some(sort) if sort.column == column && !sort.descending => Some(Self {
                column,
                descending: true,
            }),
            Some(sort) if sort.column == column => None,
            _ => Some(Self {
                column,
                descending: false,
            }),
        }
    }

    pub fn sql(self) -> String {
        let way = if self.descending { "DESC" } else { "ASC" };
        format!("{} {way}, id {way}", self.column.order_by())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Columns {
    hidden: Vec<Column>,
    /// Columns hidden until asked for, which have been.
    shown: Vec<Column>,
    /// Only those changed from their defaults.
    widths: BTreeMap<Column, f32>,
}

impl Columns {
    /// In order, without those hidden.
    pub fn shown(&self) -> Vec<Column> {
        Column::ALL
            .into_iter()
            .filter(|column| self.is_shown(*column))
            .collect()
    }

    pub fn is_shown(&self, column: Column) -> bool {
        if column.opt_in() {
            return self.shown.contains(&column);
        }
        column.fills() || !self.hidden.contains(&column)
    }

    /// Shows a hidden column, or hides a shown one. The path stays.
    pub fn toggle(&mut self, column: Column) {
        if column.fills() {
            return;
        }
        if column.opt_in() {
            match self.shown.iter().position(|c| *c == column) {
                Some(at) => {
                    self.shown.remove(at);
                }
                None => self.shown.push(column),
            }
            return;
        }
        if let Some(at) = self.hidden.iter().position(|c| *c == column) {
            self.hidden.remove(at);
        } else {
            self.hidden.push(column);
        }
    }

    pub fn width(&self, column: Column) -> f32 {
        self.widths
            .get(&column)
            .copied()
            .unwrap_or_else(|| column.default_width())
    }

    pub fn set_width(&mut self, column: Column, width: f32) {
        if column.fills() {
            return;
        }
        let width = width.clamp(WIDTH_MIN, WIDTH_MAX);
        if (width - column.default_width()).abs() < 0.5 {
            self.widths.remove(&column);
        } else {
            self.widths.insert(column, width);
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_sorts_up_then_down_then_not() {
        let up = Sort::after_click(None, Column::Size);
        assert_eq!(
            up,
            Some(Sort {
                column: Column::Size,
                descending: false
            })
        );
        let down = Sort::after_click(up, Column::Size);
        assert!(down.unwrap().descending);
        assert_eq!(Sort::after_click(down, Column::Size), None);
        // Another column starts going up.
        assert_eq!(
            Sort::after_click(down, Column::Host).unwrap().column,
            Column::Host
        );
        assert_eq!(down.unwrap().sql(), "resp_bytes DESC, id DESC");
    }

    #[test]
    fn the_path_is_always_there() {
        let mut columns = Columns::default();
        columns.toggle(Column::Path);
        columns.toggle(Column::App);
        assert!(columns.is_shown(Column::Path));
        assert!(!columns.is_shown(Column::App));
        assert!(!columns.shown().contains(&Column::App));
        columns.toggle(Column::App);
        assert!(columns.is_default());
    }

    #[test]
    fn widths_stay_in_bounds_and_defaults_are_not_kept() {
        let mut columns = Columns::default();
        columns.set_width(Column::Host, 5.);
        assert_eq!(columns.width(Column::Host), WIDTH_MIN);
        columns.set_width(Column::Host, 10_000.);
        assert_eq!(columns.width(Column::Host), WIDTH_MAX);
        columns.set_width(Column::Host, Column::Host.default_width());
        assert!(columns.is_default());
    }

    #[test]
    fn they_are_kept_as_written() {
        let mut columns = Columns::default();
        columns.toggle(Column::Number);
        columns.set_width(Column::Host, 240.);
        let text = toml::to_string(&columns).unwrap();
        assert_eq!(toml::from_str::<Columns>(&text).unwrap(), columns);
        // A file from before columns could be changed.
        assert_eq!(toml::from_str::<Columns>("").unwrap(), Columns::default());
    }
}
