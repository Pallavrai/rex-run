//! Game state, timing, collisions and the viewport mapping.
//!
//! Everything here is in Chrome's world units -- a 600x150 canvas with a
//! 44x47 T-Rex -- so the physics constants and the collision boxes are the
//! browser's, unmodified. The renderer halves those coordinates, since one
//! terminal cell holds two square pixels.
//!
//! One thing has to bend for a terminal. The world is only as wide as the
//! window -- `cols * 2` units, where Chrome has 600 -- so there is less room
//! to see an obstacle coming. Rather than shrink the world's geometry, which
//! would make jumps unclearable, the simulation clock runs slower by the same
//! ratio ([`View::scale`]): identical world, identical jump arcs, identical
//! scores per obstacle, just more seconds of warning. At 300 columns and up
//! the clock is Chrome's exactly.

use crate::font;
use crate::horizon::Horizon;
use crate::render::Screen;
use crate::trex::{self, Status, Trex};

pub const FPS: f64 = 60.0;
pub const MS_PER_FRAME: f64 = 1000.0 / FPS;
/// Chrome's canvas, and the width its pacing is tuned for.
pub const WORLD_H: f64 = 150.0;
pub const REFERENCE_W: f64 = 600.0;
pub const BOTTOM_PAD: f64 = 10.0;
/// Top of the ground strip.
pub const GROUND_Y: f64 = 127.0;
/// Lowest world row worth showing: below this Chrome's canvas is empty.
const VIEW_BOTTOM: f64 = 140.0;

pub const CLEAR_TIME: f64 = 3000.0;
pub const GAMEOVER_CLEAR_TIME: f64 = 1200.0;
pub const INVERT_FADE_DURATION: f64 = 12000.0;
pub const MAX_CLOUDS: usize = 6;
pub const CLOUD_FREQUENCY: f64 = 0.5;
pub const BG_CLOUD_SPEED: f64 = 0.2;
pub const MAX_OBSTACLE_LENGTH: i64 = 3;
pub const MAX_OBSTACLE_DUPLICATION: usize = 2;
/// Pixels of distance per point of score.
pub const SCORE_COEFFICIENT: f64 = 0.025;
const ACHIEVEMENT_DISTANCE: u32 = 100;
const FLASH_DURATION: f64 = 1000.0 / 4.0;
const FLASH_ITERATIONS: u32 = 3;
const BLINK_TIMING: i64 = 7000;

/// How long a duck key counts as held when the terminal cannot report key
/// releases. Long enough to outlast the auto-repeat delay of a slow keyboard.
///
/// ponytail: crude, but only used as a fallback -- terminals that speak the
/// kitty keyboard protocol get real key-up events instead.
const DUCK_HOLD_MS: f64 = 600.0;

/// Minimum usable terminal, in cells.
pub const MIN_COLS: u16 = 60;
pub const MIN_ROWS: u16 = 24;

/// Chrome's 600x150 canvas, in cells: two world pixels per column, four per
/// row. The play area never grows past this, so a big terminal gets the
/// browser's exact field of view with margins around it rather than a wider
/// world than Chrome ever had.
pub const FULL_COLS: u16 = (REFERENCE_W / 2.0) as u16;
pub const FULL_ROWS: u16 = (WORLD_H / 4.0) as u16 + 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Waiting,
    Running,
    Crashed,
}

/// Chrome's normal and slow presets, merged from `game_config.ts` and the
/// jump configs in `trex.ts`.
pub struct Config {
    pub speed: f64,
    pub acceleration: f64,
    pub max_speed: f64,
    pub gap_coefficient: f64,
    pub invert_distance: u32,
    pub gravity: f64,
    pub min_jump_height: f64,
    pub max_jump_height: f64,
    pub initial_jump_velocity: f64,
    pub drop_velocity: f64,
    pub slow: bool,
}

pub static NORMAL: Config = Config {
    speed: 6.0,
    acceleration: 0.001,
    max_speed: 13.0,
    gap_coefficient: 0.6,
    invert_distance: 700,
    gravity: 0.6,
    min_jump_height: 30.0,
    max_jump_height: 30.0,
    initial_jump_velocity: -10.0,
    drop_velocity: -5.0,
    slow: false,
};

pub static SLOW: Config = Config {
    speed: 4.2,
    acceleration: 0.0005,
    max_speed: 9.0,
    gap_coefficient: 0.3,
    invert_distance: 350,
    gravity: 0.25,
    min_jump_height: 45.0,
    max_jump_height: 50.0,
    initial_jump_velocity: -20.0,
    drop_velocity: -5.0,
    slow: true,
};

impl Config {
    /// Slow mode unlocks obstacles earlier and spaces them further apart.
    pub fn min_speed(&self, speed: f64) -> f64 {
        if self.slow {
            speed / 2.0
        } else {
            speed
        }
    }

    pub fn multiple_speed(&self, speed: f64) -> f64 {
        self.min_speed(speed)
    }

    pub fn min_gap(&self, gap: f64) -> f64 {
        if self.slow {
            gap * 1.5
        } else {
            gap
        }
    }

    pub fn gaps(&self) -> f64 {
        if self.slow {
            self.gap_coefficient * 2.0
        } else {
            self.gap_coefficient
        }
    }
}

/// Maps Chrome's world onto a play area inside the terminal.
///
/// The play area is the terminal, capped at Chrome's own canvas and centred
/// in whatever is left over. Below that cap it shrinks with the window --
/// the alternative, a hard minimum, would lock out an 80x24 terminal
/// entirely.
#[derive(Clone, Copy)]
pub struct View {
    pub cols: u16,
    pub rows: u16,
    /// Play area in cells.
    pub play_cols: u16,
    pub play_rows: u16,
    /// Cell column the play area starts at.
    pub x_off: i32,
    /// World width in Chrome units: one cell is two of them.
    pub world_w: f64,
    /// World width relative to Chrome's 600px canvas, capped at 1. Scales
    /// the simulation clock and the T-Rex's distance from the left edge.
    pub scale: f64,
    /// Pixel offset applied to every world row, negative when the sky is
    /// cropped because the play area is short.
    pub y_off: i32,
}

impl View {
    pub fn new(cols: u16, rows: u16) -> View {
        let play_cols = cols.min(FULL_COLS);
        let play_rows = rows.min(FULL_ROWS);
        let world_w = play_cols as f64 * 2.0;
        let bottom_row = (rows - play_rows) / 2 + play_rows;
        View {
            cols,
            rows,
            play_cols,
            play_rows,
            x_off: ((cols - play_cols) / 2) as i32,
            world_w,
            scale: (world_w / REFERENCE_W).min(1.0),
            y_off: bottom_row as i32 * 2 - (VIEW_BOTTOM / 2.0) as i32,
        }
    }

    /// Top-left cell of the play area.
    pub fn origin(&self) -> (u16, u16) {
        (self.x_off as u16, (self.rows - self.play_rows) / 2)
    }

    #[inline]
    pub fn sx(&self, x: f64) -> i32 {
        (x * 0.5).round() as i32 + self.x_off
    }

    #[inline]
    pub fn sy(&self, y: f64) -> i32 {
        (y * 0.5).round() as i32 + self.y_off
    }

    /// The T-Rex keeps Chrome's position relative to the window's width.
    pub fn trex_x(&self) -> f64 {
        (trex::START_X_POS * self.scale).round()
    }

    /// Highest world row still inside the play area.
    pub fn sky_top(&self) -> f64 {
        let top = self.origin().1 as i32 * 2;
        (top - self.y_off).max(0) as f64 * 2.0
    }

    /// Highest world row a jump should reach, so the T-Rex stays in frame.
    /// Chrome clamps a rising jump to -5px a frame once past this line, which
    /// carries it about 21px higher; the margin covers that.
    pub fn jump_ceiling(&self) -> f64 {
        self.sky_top() + 24.0
    }

    /// Keep a background element inside the visible strip of sky. Chrome has
    /// 150 rows to play with and a terminal usually has fewer.
    pub fn sky_y(&self, want: f64, height: f64) -> f64 {
        let lowest = (GROUND_Y - height - 4.0).max(0.0);
        want.max(self.sky_top() + 4.0).min(lowest)
    }
}

/// The three things every part of the world needs: where it is drawn, which
/// preset it follows, and a source of randomness.
pub struct Env<'a> {
    pub view: &'a View,
    pub config: &'a Config,
    pub rng: &'a mut Rng,
}

/// A box in world units, relative to a sprite's top-left corner.
#[derive(Clone, Copy, Debug)]
pub struct CollisionBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl CollisionBox {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> CollisionBox {
        CollisionBox { x, y, w, h }
    }

    /// Move a sprite-relative box to where the sprite actually is.
    pub fn offset(&self, by: &CollisionBox) -> CollisionBox {
        CollisionBox::new(self.x + by.x, self.y + by.y, self.w, self.h)
    }

    pub fn intersects(&self, other: &CollisionBox) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

/// xorshift64*, seeded from the clock. Stands in for `Math.random()`.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn from_clock() -> Rng {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15);
        Rng::new(nanos)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Inclusive on both ends, like Chrome's `getRandomNum`.
    pub fn range(&mut self, min: i64, max: i64) -> i64 {
        min + (self.f64() * (max - min + 1) as f64) as i64
    }
}

pub struct Game {
    pub view: View,
    pub config: &'static Config,
    pub trex: Trex,
    pub horizon: Horizon,
    pub rng: Rng,
    pub state: State,
    pub speed: f64,
    pub distance: f64,
    pub high_score: u32,
    pub inverted: bool,
    /// Set when the terminal cannot report key releases, so held keys have
    /// to be guessed from auto-repeat.
    pub auto_release: bool,
    running_time: f64,
    invert_timer: f64,
    crash_time: f64,
    duck_hold: f64,
    flash_timer: f64,
    flash_iterations: u32,
    flashing: bool,
    last_score: u32,
    pending_high_score: Option<u32>,
}

impl Game {
    pub fn new(view: View, config: &'static Config, high_score: u32, mut rng: Rng) -> Game {
        let horizon = Horizon::new(&mut Env {
            view: &view,
            config,
            rng: &mut rng,
        });
        Game {
            trex: Trex::new(view.trex_x(), config, view.jump_ceiling()),
            view,
            config,
            horizon,
            rng,
            state: State::Waiting,
            speed: config.speed,
            distance: 0.0,
            high_score,
            inverted: false,
            auto_release: true,
            running_time: 0.0,
            invert_timer: 0.0,
            crash_time: 0.0,
            duck_hold: 0.0,
            flash_timer: 0.0,
            flash_iterations: 0,
            flashing: false,
            last_score: 0,
            pending_high_score: None,
        }
    }

    pub fn score(&self) -> u32 {
        if self.distance > 0.0 {
            (self.distance * SCORE_COEFFICIENT).round() as u32
        } else {
            0
        }
    }

    /// A new record to write to disk, taken once.
    pub fn take_high_score(&mut self) -> Option<u32> {
        self.pending_high_score.take()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.view = View::new(cols, rows);
        self.trex.x = self.view.trex_x();
        self.trex.set_ceiling(self.config, self.view.jump_ceiling());
    }

    pub fn update(&mut self, dt: f64) {
        match self.state {
            State::Waiting => {
                if self.trex.blink_expired() {
                    let delay = self.rng.range(1, BLINK_TIMING) as f64;
                    self.trex.reset_blink(delay);
                }
                self.trex.update(dt, None);
            }
            State::Crashed => {
                self.crash_time += dt;
            }
            State::Running => self.update_running(dt),
        }
    }

    fn update_running(&mut self, dt_real: f64) {
        // A narrow terminal shows less of the world, so it runs proportionally
        // slower. Geometry stays Chrome's; only the clock changes.
        let dt = dt_real * self.view.scale;

        if self.trex.jumping {
            self.trex.update_jump(dt, self.config);
        }

        // The opening pause is for the player, so it stays in real seconds.
        self.running_time += dt_real;
        // Chrome gives the player a few seconds of empty ground to start.
        let has_obstacles = self.running_time > CLEAR_TIME;

        self.horizon.update(
            dt,
            self.speed,
            has_obstacles,
            self.inverted,
            &mut Env {
                view: &self.view,
                config: self.config,
                rng: &mut self.rng,
            },
        );

        if has_obstacles && self.collided() {
            self.game_over();
            return;
        }

        self.distance += self.speed * dt / MS_PER_FRAME;
        if self.speed < self.config.max_speed {
            self.speed += self.config.acceleration * dt / MS_PER_FRAME;
        }
        self.update_score_flash(dt);
        self.update_night(dt);

        if self.auto_release && self.duck_hold > 0.0 {
            self.duck_hold -= dt_real;
            if self.duck_hold <= 0.0 {
                self.duck_release();
            }
        }

        self.trex.update(dt, None);
    }

    /// Chrome flashes the score every 100 points.
    fn update_score_flash(&mut self, dt: f64) {
        let score = self.score();
        if !self.flashing {
            if score > self.last_score && score % ACHIEVEMENT_DISTANCE == 0 {
                self.flashing = true;
                self.flash_timer = 0.0;
                self.flash_iterations = 0;
            }
        } else {
            self.flash_timer += dt;
            if self.flash_timer > FLASH_DURATION * 2.0 {
                self.flash_timer = 0.0;
                self.flash_iterations += 1;
                if self.flash_iterations > FLASH_ITERATIONS {
                    self.flashing = false;
                }
            }
        }
        self.last_score = score;
    }

    /// Night falls every `invert_distance` points and lasts 12 seconds.
    fn update_night(&mut self, dt: f64) {
        if self.invert_timer > INVERT_FADE_DURATION {
            self.invert_timer = 0.0;
            self.inverted = false;
        } else if self.invert_timer > 0.0 {
            self.invert_timer += dt;
        } else {
            let score = self.score();
            if score > 0 && score % self.config.invert_distance == 0 {
                self.invert_timer += dt;
                self.inverted = true;
            }
        }
    }

    fn collided(&self) -> bool {
        let Some(obstacle) = self.horizon.obstacles.first() else {
            return false;
        };

        // Chrome shrinks both boxes by the 1px white border its sprites carry,
        // and always uses the standing width -- even while ducking.
        let trex_box = CollisionBox::new(
            self.trex.x + 1.0,
            self.trex.y + 1.0,
            trex::WIDTH - 2.0,
            trex::HEIGHT - 2.0,
        );
        let obstacle_box = CollisionBox::new(
            obstacle.x + 1.0,
            obstacle.y + 1.0,
            obstacle.width - 2.0,
            obstacle.height() - 2.0,
        );

        if !trex_box.intersects(&obstacle_box) {
            return false;
        }
        self.trex.collision_boxes().iter().any(|tb| {
            let tb = tb.offset(&trex_box);
            obstacle
                .boxes
                .iter()
                .any(|ob| tb.intersects(&ob.offset(&obstacle_box)))
        })
    }

    fn game_over(&mut self) {
        self.state = State::Crashed;
        self.crash_time = 0.0;
        self.flashing = false;
        self.trex.update(100.0, Some(Status::Crashed));
        if self.score() > self.high_score {
            self.high_score = self.score();
            self.pending_high_score = Some(self.high_score);
        }
    }

    pub fn restart(&mut self) {
        self.state = State::Running;
        self.running_time = 0.0;
        self.distance = 0.0;
        self.last_score = 0;
        self.speed = self.config.speed;
        self.invert_timer = 0.0;
        self.inverted = false;
        self.flashing = false;
        self.horizon.reset();
        self.trex.reset();
    }

    pub fn jump_press(&mut self) {
        match self.state {
            State::Waiting => {
                self.state = State::Running;
                self.trex.reset();
                self.trex.start_jump(self.speed, self.config);
            }
            State::Running => {
                if self.trex.ducking && self.auto_release {
                    // Without key-up events a duck can outstay its welcome;
                    // jumping cancels it rather than being swallowed.
                    self.duck_release();
                }
                if !self.trex.jumping && !self.trex.ducking {
                    self.trex.start_jump(self.speed, self.config);
                }
            }
            State::Crashed => {
                if self.crash_time >= GAMEOVER_CLEAR_TIME {
                    self.restart();
                }
            }
        }
    }

    pub fn jump_release(&mut self) {
        if self.state == State::Running {
            self.trex.end_jump(self.config);
        }
    }

    pub fn duck_press(&mut self) {
        if self.state != State::Running {
            return;
        }
        self.duck_hold = DUCK_HOLD_MS;
        if self.trex.jumping {
            self.trex.set_speed_drop();
        } else if !self.trex.ducking {
            self.trex.set_duck(true);
        }
    }

    pub fn duck_release(&mut self) {
        self.duck_hold = 0.0;
        self.trex.speed_drop = false;
        self.trex.set_duck(false);
    }

    /// Enter restarts immediately after a crash.
    pub fn restart_press(&mut self) {
        if self.state == State::Crashed {
            self.restart();
        }
    }

    pub fn draw(&self, screen: &mut Screen) {
        screen.clear();
        screen.set_night(self.inverted);
        self.horizon.draw(screen, &self.view);
        screen.blit(
            self.trex.sprite(),
            self.view.sx(self.trex.x),
            self.view.sy(self.trex.y),
        );

        // The HUD is drawn into the framebuffer with the pixel font rather
        // than printed as terminal text: it scales with the game instead of
        // the terminal's font, and inverts with night mode.
        let (left, top) = self.view.origin();
        let (left, top) = (left as i32, top as i32 * 2);
        let width = self.view.play_cols as i32;
        // Panels sit a third of the way down, clear of the ground and the
        // T-Rex, where Chrome puts its game-over text.
        let panel = top + self.view.play_rows as i32 * 2 / 3;

        if !self.flashing || self.flash_timer >= FLASH_DURATION {
            let score = if self.high_score > 0 {
                format!("HI {:05}  {:05}", self.high_score, self.score())
            } else {
                format!("{:05}", self.score())
            };
            let x = left + width - font::width(&score, 1) - 1;
            screen.blit_text(x, top + 2, &score, 1);
        }

        match self.state {
            State::Waiting => {
                // Longest hint that fits; a 60-column terminal has room for
                // very little.
                let hint = [
                    "SPACE JUMP · DOWN DUCK · Q QUIT",
                    "SPACE JUMP · Q QUIT",
                    "SPACE JUMP",
                ]
                .into_iter()
                .find(|text| font::width(text, 1) <= width - 2)
                .unwrap_or("");
                self.centered(screen, panel, hint, 1);

                // The play area is as wide as the terminal until it reaches
                // Chrome's canvas, so say what size shows the whole world.
                if self.view.play_cols < FULL_COLS || self.view.play_rows < FULL_ROWS {
                    let resize = format!("RESIZE TO {FULL_COLS}X{FULL_ROWS}");
                    if font::width(&resize, 1) <= width - 2 {
                        self.centered(screen, panel + font::HEIGHT + 4, &resize, 1);
                    }
                }
            }
            State::Crashed => {
                let banner = if font::width("GAME OVER", 2) + 4 <= width {
                    2
                } else {
                    1
                };
                self.centered(screen, panel, "GAME OVER", banner);
                self.centered(screen, panel + font::HEIGHT * banner + 4, "PRESS SPACE", 1);
            }
            State::Running => {}
        }
    }

    /// Centre pixel-font text on the play area.
    fn centered(&self, screen: &mut Screen, y: i32, text: &str, scale: i32) {
        let (left, _) = self.view.origin();
        let x = left as i32 + (self.view.play_cols as i32 - font::width(text, scale)) / 2;
        screen.blit_text(x, y, text, scale);
    }
}
