//! A QR code, drawn a run of squares at a time. Black on white whatever the
//! theme: that is what a phone's camera reads best.

use gpui::{Bounds, IntoElement, canvas, div, fill, point, prelude::*, px, rgb, size};
use qrcode::{Color, QrCode};

/// The light border the format asks for around the code, in squares.
const QUIET: usize = 4;

/// Which squares are dark, row by row, `width` to a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Squares {
    pub width: usize,
    pub dark: Vec<bool>,
}

impl Squares {
    pub fn of(text: &str) -> Option<Self> {
        let code = QrCode::new(text.as_bytes()).ok()?;
        Some(Self {
            width: code.width(),
            dark: code
                .to_colors()
                .into_iter()
                .map(|c| c == Color::Dark)
                .collect(),
        })
    }

    /// Each run of dark squares in a row: its row, first column, and length.
    pub fn runs(&self) -> Vec<(usize, usize, usize)> {
        let mut runs = Vec::new();
        for (y, row) in self.dark.chunks(self.width).enumerate() {
            let mut x = 0;
            while x < row.len() {
                if row[x] {
                    let start = x;
                    while x < row.len() && row[x] {
                        x += 1;
                    }
                    runs.push((y, start, x - start));
                } else {
                    x += 1;
                }
            }
        }
        runs
    }
}

/// `text` as a code about `side` across, border and all. Squares are whole
/// pixels, so it comes out a little smaller rather than blurred.
pub fn qr_code(text: &str, side: f32) -> Option<impl IntoElement + use<>> {
    let squares = Squares::of(text)?;
    let across = squares.width + 2 * QUIET;
    let cell = (side / across as f32).floor().max(1.);
    let runs = squares.runs();
    Some(
        div()
            .flex_shrink_0()
            .size(px(cell * across as f32))
            .rounded(px(cell * 2.))
            .bg(rgb(0xffffff))
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for &(y, x, length) in &runs {
                            let at = point(
                                origin.x + px((x + QUIET) as f32 * cell),
                                origin.y + px((y + QUIET) as f32 * cell),
                            );
                            let area = size(px(length as f32 * cell), px(cell));
                            window.paint_quad(fill(Bounds::new(at, area), rgb(0x000000)));
                        }
                    },
                )
                .size_full(),
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_fits_a_small_code() {
        let squares = Squares::of("http://192.168.100.200:19390/").unwrap();
        assert!(squares.width <= 29, "{}", squares.width);
        assert_eq!(squares.dark.len(), squares.width * squares.width);
        // The finder pattern's top row: seven dark squares, from the corner.
        assert_eq!(squares.runs()[0], (0, 0, 7));
    }

    /// A phone's camera wants about four pixels a square on a screen, and
    /// the WireGuard configuration is the longest thing drawn as a code
    /// here: 258 bytes comes to 65 squares across. The size it is drawn at
    /// is fixed, so a configuration that grew would not break — it would get
    /// quietly harder to scan, and nothing would say so. This is what says
    /// so.
    #[test]
    fn a_wireguard_configuration_still_scans_at_the_size_it_is_drawn() {
        let ours = tracewing_proxy::wireguard::new_key().unwrap();
        let phone = tracewing_proxy::wireguard::new_key().unwrap();
        let config = tracewing_proxy::wireguard::config_for(
            &ours,
            &phone,
            "192.168.1.13:51820".parse().unwrap(),
        );
        let squares = Squares::of(&config).expect("a code for it");

        // What `qr_code` would work out for the size Settings asks for.
        let side = 300.0f32;
        let across = squares.width + 2 * QUIET;
        let cell = (side / across as f32).floor();
        assert!(
            cell >= 4.0,
            "{} squares across is {cell} pixels a square at {side}: too small to scan",
            squares.width
        );
    }

    #[test]
    fn runs_cover_exactly_the_dark_squares() {
        let squares = Squares {
            width: 4,
            dark: vec![
                true, true, false, true, //
                false, false, false, false, //
                true, true, true, true, //
                false, true, false, false,
            ],
        };
        assert_eq!(squares.runs(), [(0, 0, 2), (0, 3, 1), (2, 0, 4), (3, 1, 1)]);
    }
}
