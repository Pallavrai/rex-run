//! Cacti and pterodactyls.
//!
//! Ported from Chromium's `dino_game/obstacle.ts` and the obstacle table in
//! `offline_sprite_definitions.ts`.

use crate::game::{CollisionBox, Env, FPS, MAX_OBSTACLE_LENGTH};
use crate::render::Sprite;
use crate::sprites;

/// Widest the gap between two obstacles can get, relative to the minimum.
const MAX_GAP_COEFFICIENT: f64 = 1.5;

pub struct Type {
    pub width: f64,
    pub height: f64,
    /// More than one entry means the obstacle flies at a random height.
    pub y_pos: &'static [f64],
    /// Groups of two or three only appear once the game is this fast.
    pub multiple_speed: f64,
    pub min_gap: f64,
    pub min_speed: f64,
    /// Pterodactyls drift slightly faster or slower than the ground.
    pub speed_offset: f64,
    pub boxes: &'static [CollisionBox],
    pub frames: usize,
    pub frame_rate: f64,
}

pub static TYPES: [Type; 3] = [
    // cactusSmall
    Type {
        width: 17.0,
        height: 35.0,
        y_pos: &[105.0],
        multiple_speed: 4.0,
        min_gap: 120.0,
        min_speed: 0.0,
        speed_offset: 0.0,
        boxes: &[
            CollisionBox::new(0.0, 7.0, 5.0, 27.0),
            CollisionBox::new(4.0, 0.0, 6.0, 34.0),
            CollisionBox::new(10.0, 4.0, 7.0, 14.0),
        ],
        frames: 1,
        frame_rate: 0.0,
    },
    // cactusLarge
    Type {
        width: 25.0,
        height: 50.0,
        y_pos: &[90.0],
        multiple_speed: 7.0,
        min_gap: 120.0,
        min_speed: 0.0,
        speed_offset: 0.0,
        boxes: &[
            CollisionBox::new(0.0, 12.0, 7.0, 38.0),
            CollisionBox::new(8.0, 0.0, 7.0, 49.0),
            CollisionBox::new(13.0, 10.0, 10.0, 38.0),
        ],
        frames: 1,
        frame_rate: 0.0,
    },
    // pterodactyl
    Type {
        width: 46.0,
        height: 40.0,
        y_pos: &[100.0, 75.0, 50.0],
        multiple_speed: 999.0,
        min_gap: 150.0,
        min_speed: 8.5,
        speed_offset: 0.8,
        boxes: &[
            CollisionBox::new(15.0, 15.0, 16.0, 5.0),
            CollisionBox::new(18.0, 21.0, 24.0, 6.0),
            CollisionBox::new(2.0, 14.0, 4.0, 3.0),
            CollisionBox::new(6.0, 10.0, 4.0, 7.0),
            CollisionBox::new(10.0, 8.0, 6.0, 9.0),
        ],
        frames: 2,
        frame_rate: 1000.0 / 6.0,
    },
];

pub struct Obstacle {
    pub kind: usize,
    pub x: f64,
    pub y: f64,
    pub size: usize,
    pub width: f64,
    pub gap: f64,
    pub boxes: Vec<CollisionBox>,
    pub following_created: bool,
    pub remove: bool,
    speed_offset: f64,
    frame: usize,
    timer: f64,
}

impl Obstacle {
    pub fn new(kind: usize, speed: f64, env: &mut Env) -> Obstacle {
        let (view, config, rng) = (env.view, env.config, &mut *env.rng);
        let ty = &TYPES[kind];
        let mut size = rng.range(1, MAX_OBSTACLE_LENGTH) as usize;
        if size > 1 && config.multiple_speed(ty.multiple_speed) > speed {
            size = 1;
        }
        let width = ty.width * size as f64;

        // Slow mode flies pterodactyls at a single height instead of three.
        let y = if ty.y_pos.len() > 1 && !config.slow {
            ty.y_pos[rng.range(0, ty.y_pos.len() as i64 - 1) as usize]
        } else {
            ty.y_pos[0]
        };

        let mut boxes = ty.boxes.to_vec();
        // Grouped cacti are one wide obstacle: stretch the middle box and
        // push the right-hand one out to the new edge.
        //      ____        ______        ________
        //    _|   |-|    _|     |-|    _|       |-|
        //   | |<->| |   | |<--->| |   | |<----->| |
        //   |_|___|_|   |_|_____|_|   |_|_______|_|
        if size > 1 {
            boxes[1].w = width - boxes[0].w - boxes[2].w;
            boxes[2].x = width - boxes[2].w;
        }

        let speed_offset = if ty.speed_offset != 0.0 {
            if rng.f64() > 0.5 {
                ty.speed_offset
            } else {
                -ty.speed_offset
            }
        } else {
            0.0
        };

        let min_gap = (width * speed + config.min_gap(ty.min_gap) * config.gaps()).round();
        let max_gap = (min_gap * MAX_GAP_COEFFICIENT).round();
        let gap = rng.range(min_gap as i64, max_gap as i64) as f64;

        Obstacle {
            kind,
            x: view.world_w + ty.width,
            y,
            size,
            width,
            gap,
            boxes,
            following_created: false,
            remove: false,
            speed_offset,
            frame: 0,
            timer: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64, speed: f64) {
        let speed = speed + self.speed_offset;
        self.x -= speed * FPS / 1000.0 * dt;

        let ty = &TYPES[self.kind];
        if ty.frames > 1 {
            self.timer += dt;
            if self.timer >= ty.frame_rate {
                self.frame = (self.frame + 1) % ty.frames;
                self.timer = 0.0;
            }
        }

        if !self.is_visible() {
            self.remove = true;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.x + self.width > 0.0
    }

    pub fn height(&self) -> f64 {
        TYPES[self.kind].height
    }

    pub fn sprite(&self) -> &'static Sprite {
        match (self.kind, self.size) {
            (0, 1) => &sprites::CACTUS_SMALL_1,
            (0, 2) => &sprites::CACTUS_SMALL_2,
            (0, _) => &sprites::CACTUS_SMALL_3,
            (1, 1) => &sprites::CACTUS_LARGE_1,
            (1, 2) => &sprites::CACTUS_LARGE_2,
            (1, _) => &sprites::CACTUS_LARGE_3,
            (_, _) if self.frame == 0 => &sprites::PTERODACTYL_1,
            (_, _) => &sprites::PTERODACTYL_2,
        }
    }
}
