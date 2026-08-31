//! Everything behind and around the T-Rex: the scrolling ground, clouds,
//! obstacle spawning, and night mode with its moon phases and stars.
//!
//! Ported from Chromium's `horizon.ts`, `horizon_line.ts`, `cloud.ts` and
//! `night_mode.ts`.

use crate::game::{
    Env, View, BG_CLOUD_SPEED, CLOUD_FREQUENCY, FPS, GROUND_Y, MAX_CLOUDS,
    MAX_OBSTACLE_DUPLICATION, MS_PER_FRAME,
};
use crate::obstacle::{Obstacle, TYPES};
use crate::render::{Screen, Sprite};
use crate::sprites;

const LINE_WIDTH: f64 = 600.0;

const CLOUD_WIDTH: f64 = 46.0;
const CLOUD_HEIGHT: f64 = 14.0;
const MIN_CLOUD_GAP: i64 = 100;
const MAX_CLOUD_GAP: i64 = 400;
const MAX_SKY_LEVEL: i64 = 30;
const MIN_SKY_LEVEL: i64 = 71;

const MOON_WIDTH: f64 = 20.0;
const MOON_HEIGHT: f64 = 40.0;
const MOON_SPEED: f64 = 0.25;
const MOON_Y: f64 = 30.0;
const STAR_SIZE: f64 = 9.0;
const STAR_SPEED: f64 = 0.3;
const STAR_MAX_Y: i64 = 70;
const NUM_STARS: usize = 2;
const FADE_SPEED: f64 = 0.035;

struct Cloud {
    x: f64,
    y: f64,
    gap: f64,
}

struct Star {
    x: f64,
    y: f64,
    sprite: &'static Sprite,
}

struct NightMode {
    x: f64,
    y: f64,
    phase: usize,
    opacity: f64,
    stars: Vec<Star>,
}

pub struct Horizon {
    pub obstacles: Vec<Obstacle>,
    clouds: Vec<Cloud>,
    /// Two copies of the ground strip, scrolling and swapping places.
    line_x: [f64; 2],
    line_bumpy: [bool; 2],
    night: NightMode,
    history: Vec<usize>,
}

impl Horizon {
    pub fn new(env: &mut Env) -> Horizon {
        let mut horizon = Horizon {
            obstacles: Vec::new(),
            clouds: Vec::new(),
            line_x: [0.0, LINE_WIDTH],
            line_bumpy: [false, false],
            night: NightMode {
                x: 0.0,
                y: MOON_Y,
                phase: 0,
                opacity: 0.0,
                stars: Vec::new(),
            },
            history: Vec::new(),
        };
        horizon.add_cloud(env);
        horizon.place_stars(env);
        horizon
    }

    pub fn reset(&mut self) {
        self.obstacles.clear();
        self.history.clear();
        self.line_x = [0.0, LINE_WIDTH];
        self.night.phase = 0;
        self.night.opacity = 0.0;
    }

    pub fn update(
        &mut self,
        dt: f64,
        speed: f64,
        update_obstacles: bool,
        night: bool,
        env: &mut Env,
    ) {
        self.update_line(dt, speed, env);
        self.update_night(dt, night, env);
        self.update_clouds(dt, speed, env);
        if update_obstacles {
            self.update_obstacles(dt, speed, env);
        }
    }

    fn update_line(&mut self, dt: f64, speed: f64, env: &mut Env) {
        let increment = speed * FPS / 1000.0 * dt;
        let (first, second) = if self.line_x[0] <= 0.0 {
            (0, 1)
        } else {
            (1, 0)
        };

        self.line_x[first] -= increment;
        self.line_x[second] = self.line_x[first] + LINE_WIDTH;

        if self.line_x[first] <= -LINE_WIDTH {
            self.line_x[first] += LINE_WIDTH * 2.0;
            self.line_x[second] = self.line_x[first] - LINE_WIDTH;
            // Each strip picks flat or bumpy ground as it wraps around.
            self.line_bumpy[first] = env.rng.f64() > 0.5;
        }
    }

    fn update_clouds(&mut self, dt: f64, speed: f64, env: &mut Env) {
        // Chrome rounds the cloud step up, so clouds drift at a steady 1px
        // per frame at any normal speed.
        let step = (BG_CLOUD_SPEED / 1000.0 * MS_PER_FRAME * speed).max(1.0);

        if self.clouds.is_empty() {
            self.add_cloud(env);
            return;
        }
        for cloud in &mut self.clouds {
            cloud.x -= step * dt / MS_PER_FRAME;
        }
        self.clouds.retain(|c| c.x + CLOUD_WIDTH > 0.0);

        if let Some(last) = self.clouds.last() {
            if self.clouds.len() < MAX_CLOUDS
                && (env.view.world_w - last.x) > last.gap
                && CLOUD_FREQUENCY > env.rng.f64()
            {
                self.add_cloud(env);
            }
        }
    }

    fn add_cloud(&mut self, env: &mut Env) {
        let y = env.rng.range(MAX_SKY_LEVEL, MIN_SKY_LEVEL) as f64;
        self.clouds.push(Cloud {
            x: env.view.world_w,
            y: env.view.sky_y(y, CLOUD_HEIGHT),
            gap: env.rng.range(MIN_CLOUD_GAP, MAX_CLOUD_GAP) as f64,
        });
    }

    fn place_stars(&mut self, env: &mut Env) {
        let segment = env.view.world_w / NUM_STARS as f64;
        self.night.stars = (0..NUM_STARS)
            .map(|i| {
                let y = env.rng.range(0, STAR_MAX_Y) as f64;
                Star {
                    x: env.rng.range(
                        (segment * i as f64) as i64,
                        (segment * (i + 1) as f64) as i64,
                    ) as f64,
                    y: env.view.sky_y(y, STAR_SIZE),
                    sprite: if i == 0 {
                        &sprites::STAR_1
                    } else {
                        &sprites::STAR_2
                    },
                }
            })
            .collect();
    }

    fn update_night(&mut self, dt: f64, activated: bool, env: &mut Env) {
        let frames = dt / MS_PER_FRAME;

        // A new moon phase is picked each time night falls.
        if activated && self.night.opacity == 0.0 {
            self.night.phase = (self.night.phase + 1) % MOON_PHASES.len();
            self.night.y = env.view.sky_y(MOON_Y, MOON_HEIGHT);
        }
        if activated {
            self.night.opacity = (self.night.opacity + FADE_SPEED * frames).min(1.0);
        } else {
            self.night.opacity = (self.night.opacity - FADE_SPEED * frames).max(0.0);
        }

        if self.night.opacity > 0.0 {
            self.night.x = wrap(self.night.x, MOON_SPEED * frames, env.view, MOON_WIDTH);
            for star in &mut self.night.stars {
                star.x = wrap(star.x, STAR_SPEED * frames, env.view, STAR_SIZE);
            }
        } else {
            self.place_stars(env);
        }
    }

    fn update_obstacles(&mut self, dt: f64, speed: f64, env: &mut Env) {
        for obstacle in &mut self.obstacles {
            obstacle.update(dt, speed);
        }
        self.obstacles.retain(|o| !o.remove);

        let spawn = match self.obstacles.last() {
            Some(last) => {
                !last.following_created
                    && last.is_visible()
                    && (last.x + last.width + last.gap) < env.view.world_w
            }
            None => true,
        };
        if spawn {
            if let Some(last) = self.obstacles.last_mut() {
                last.following_created = true;
            }
            self.add_obstacle(speed, env);
        }
    }

    fn add_obstacle(&mut self, speed: f64, env: &mut Env) {
        // Chrome retries until it finds a type that is unlocked at this speed
        // and is not a third repeat; bounded here so it always terminates.
        for _ in 0..16 {
            let kind = env.rng.range(0, TYPES.len() as i64 - 1) as usize;
            if self.duplicate(kind) || speed < env.config.min_speed(TYPES[kind].min_speed) {
                continue;
            }
            self.obstacles.push(Obstacle::new(kind, speed, env));
            self.history.insert(0, kind);
            self.history.truncate(MAX_OBSTACLE_DUPLICATION);
            return;
        }
    }

    /// True once the same obstacle has been used for the last two spawns.
    fn duplicate(&self, kind: usize) -> bool {
        let mut count = 0;
        for &previous in &self.history {
            count = if previous == kind { count + 1 } else { 0 };
        }
        count >= MAX_OBSTACLE_DUPLICATION
    }

    pub fn draw(&self, screen: &mut Screen, view: &View) {
        if self.night.opacity > 0.0 {
            for star in &self.night.stars {
                screen.blit(star.sprite, view.sx(star.x), view.sy(star.y));
            }
            let moon = MOON_PHASES[self.night.phase];
            screen.blit(moon, view.sx(self.night.x), view.sy(self.night.y));
        }
        for cloud in &self.clouds {
            screen.blit(&sprites::CLOUD, view.sx(cloud.x), view.sy(cloud.y));
        }
        for (i, &x) in self.line_x.iter().enumerate() {
            let sprite = if self.line_bumpy[i] {
                &sprites::HORIZON_BUMPY
            } else {
                &sprites::HORIZON_FLAT
            };
            screen.blit(sprite, view.sx(x), view.sy(GROUND_Y));
        }
        for obstacle in &self.obstacles {
            screen.blit(obstacle.sprite(), view.sx(obstacle.x), view.sy(obstacle.y));
        }
    }
}

/// Scroll a background element leftwards, wrapping it around the world.
fn wrap(pos: f64, speed: f64, view: &View, width: f64) -> f64 {
    if pos < -width {
        view.world_w
    } else {
        pos - speed
    }
}

static MOON_PHASES: [&Sprite; 7] = [
    &sprites::MOON_0,
    &sprites::MOON_1,
    &sprites::MOON_2,
    &sprites::MOON_3,
    &sprites::MOON_4,
    &sprites::MOON_5,
    &sprites::MOON_6,
];
