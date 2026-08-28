//! Rendering: menu, game board, statistics, and modal overlays, all themed.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Overlay, Screen, CELL_H, CELL_W, DIFFICULTIES};
use crate::game::{Cell, CellState, Pos, Status};
use crate::state::StatItem;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let bg = app.theme().bg;

    match app.screen {
        Screen::Menu => draw_menu(app, frame),
        Screen::Game => draw_game(app, frame),
        Screen::Stats => draw_stats(app, frame),
    }

    match &app.overlay {
        Overlay::None => {}
        Overlay::Help => draw_help(app, frame),
        Overlay::Win => draw_win(app, frame),
        Overlay::FallbackNotice => draw_fallback_notice(app, frame),
        Overlay::CustomInput { buf, error } => {
            let buf = buf.clone();
            let error = error.clone();
            draw_custom_input(app, frame, &buf, error.as_deref());
        }
    }

    fill_background(frame, area, bg);
}

/// Fill any cell with a transparent background with the theme background so the
/// whole terminal adopts the theme, regardless of what widgets reset.
fn fill_background(frame: &mut Frame, area: Rect, bg: Color) {
    let buf = frame.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y))
                && cell.bg == Color::Reset
            {
                cell.bg = bg;
            }
        }
    }
}

fn draw_menu(app: &App, frame: &mut Frame) {
    let t = app.theme();
    let mut lines = vec![Line::from("")];
    for (i, d) in DIFFICULTIES.iter().enumerate() {
        let selected = i == app.menu_selected;
        let marker = if selected { "▶" } else { " " };
        let style = if selected {
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} "), style),
            Span::styled(format!("{:<13}", d.name()), style),
            Span::styled(d.describe(), Style::default().fg(t.fg)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Theme ", Style::default().fg(t.fg)),
        Span::styled(
            t.name,
            Style::default().fg(t.secondary).add_modifier(Modifier::BOLD),
        ),
        Span::styled("   < / > change ", Style::default().fg(t.fg)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " enter play · s statistics · q quit ",
        Style::default().fg(t.fg),
    )));

    let width = 46;
    let height = lines.len() as u16 + 2;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border))
        .title(Span::styled(
            " Minesweeper ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    let rect = centered_rect(frame.area(), width, height);
    frame.render_widget(block, rect);
    frame.render_widget(
        Paragraph::new(lines),
        Block::default().borders(Borders::ALL).inner(rect),
    );
}

fn draw_game(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(0),    // board
        Constraint::Length(1), // footer
    ])
    .split(area);

    draw_header(app, frame, chunks[0]);
    draw_board(app, frame, chunks[1]);
    draw_footer(app, frame, chunks[2]);
}

fn draw_header(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border))
        .title(Span::styled(
            " Minesweeper ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(block, area);

    let inner = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .split(Block::default().borders(Borders::ALL).inner(area));

    let rem = app.game.mines_remaining();
    let mines = Line::from(vec![
        Span::styled("Mines ", Style::default().fg(t.fg)),
        Span::styled(
            format!("{:03}", rem),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(mines).alignment(Alignment::Left), inner[0]);

    let (status, color) = status_text(app);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            status,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        inner[1],
    );

    let secs = app.elapsed_time().as_secs().min(999);
    let time = Line::from(vec![
        Span::styled("Time ", Style::default().fg(t.fg)),
        Span::styled(
            format!("{:03}", secs),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(time).alignment(Alignment::Right), inner[2]);
}

fn status_text(app: &App) -> (&'static str, Color) {
    let t = app.theme();
    match app.game.status {
        Status::Ready => ("READY", t.fg),
        Status::Playing => ("PLAYING", t.accent),
        Status::Won => ("WON", t.flag),
        Status::Lost => ("LOST", t.error),
    }
}

fn draw_board(app: &mut App, frame: &mut Frame, area: Rect) {
    let t = app.theme();
    let board_w = app.game.width as u16 * CELL_W + 2;
    let board_h = app.game.height as u16 * CELL_H + 2;
    let rect = centered_rect(area, board_w, board_h);

    let title = format!(" {} ", app.difficulty.name());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border))
        .title(Span::styled(title, Style::default().fg(t.fg)));
    frame.render_widget(block, rect);

    let inner = Block::default().borders(Borders::ALL).inner(rect);
    app.board_origin = (inner.x, inner.y);

    let mut lines = Vec::with_capacity(app.game.height);
    for y in 0..app.game.height {
        let mut spans = Vec::with_capacity(app.game.width);
        for x in 0..app.game.width {
            spans.push(cell_span(app, x, y));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn is_neighbor(x1: usize, y1: usize, x2: usize, y2: usize) -> bool {
    let dx = (x1 as isize - x2 as isize).abs();
    let dy = (y1 as isize - y2 as isize).abs();
    dx <= 1 && dy <= 1 && !(dx == 0 && dy == 0)
}

/// Render one cell as a single row of two columns.
fn cell_span(app: &App, x: usize, y: usize) -> Span<'static> {
    let g = &app.game;
    let t = app.theme();
    let is_cursor = app.cursor == Pos { x, y };

    // Adjacent-cell highlight preview: the cursor sits on a revealed number.
    let cursor_on_number = matches!(g.cell(app.cursor.x, app.cursor.y), Cell::Empty(n) if n > 0)
        && g.state(app.cursor.x, app.cursor.y) == CellState::Revealed
        && g.status == Status::Playing;
    let is_adjacent = cursor_on_number
        && g.state(x, y) == CellState::Hidden
        && is_neighbor(app.cursor.x, app.cursor.y, x, y);

    let (ch, mut style) = match g.state(x, y) {
        CellState::Hidden => (' ', Style::new().bg(t.close)),
        CellState::Flagged => (
            'F',
            Style::new()
                .bg(t.flag)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        CellState::Revealed => match g.cell(x, y) {
            Cell::Mine => {
                let exploded = g.lost_at == Some(Pos { x, y });
                if exploded {
                    ('*', Style::new().bg(t.error).fg(Color::White).add_modifier(Modifier::BOLD))
                } else {
                    ('*', Style::new().bg(t.mine).fg(Color::Black))
                }
            }
            Cell::Empty(0) => (' ', Style::new().bg(t.open)),
            Cell::Empty(n) => (
                (b'0' + n) as char,
                Style::new()
                    .bg(t.open)
                    .fg(t.dangers[n as usize - 1])
                    .add_modifier(Modifier::BOLD),
            ),
        },
    };

    if is_adjacent {
        style = style.bg(t.adjacent);
    }
    if is_cursor {
        style = Style::new()
            .bg(t.selected)
            .fg(t.bg)
            .add_modifier(Modifier::BOLD);
    }

    Span::styled(format!("{ch} "), style)
}

fn draw_footer(app: &App, frame: &mut Frame, area: Rect) {
    let t = app.theme();
    let text = format!(
        " seed {} · {} · space reveal · f flag · a chord/flag · r restart · n menu · s stats · < > theme · q quit ",
        app.game.seed,
        t.name,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(text, Style::default().fg(t.fg))).alignment(Alignment::Center),
        area,
    );
}

fn draw_stats(app: &App, frame: &mut Frame) {
    let t = app.theme();
    let header = Style::default().fg(t.fg).add_modifier(Modifier::BOLD);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{:<13}", "Difficulty"), header),
            Span::styled(format!("{:>5}", "Wins"), header),
            Span::styled(format!("{:>7}", "Games"), header),
            Span::styled(format!("{:>9}", "Best"), header),
            Span::styled(format!("{:>9}", "Avg (W)"), header),
        ]),
    ];
    for d in DIFFICULTIES {
        let s = app.stats.item(d);
        lines.push(Line::from(vec![
            Span::styled(format!("{:<13}", d.name()), Style::default().fg(t.text)),
            Span::styled(format!("{:>5}", s.wins), Style::default().fg(t.accent)),
            Span::styled(format!("{:>7}", s.total), Style::default().fg(t.text)),
            Span::styled(format!("{:>9}", best_str(s.best_time)), Style::default().fg(t.secondary)),
            Span::styled(format!("{:>9}", avg_str(s)), Style::default().fg(t.fg)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " esc / q back ",
        Style::default().fg(t.fg),
    )));

    let width = 48;
    let height = lines.len() as u16 + 2;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border))
        .title(Span::styled(
            " Statistics ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    let rect = centered_rect(frame.area(), width, height);
    frame.render_widget(block, rect);
    frame.render_widget(
        Paragraph::new(lines),
        Block::default().borders(Borders::ALL).inner(rect),
    );
}

fn draw_help(app: &App, frame: &mut Frame) {
    let t = app.theme();
    let lines = vec![
        Line::from(Span::styled(
            " Controls ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  ← ↑ ↓ →  /  h j k l               move cursor"),
        Line::from("  f                                toggle flag"),
        Line::from("  a                                chord revealed · flag covered"),
        Line::from("  r                                new game (same difficulty)"),
        Line::from("  n                                difficulty menu"),
        Line::from("  s                                statistics"),
        Line::from("  < / >                            cycle theme"),
        Line::from("  ? / esc                          close this help"),
        Line::from("  q / ctrl-c                       quit"),
        Line::from(""),
        Line::from("  mouse: left reveal · right flag · middle chord"),
        Line::from(""),
        Line::from(Span::styled(
            " Fairness ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from("  Boards aim to be no-guess after the opening."),
        Line::from("  If search gives up, you get a one-time notice"),
        Line::from("  that this layout may require guesses."),
        Line::from("  (Dense Custom falls back more often than presets.)"),
    ];
    let width = 48;
    let height = lines.len() as u16 + 2;
    let popup = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));
    let area = centered_rect(frame.area(), width, height);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        Block::default().borders(Borders::ALL).inner(area),
    );
}

fn draw_fallback_notice(app: &App, frame: &mut Frame) {
    let t = app.theme();
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Fallback layout ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  No-guess search gave up on this seed."),
        Line::from("  This board may require guesses."),
        Line::from(""),
        Line::from(Span::styled(
            "  esc / enter continue · r new game ",
            Style::default().fg(t.fg),
        )),
    ];
    let width = 44;
    let height = lines.len() as u16 + 2;
    let popup = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));
    let area = centered_rect(frame.area(), width, height);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        Block::default().borders(Borders::ALL).inner(area),
    );
}

fn draw_win(app: &App, frame: &mut Frame) {
    let t = app.theme();
    let secs = app.elapsed_time().as_secs();
    let best = app.stats.item(app.difficulty).best_time;

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " You Win! ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Time   ", Style::default().fg(t.fg)),
            Span::styled(fmt_dur(secs), Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Board  ", Style::default().fg(t.fg)),
            Span::styled(app.difficulty.describe(), Style::default().fg(t.text)),
        ]),
    ];
    if let Some(b) = best {
        lines.push(Line::from(vec![
            Span::styled("  Best   ", Style::default().fg(t.fg)),
            Span::styled(
                fmt_dur(b),
                Style::default().fg(t.secondary).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  r play again · n new game · esc continue ",
        Style::default().fg(t.fg),
    )));

    let width = 44;
    let height = lines.len() as u16 + 2;
    let popup = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));
    let area = centered_rect(frame.area(), width, height);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    frame.render_widget(
        Paragraph::new(lines),
        Block::default().borders(Borders::ALL).inner(area),
    );
}

fn draw_custom_input(app: &App, frame: &mut Frame, buf: &str, error: Option<&str>) {
    let t = app.theme();
    let mut lines = vec![
        Line::from(Span::styled(
            " Custom board ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Enter: width height mines   (e.g. 20 20 60)"),
        Line::from(""),
        Line::from(Span::styled(
            format!("  > {buf}_"),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
    ];
    if let Some(err) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {err}"),
            Style::default().fg(t.error),
        )));
    }

    let width = 44;
    let height = lines.len() as u16 + 2;
    let popup = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));
    let area = centered_rect(frame.area(), width, height);
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
    frame.render_widget(
        Paragraph::new(lines),
        Block::default().borders(Borders::ALL).inner(area),
    );
}

fn fmt_dur(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m{:02}s", secs / 60, secs % 60)
    }
}

fn best_str(best: Option<u64>) -> String {
    best.map(fmt_dur).unwrap_or_else(|| "--".to_string())
}

fn avg_str(s: &StatItem) -> String {
    if s.wins == 0 {
        "--".to_string()
    } else {
        fmt_dur(s.win_duration / s.wins as u64)
    }
}

fn centered_rect(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Difficulty;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(app, f)).unwrap();
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area().height {
            for x in 0..buf.area().width {
                out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn menu_renders_difficulties() {
        let mut app = App::new(crate::state::PersistentState::default());
        let text = render(&mut app, 60, 20);
        assert!(text.contains("Minesweeper"), "title missing");
        assert!(text.contains("Beginner"), "difficulty list missing");
        assert!(text.contains("Theme"), "theme line missing");
    }

    #[test]
    fn game_renders_board_with_numbers_after_reveal() {
        let mut app = App::new(crate::state::PersistentState::default());
        app.start_game(Difficulty::Beginner, 42);
        app.reveal(4, 4);
        let text = render(&mut app, 60, 24);
        assert!(text.chars().any(|c| c.is_ascii_digit()), "revealed numbers missing");
    }

    #[test]
    fn win_shows_overlay_and_records_stats() {
        let mut app = App::new(crate::state::PersistentState::default());
        app.start_game(
            Difficulty::Custom {
                width: 3,
                height: 3,
                mines: 0,
            },
            1,
        );
        app.reveal(1, 1);
        assert!(matches!(app.overlay, Overlay::Win), "win overlay expected");
        assert_eq!(app.stats.item(app.difficulty).wins, 1);
        assert!(app.stats.item(app.difficulty).best_time.is_some());
        let text = render(&mut app, 60, 20);
        assert!(text.contains("Win"), "win overlay text missing");
    }

    #[test]
    fn stats_screen_renders() {
        let mut app = App::new(crate::state::PersistentState::default());
        app.open_stats();
        let text = render(&mut app, 60, 20);
        assert!(text.contains("Statistics"), "statistics title missing");
        assert!(text.contains("Difficulty"), "stats header missing");
        assert!(text.contains("Avg (W)"), "average win column missing");
    }

    #[test]
    fn avg_win_time_shows_dash_when_no_wins() {
        let s = StatItem {
            wins: 0,
            total: 3,
            win_duration: 0,
            best_time: None,
        };
        assert_eq!(avg_str(&s), "--");
    }

    #[test]
    fn avg_win_time_averages_recorded_win_durations() {
        let s = StatItem {
            wins: 2,
            total: 5,
            win_duration: 150,
            best_time: Some(60),
        };
        assert_eq!(avg_str(&s), "1m15s");
    }
}
