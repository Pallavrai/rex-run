//! The T-Rex: jump physics, ducking and animation.
//!
//! Ported from Chromium's `dino_game/trex.ts`; the constants and the update
//! order are the browser's, which is what makes the jump feel right.

use crate::game::{CollisionBox, Config, BOTTOM_PAD, MS_PER_FRAME, WORLD_H};
use crate::render::Sprite;
use crate::sprites;

/// Sprite dimensions, in Chrome's world units.
pub const WIDTH: f64 = 44.0;
pub const HEIGHT: f64 = 47.0;
/// Distance from the left edge of a 600px world.
pub const START_X_POS: f64 = 50.0;

const SPEED_DROP_COEFFICIENT: f64 = 3.0;
const BLINK_TIMING: f64 = 7000.0;
const BLINK_DURATION: f64 = 1000.0 / 3.0;

/// Boxes are relative to the sprite's top-left corner, in world units.
const DUCKING_BOXES: &[CollisionBox] = &[CollisionBox::new(1.0, 18.0, 55.0, 25.0)];
const RUNNING_BOXES: &[CollisionBox] = &[
    CollisionBox::new(22.0, 0.0, 17.0, 16.0),
    CollisionBox::new(1.0, 18.0, 30.0, 9.0),
    CollisionBox::new(10.0, 35.0, 14.0, 8.0),
    CollisionBox::new(1.0, 24.0, 29.0, 5.0),
    CollisionBox::new(5.0, 30.0, 21.0, 4.0),
    CollisionBox::new(9.0, 34.0, 15.0, 4.0),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Waiting,
    Running,
    Jumping,
    Ducking,
    Crashed,
}

impl Status {
    /// Milliseconds each animation frame is held.
    fn ms_per_frame(self) -> f64 {
        match self {
            Status::Waiting => 1000.0 / 3.0,
            Status::Running => 1000.0 / 12.0,
            Status::Ducking => 1000.0 / 8.0,
            Status::Jumping | Status::Crashed => MS_PER_FRAME,
        }
    }

    fn frames(self) -> usize {
        match self {
            Status::Running | Status::Ducking | Status::Waiting => 2,
            Status::Jumping | Status::Crashed => 1,
        }
    }
}

pub struct Trex {
    pub x: f64,
    pub y: f64,
    pub status: Status,
    pub jumping: bool,
    pub ducking: bool,
    pub speed_drop: bool,
    pub jump_velocity: f64,
    pub jump_count: u32,
    ground_y: f64,
    min_jump_height: f64,
    max_jump_height: f64,
    reached_min_height: bool,
    timer: f64,
    frame: usize,
    blink_timer: f64,
    blink_delay: f64,
}

impl Trex {
    /// `ceiling` is the highest world row the T-Rex may reach. Chrome's jump
    /// peaks near the top of its 150px canvas, which is off-screen on a short
    /// terminal, so the apex is pulled down to whatever is visible.
    pub fn new(x: f64, config: &Config, ceiling: f64) -> Self {
        let ground_y = WORLD_H - HEIGHT - BOTTOM_PAD;
        let mut trex = Trex {
            x,
            y: ground_y,
            status: Status::Waiting,
            jumping: false,
            ducking: false,
            speed_drop: false,
            jump_velocity: 0.0,
            jump_count: 0,
            ground_y,
            min_jump_height: 0.0,
            max_jump_height: 0.0,
            reached_min_height: false,
            timer: 0.0,
            frame: 0,
            blink_timer: 0.0,
            blink_delay: BLINK_TIMING / 2.0,
        };
        trex.set_ceiling(config, ceiling);
        trex
    }

    /// Re-apply the jump ceiling, e.g. after the terminal is resized.
    pub fn set_ceiling(&mut self, config: &Config, ceiling: f64) {
        self.max_jump_height = config.max_jump_height.max(ceiling);
        self.min_jump_height = (self.ground_y - config.min_jump_height).max(self.max_jump_height);
    }

    pub fn update(&mut self, dt: f64, status: Option<Status>) {
        self.timer += dt;

        if let Some(status) = status {
            self.status = status;
            self.frame = 0;
            self.timer = 0.0;
        }

        if self.timer >= self.status.ms_per_frame() {
            self.frame = (self.frame + 1) % self.status.frames();
            self.timer = 0.0;
        }

        // Chrome blinks by drawing the closed-eye frame once a random delay
        // has elapsed; same effect, spelled out.
        if self.status == Status::Waiting {
            self.blink_timer += dt;
        }

        // A speed drop that lands becomes a duck if the key is still down.
        if self.speed_drop && self.y == self.ground_y {
            self.speed_drop = false;
            self.set_duck(true);
        }
    }

    /// True once a blink has run its course and a new delay is due.
    pub fn blink_expired(&self) -> bool {
        self.blink_timer >= self.blink_delay + BLINK_DURATION
    }

    /// Pick a new random delay before the next blink.
    pub fn reset_blink(&mut self, delay: f64) {
        self.blink_delay = delay;
        self.blink_timer = 0.0;
    }

    pub fn start_jump(&mut self, speed: f64, config: &Config) {
        if !self.jumping {
            self.update(0.0, Some(Status::Jumping));
            // Faster games get a slightly higher jump.
            self.jump_velocity = config.initial_jump_velocity - (speed / 10.0);
            self.jumping = true;
            self.reached_min_height = false;
            self.speed_drop = false;
        }
    }

    /// Cut the jump short -- the jump key was released, or the apex reached.
    pub fn end_jump(&mut self, config: &Config) {
        if self.reached_min_height && self.jump_velocity < config.drop_velocity {
            self.jump_velocity = config.drop_velocity;
        }
    }

    pub fn update_jump(&mut self, dt: f64, config: &Config) {
        let frames_elapsed = dt / Status::Jumping.ms_per_frame();

        if self.speed_drop {
            self.y += (self.jump_velocity * SPEED_DROP_COEFFICIENT * frames_elapsed).round();
        } else {
            self.y += (self.jump_velocity * frames_elapsed).round();
        }
        self.jump_velocity += config.gravity * frames_elapsed;

        if self.y < self.min_jump_height || self.speed_drop {
            self.reached_min_height = true;
        }
        if self.y < self.max_jump_height || self.speed_drop {
            self.end_jump(config);
        }
        if self.y > self.ground_y {
            self.reset();
            self.jump_count += 1;
        }
    }

    /// Cancel the current jump and drop like a stone.
    pub fn set_speed_drop(&mut self) {
        self.speed_drop = true;
        self.jump_velocity = 1.0;
    }

    pub fn set_duck(&mut self, ducking: bool) {
        if ducking && self.status != Status::Ducking {
            self.update(0.0, Some(Status::Ducking));
            self.ducking = true;
        } else if !ducking && self.status == Status::Ducking {
            self.update(0.0, Some(Status::Running));
            self.ducking = false;
        }
    }

    pub fn reset(&mut self) {
        self.y = self.ground_y;
        self.jump_velocity = 0.0;
        self.jumping = false;
        self.ducking = false;
        self.speed_drop = false;
        self.jump_count = 0;
        self.update(0.0, Some(Status::Running));
    }

    pub fn collision_boxes(&self) -> &'static [CollisionBox] {
        if self.ducking {
            DUCKING_BOXES
        } else {
            RUNNING_BOXES
        }
    }

    pub fn sprite(&self) -> &'static Sprite {
        match self.status {
            Status::Waiting => {
                if self.blink_timer >= self.blink_delay {
                    &sprites::TREX_BLINK
                } else {
                    &sprites::TREX_STAND
                }
            }
            Status::Running => {
                if self.frame == 0 {
                    &sprites::TREX_RUN_1
                } else {
                    &sprites::TREX_RUN_2
                }
            }
            Status::Ducking => {
                if self.frame == 0 {
                    &sprites::TREX_DUCK_1
                } else {
                    &sprites::TREX_DUCK_2
                }
            }
            Status::Jumping => &sprites::TREX_STAND,
            Status::Crashed => &sprites::TREX_CRASHED,
        }
    }
}
