//! rex-run -- Chrome's offline dinosaur, in a terminal.

use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::{cursor, execute, style, terminal};

use rex_run::game::{self, Game, Rng, View, MIN_COLS, MIN_ROWS};
use rex_run::render::Screen;
use rex_run::score;

const FRAME: Duration = Duration::from_micros(16_667);
/// A stalled terminal must not teleport the world past an obstacle.
const MAX_DT: f64 = 50.0;

const HELP: &str = "\
rex-run -- Chrome's offline dinosaur, in your terminal.

USAGE:
    rex-run [OPTIONS]

OPTIONS:
    -s, --slow       Chrome's slow mode: gentler acceleration, wider gaps
    -h, --help       Print this help
    -V, --version    Print version

CONTROLS:
    space, up        jump (hold for a higher one)
    down             duck, or drop out of a jump
    enter, space     restart after a crash
    q, esc, ctrl-c   quit
";

/// Whether the kitty keyboard protocol was pushed, so it can be popped.
static ENHANCED_KEYS: AtomicBool = AtomicBool::new(false);

fn main() {
    let mut config = &game::NORMAL;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => return print!("{HELP}"),
            "-V" | "--version" => return println!("rex-run {}", env!("CARGO_PKG_VERSION")),
            "-s" | "--slow" => config = &game::SLOW,
            other => {
                eprint!("rex-run: unrecognised option `{other}`\n\n{HELP}");
                std::process::exit(2);
            }
        }
    }

    if !io::stdout().is_terminal() {
        eprintln!("rex-run: stdout is not a terminal");
        std::process::exit(1);
    }

    if let Err(err) = run(config) {
        leave();
        eprintln!("rex-run: {err}");
        std::process::exit(1);
    }
}

fn run(config: &'static game::Config) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let mut screen = Screen::new(cols, rows);
    let mut game = Game::new(
        View::new(cols, rows),
        config,
        score::load(),
        Rng::from_clock(),
    );

    // Leave the terminal usable even if something panics mid-frame.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        leave();
        hook(info);
    }));

    enter()?;
    game.auto_release = !ENHANCED_KEYS.load(Ordering::Relaxed);
    let result = play(&mut game, &mut screen);
    leave();
    result
}

fn enter() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;

    // Terminals that speak the kitty keyboard protocol can report key
    // releases, which is what makes short hops and quick ducks possible.
    if matches!(terminal::supports_keyboard_enhancement(), Ok(true))
        && execute!(
            out,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        )
        .is_ok()
    {
        ENHANCED_KEYS.store(true, Ordering::Relaxed);
    }
    Ok(())
}

fn leave() {
    let mut out = io::stdout();
    if ENHANCED_KEYS.swap(false, Ordering::Relaxed) {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        out,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    );
    let _ = terminal::disable_raw_mode();
    let _ = out.flush();
}

fn play(game: &mut Game, screen: &mut Screen) -> io::Result<()> {
    let mut out = io::stdout();
    let mut last = Instant::now();

    loop {
        let now = Instant::now();
        let dt = ((now - last).as_secs_f64() * 1000.0).min(MAX_DT);
        last = now;

        if game.view.cols < MIN_COLS || game.view.rows < MIN_ROWS {
            screen.clear();
            screen.text_centered(
                game.view.rows / 2,
                &format!("terminal too small — need {MIN_COLS}x{MIN_ROWS}"),
            );
        } else {
            game.update(dt);
            game.draw(screen);
        }
        screen.present(&mut out, dt)?;

        if let Some(high) = game.take_high_score() {
            score::save(high);
        }

        // Spend whatever is left of the frame waiting for input, so keys
        // land on the very next update.
        let mut budget = FRAME.saturating_sub(now.elapsed());
        while event::poll(budget)? {
            match event::read()? {
                Event::Key(key) if !handle_key(game, key) => return Ok(()),
                Event::Resize(cols, rows) => {
                    game.resize(cols, rows);
                    screen.resize(cols, rows);
                }
                _ => {}
            }
            budget = FRAME.saturating_sub(now.elapsed());
            if budget.is_zero() {
                break;
            }
        }
    }
}

/// Returns false when the player wants out.
fn handle_key(game: &mut Game, key: KeyEvent) -> bool {
    let pressed = key.kind != KeyEventKind::Release;

    match key.code {
        KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,
        KeyCode::Char('q') | KeyCode::Esc => return false,
        KeyCode::Char(' ') | KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('k') => {
            if pressed {
                game.jump_press();
            } else {
                game.jump_release();
            }
        }
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') => {
            if pressed {
                game.duck_press();
            } else {
                game.duck_release();
            }
        }
        KeyCode::Enter if pressed => game.restart_press(),
        _ => {}
    }
    true
}
