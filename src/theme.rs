//! Color themes, ported from the `minesweeper-tui` palette.
//!
//! Each theme carries a general UI palette (background, foreground, text,
//! accent, secondary, border, error) plus a game-cell palette (closed tile,
//! cursor/selection, adjacent-cell highlight, mine, flag, open tile, and the
//! eight per-number "danger" colors).

use std::sync::OnceLock;

use ratatui::style::Color;

pub const THEME_COUNT: usize = 8;

pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub text: Color,
    pub accent: Color,
    pub secondary: Color,
    pub border: Color,
    pub error: Color,
    pub close: Color,
    pub selected: Color,
    pub adjacent: Color,
    pub mine: Color,
    pub flag: Color,
    pub open: Color,
    pub dangers: [Color; 8],
}

fn hex(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).expect("bad hex");
    let g = u8::from_str_radix(&s[2..4], 16).expect("bad hex");
    let b = u8::from_str_radix(&s[4..6], 16).expect("bad hex");
    Color::Rgb(r, g, b)
}

#[allow(clippy::too_many_arguments)]
fn th(
    name: &'static str,
    bg: &str,
    fg: &str,
    text: &str,
    accent: &str,
    secondary: &str,
    border: &str,
    error: &str,
    close: &str,
    selected: &str,
    adjacent: &str,
    mine: &str,
    flag: &str,
    open: &str,
    dangers: [&str; 8],
) -> Theme {
    Theme {
        name,
        bg: hex(bg),
        fg: hex(fg),
        text: hex(text),
        accent: hex(accent),
        secondary: hex(secondary),
        border: hex(border),
        error: hex(error),
        close: hex(close),
        selected: hex(selected),
        adjacent: hex(adjacent),
        mine: hex(mine),
        flag: hex(flag),
        open: hex(open),
        dangers: dangers.map(hex),
    }
}

static THEMES: OnceLock<[Theme; THEME_COUNT]> = OnceLock::new();

pub fn theme(index: usize) -> &'static Theme {
    let themes = THEMES.get_or_init(|| {
        [
            th(
                "Catppuccin",
                "1E1E2E", "585B70", "CDD6F4", "89B4FA", "F5C2E7", "313244", "F38BA8",
                "313244", "89B4FA", "45475A", "F38BA8", "A6E3A1", "1E1E2E",
                ["A6E3A1", "94E2D5", "F9E2AF", "FAB387", "EBA0AC", "F38BA8", "CBA6F7", "F5C2E7"],
            ),
            th(
                "Everforest",
                "2B3339", "50585D", "D3C6AA", "A7C080", "DBBC7F", "343F44", "E67E80",
                "4A555B", "7FBBB3", "525C62", "E67E80", "A7C080", "2D353B",
                ["A7C080", "83C092", "DBBC7F", "E69875", "E67E80", "D699B6", "7FBBB3", "D3C6AA"],
            ),
            th(
                "Gruvbox",
                "282828", "665C54", "EBDBB2", "FABD2F", "83A598", "3C3836", "FB4934",
                "504945", "83A598", "665C54", "FB4934", "B8BB26", "282828",
                ["B8BB26", "8EC07C", "FABD2F", "FE8019", "FB4934", "CC241D", "D3869B", "B16286"],
            ),
            th(
                "Rosepine",
                "191724", "524F67", "E0DEF4", "EBBCBA", "31748F", "26233A", "EB6F92",
                "26233A", "9CCFD8", "31748F", "EB6F92", "F6C177", "191724",
                ["9CCFD8", "31748F", "F6C177", "EBBCBA", "EB6F92", "B4637A", "C4A7E7", "E0DEF4"],
            ),
            th(
                "Tokyonight",
                "1A1B26", "414868", "A9B1D6", "7AA2F7", "BB9AF7", "24283B", "F7768E",
                "292E42", "7AA2F7", "3B4261", "F7768E", "9ECE6B", "1A1B26",
                ["9ECE6A", "73DACA", "E0AF68", "FF9E64", "F7768E", "FF007C", "BB9AF7", "C0CAF5"],
            ),
            th(
                "Nord",
                "2E3440", "4C566A", "D8DEE9", "88C0D0", "81A1C1", "3B4252", "BF616A",
                "434C5E", "88C0D0", "4C566A", "BF616A", "A3BE8D", "2E3440",
                ["A3BE8C", "8FBCBB", "EBCB8B", "D08770", "BF616A", "B48EAD", "88C0D0", "81A1C1"],
            ),
            th(
                "Monokai",
                "272822", "49483E", "F8F8F2", "A6E22E", "FD971F", "3E3D32", "F92672",
                "49483E", "66D9EF", "75715E", "F92672", "A6E22F", "272822",
                ["A6E22E", "66D9EF", "E6DB74", "FD971F", "F92672", "AE81FF", "F8F8F2", "75715E"],
            ),
            th(
                "Dracula",
                "282A36", "44475A", "F8F8F2", "BD93F9", "FF79C6", "343746", "FF5555",
                "44475A", "8BE9FD", "6272A4", "FF5555", "50FA7C", "282A36",
                ["50FA7B", "8BE9FD", "F1FA8C", "FFB86C", "FF79C6", "FF5555", "BD93F9", "6272A4"],
            ),
        ]
    });
    &themes[index % THEME_COUNT]
}
