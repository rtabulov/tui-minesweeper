//! TUI Minesweeper — entry point, CLI parsing, terminal lifecycle, event loop.

mod app;
mod game;
mod state;
mod theme;
mod ui;

use std::io;
use std::io::IsTerminal;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind, MouseEvent};
use ratatui::DefaultTerminal;

use app::{App, Difficulty};

struct Cli {
    difficulty: Difficulty,
    seed: u64,
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
         \x20 --help                             show this help\n\
         \n\
         In-game: ? shows key bindings; n opens the difficulty menu."
    );
}

fn parse_args() -> Cli {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut difficulty: Option<Difficulty> = None;
    let mut width: Option<usize> = None;
    let mut height: Option<usize> = None;
    let mut mines: Option<usize> = None;
    let mut seed: Option<u64> = None;

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
            "--difficulty" => {
                i += 1;
                difficulty = match args.get(i).map(String::as_str) {
                    Some("easy" | "beginner") => Some(Difficulty::Beginner),
                    Some("medium" | "intermediate") => Some(Difficulty::Intermediate),
                    Some("hard" | "expert") => Some(Difficulty::Expert),
                    _ => {
                        eprintln!("error: unknown --difficulty (expected easy|medium|hard)");
                        std::process::exit(2);
                    }
                };
            }
            other => {
                eprintln!("error: unknown option `{other}` (try --help)");
                std::process::exit(2);
            }
        }
        i += 1;
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

    Cli { difficulty, seed }
}

fn run_app(terminal: &mut DefaultTerminal, cli: Cli) -> io::Result<()> {
    let mut state = state::load();

    // A difficulty (or custom size) given on the CLI starts a game right away,
    // overriding the persisted difficulty; otherwise show the difficulty menu.
    let cli_difficulty = std::env::args().any(|a| {
        matches!(
            a.as_str(),
            "--difficulty" | "--width" | "--height" | "--mines"
        )
    });

    if cli_difficulty {
        state.difficulty = cli.difficulty;
    }

    let mut app = App::new(state);
    if cli_difficulty {
        app.start_game(cli.difficulty, cli.seed);
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
    if !io::stdout().is_terminal() {
        eprintln!("error: stdout is not a terminal; tui-minesweeper is an interactive TUI");
        std::process::exit(1);
    }

    let cli = parse_args();

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
