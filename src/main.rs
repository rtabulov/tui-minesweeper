//! TUI Minesweeper — entry point, CLI parsing, terminal lifecycle, event loop.

mod app;
mod game;
mod share;
mod solver;
mod state;
mod theme;
mod ui;

use std::io;
use std::io::IsTerminal;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind, MouseEvent};
use ratatui::DefaultTerminal;

use app::{App, Difficulty};
use game::Status;
use share::ShareError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    difficulty: Difficulty,
    seed: u64,
    /// When set, auto-start via Share string (exclusive with seed/difficulty/dims).
    share: Option<String>,
    /// Difficulty or custom dims were given → skip Menu and start a game.
    auto_start: bool,
}

fn print_help() {
    println!(
        "tui-minesweeper — a terminal minesweeper\n\
         \n\
         USAGE:\n\
         \x20 tui-minesweeper [OPTIONS]\n\
         \n\
         OPTIONS:\n\
         \x20 --difficulty <easy|medium|hard>   preset board\n\
         \x20 --width <N>                        custom width\n\
         \x20 --height <N>                       custom height\n\
         \x20 --mines <N>                        custom mine count\n\
         \x20 --seed <N>                         fixed RNG seed\n\
         \x20 --share <STRING>                   launch from Share string\n\
         \x20 --help                             show this help\n\
         \n\
         `--share` cannot be combined with `--seed` or difficulty/dims flags.\n\
         \n\
         Boards aim to be no-guess after the opening click. If search\n\
         cannot find one in budget, you get a one-time fallback notice.\n\
         \n\
         In-game: ? shows key bindings; n opens the difficulty menu."
    );
}

/// Parse CLI args. Returns usage errors as `Err` (caller exits non-zero).
/// `--help` prints and exits 0.
fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut difficulty: Option<Difficulty> = None;
    let mut width: Option<usize> = None;
    let mut height: Option<usize> = None;
    let mut mines: Option<usize> = None;
    let mut seed: Option<u64> = None;
    let mut share: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--width" => {
                i += 1;
                width = args.get(i).and_then(|v| v.parse().ok());
            }
            "--height" => {
                i += 1;
                height = args.get(i).and_then(|v| v.parse().ok());
            }
            "--mines" => {
                i += 1;
                mines = args.get(i).and_then(|v| v.parse().ok());
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|v| v.parse().ok());
            }
            "--share" => {
                i += 1;
                let value = args
                    .get(i)
                    .cloned()
                    .ok_or_else(|| "error: --share needs a Share string".to_string())?;
                if value.starts_with('-') {
                    return Err("error: --share needs a Share string".into());
                }
                share = Some(value);
            }
            "--difficulty" => {
                i += 1;
                difficulty = match args.get(i).map(String::as_str) {
                    Some("easy" | "beginner") => Some(Difficulty::Beginner),
                    Some("medium" | "intermediate") => Some(Difficulty::Intermediate),
                    Some("hard" | "expert") => Some(Difficulty::Expert),
                    _ => {
                        return Err(
                            "error: unknown --difficulty (expected easy|medium|hard)".into(),
                        );
                    }
                };
            }
            other => {
                return Err(format!("error: unknown option `{other}` (try --help)"));
            }
        }
        i += 1;
    }

    let saw_dims = args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--width" | "--height" | "--mines" | "--difficulty"
        )
    });
    let saw_seed = args.iter().any(|a| a == "--seed");

    if share.is_some() && (saw_seed || saw_dims) {
        return Err(
            "error: --share cannot be combined with --seed or difficulty/dims flags".into(),
        );
    }

    let seed = seed.unwrap_or_else(rand::random::<u64>);

    let difficulty = if width.is_some() || height.is_some() || mines.is_some() {
        Difficulty::Custom {
            width: width.unwrap_or(16),
            height: height.unwrap_or(16),
            mines: mines.unwrap_or(40),
        }
    } else {
        difficulty.unwrap_or(Difficulty::Beginner)
    };

    Ok(Cli {
        difficulty,
        seed,
        share,
        auto_start: saw_dims,
    })
}

fn parse_args() -> Cli {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_cli(&args) {
        Ok(cli) => cli,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    }
}

/// Apply CLI startup: Menu, difficulty auto-start, or Share launch.
fn apply_startup(app: &mut App, cli: &Cli) -> Result<(), ShareError> {
    if let Some(ref share) = cli.share {
        app.launch_from_share(share)?;
        return Ok(());
    }
    if cli.auto_start {
        app.start_game(cli.difficulty, cli.seed);
    }
    Ok(())
}

fn run_app(terminal: &mut DefaultTerminal, cli: Cli) -> io::Result<()> {
    let mut state = state::load();

    if cli.auto_start {
        state.difficulty = cli.difficulty;
    }

    let mut app = App::new(state);
    if let Err(e) = apply_startup(&mut app, &cli) {
        eprintln!("error: {e}");
        std::process::exit(2);
    }

    loop {
        terminal.draw(|frame| ui::draw(&mut app, frame))?;

        if event::poll(Duration::from_millis(40))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.on_key(key.code, key.modifiers);
                }
                Event::Mouse(mouse) => on_mouse(&mut app, mouse),
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn on_mouse(app: &mut App, mouse: MouseEvent) {
    app.on_mouse(mouse.kind, mouse.column, mouse.row);
}

fn main() -> io::Result<()> {
    // Parse first so `--help` works without a TTY (e.g. `npx @rassul/minesweeper --help`).
    let cli = parse_args();

    if !io::stdout().is_terminal() {
        eprintln!("error: stdout is not a terminal; tui-minesweeper is an interactive TUI");
        std::process::exit(1);
    }

    let mut terminal = match ratatui::try_init() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: could not initialize the terminal: {e}");
            eprintln!("hint: run inside a real terminal (not piped or backgrounded)");
            std::process::exit(1);
        }
    };
    let _ = crossterm::execute!(io::stdout(), event::EnableMouseCapture);

    let result = run_app(&mut terminal, cli);

    let _ = crossterm::execute!(io::stdout(), event::DisableMouseCapture);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::Screen;
    use state::PersistentState;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn share_flag_parses_without_auto_start_dims() {
        let cli = parse_cli(&args(&["--share", "abc123"])).expect("parse");
        assert_eq!(cli.share.as_deref(), Some("abc123"));
        assert!(!cli.auto_start);
    }

    #[test]
    fn share_with_seed_is_usage_error() {
        let err = parse_cli(&args(&["--share", "abc", "--seed", "1"])).unwrap_err();
        assert!(err.contains("--share"), "{err}");
        assert!(err.contains("--seed") || err.contains("difficulty"), "{err}");
    }

    #[test]
    fn share_with_difficulty_is_usage_error() {
        let err = parse_cli(&args(&["--share", "abc", "--difficulty", "easy"])).unwrap_err();
        assert!(err.contains("--share"), "{err}");
    }

    #[test]
    fn share_with_width_is_usage_error() {
        let err = parse_cli(&args(&["--share", "abc", "--width", "9"])).unwrap_err();
        assert!(err.contains("--share"), "{err}");
    }

    #[test]
    fn share_missing_value_is_usage_error() {
        let err = parse_cli(&args(&["--share"])).unwrap_err();
        assert!(err.contains("--share"), "{err}");
    }

    #[test]
    fn without_share_difficulty_still_auto_starts() {
        let cli = parse_cli(&args(&["--difficulty", "hard", "--seed", "99"])).expect("parse");
        assert!(cli.share.is_none());
        assert!(cli.auto_start);
        assert_eq!(cli.difficulty, Difficulty::Expert);
        assert_eq!(cli.seed, 99);
    }

    #[test]
    fn without_share_seed_only_stays_on_menu_path() {
        let cli = parse_cli(&args(&["--seed", "42"])).expect("parse");
        assert!(cli.share.is_none());
        assert!(!cli.auto_start);
        assert_eq!(cli.seed, 42);
    }

    #[test]
    fn apply_startup_share_launches_accepted_board() {
        let mut source = App::new(PersistentState::default());
        source.start_game(Difficulty::Beginner, 12345);
        source.reveal(2, 3);
        let share = source.encode_share().expect("encode");
        let mut layout = Vec::with_capacity(81);
        for y in 0..9 {
            for x in 0..9 {
                layout.push(source.game.cell(x, y));
            }
        }

        let cli = Cli {
            difficulty: Difficulty::Beginner,
            seed: 0,
            share: Some(share),
            auto_start: false,
        };
        let mut app = App::new(PersistentState::default());
        apply_startup(&mut app, &cli).expect("launch");

        assert!(matches!(app.screen, Screen::Game));
        assert_eq!(app.game.status, Status::Playing);
        assert_eq!(app.difficulty, Difficulty::Beginner);
        assert_eq!(app.game.seed, 12345);
        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(app.game.cell(x, y), layout[y * 9 + x]);
            }
        }
    }

    #[test]
    fn apply_startup_bad_share_errors_without_starting_game() {
        let cli = Cli {
            difficulty: Difficulty::Beginner,
            seed: 0,
            share: Some("!!!not-base64!!!".into()),
            auto_start: false,
        };
        let mut app = App::new(PersistentState::default());
        let err = apply_startup(&mut app, &cli).expect_err("bad share");
        assert!(matches!(err, ShareError::Malformed));
        assert!(matches!(app.screen, Screen::Menu));
        assert_eq!(app.game.status, Status::Ready);
    }
}
