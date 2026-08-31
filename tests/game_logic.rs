//! The parts of the port that would rot silently: collision maths, the
//! score's rate, obstacle spacing, sprite unpacking, and one long seeded
//! run that fingerprints the physics.

use rex_run::game::{self, CollisionBox, Env, Game, Rng, State, View, MS_PER_FRAME};
use rex_run::obstacle::{Obstacle, TYPES};
use rex_run::sprites;
use rex_run::trex;

fn new_game(cols: u16, rows: u16, seed: u64) -> Game {
    Game::new(View::new(cols, rows), &game::NORMAL, 0, Rng::new(seed))
}

#[test]
fn sprites_unpack_to_the_right_pixels() {
    let dino = &sprites::TREX_STAND;
    assert_eq!(
        (dino.w, dino.h),
        (22, 24),
        "T-Rex is half of Chrome's 44x47"
    );
    assert_eq!(dino.pixel(0, 0), None, "corner is transparent");
    assert_eq!(dino.pixel(12, 2), Some(true), "head is ink");
    assert_eq!(
        dino.pixel(13, 2),
        Some(false),
        "the eye is paper, not a hole"
    );
    assert_eq!(
        sprites::HORIZON_FLAT.w,
        300,
        "ground strip is 600 world units"
    );
}

#[test]
fn collision_boxes_use_chrome_s_rules() {
    let box_a = CollisionBox::new(0.0, 0.0, 10.0, 10.0);
    assert!(box_a.intersects(&CollisionBox::new(9.0, 9.0, 5.0, 5.0)));
    assert!(
        !box_a.intersects(&CollisionBox::new(10.0, 0.0, 5.0, 5.0)),
        "touching is not hitting"
    );
    assert!(!box_a.intersects(&CollisionBox::new(0.0, -5.0, 5.0, 5.0)));

    let moved =
        CollisionBox::new(1.0, 2.0, 3.0, 4.0).offset(&CollisionBox::new(10.0, 20.0, 0.0, 0.0));
    assert_eq!((moved.x, moved.y, moved.w, moved.h), (11.0, 22.0, 3.0, 4.0));
}

#[test]
fn score_ticks_at_chrome_s_rate() {
    // Chrome scores 0.025 points per pixel run and starts at 6px a frame:
    // nine points in the first second at Chrome's own width.
    let mut game = new_game(300, 30, 1);
    game.jump_press();
    for _ in 0..60 {
        game.update(MS_PER_FRAME);
    }
    assert_eq!(game.score(), 9);
}

#[test]
fn a_jump_covers_the_same_ground_at_any_width() {
    // The invariant that keeps the game playable on a small terminal: a jump
    // always carries the T-Rex over the same number of world pixels, so an
    // obstacle that can be cleared in Chrome can be cleared here.
    for cols in [80, 120, 300] {
        let mut game = new_game(cols, 30, 5);
        game.jump_press();
        let start = game.distance;
        let mut frames = 0;
        while game.trex.jumping && frames < 400 {
            game.update(MS_PER_FRAME);
            frames += 1;
        }
        let travelled = game.distance - start;
        assert!(
            (travelled - 200.0).abs() < 30.0,
            "{cols} columns: a jump covered {travelled:.0}px"
        );
    }
}

#[test]
fn obstacle_gaps_stay_within_chrome_s_bounds() {
    // A 300-column terminal is exactly Chrome's 600px world, so the gaps
    // should come out in Chrome's own units.
    let view = View::new(300, 30);
    let mut rng = Rng::new(7);

    for _ in 0..200 {
        let mut env = Env {
            view: &view,
            config: &game::NORMAL,
            rng: &mut rng,
        };
        let obstacle = Obstacle::new(0, 6.0, &mut env);
        let min = (obstacle.width * 6.0 + TYPES[0].min_gap * game::NORMAL.gap_coefficient).round();
        assert!(
            obstacle.gap >= min && obstacle.gap <= (min * 1.5).round(),
            "gap {} outside [{min}, {}]",
            obstacle.gap,
            min * 1.5
        );
        assert!((1..=3).contains(&obstacle.size));
    }
}

#[test]
fn a_jump_clears_the_tallest_cactus() {
    let mut game = new_game(300, 30, 3);
    game.jump_press();
    let ground = game.trex.y;
    let mut apex = ground;
    let mut frames = 0;

    while game.trex.jumping && frames < 120 {
        game.update(MS_PER_FRAME);
        apex = apex.min(game.trex.y);
        frames += 1;
    }

    assert!(!game.trex.jumping, "the jump ended");
    // The large cactus is 50 tall and sits at y 90; the T-Rex is 47 tall.
    assert!(
        apex + trex::HEIGHT < 90.0,
        "apex {apex} does not clear a large cactus"
    );
    assert!((20..=60).contains(&frames), "jump lasted {frames} frames");
}

#[test]
fn ducking_swaps_the_collision_boxes() {
    let mut game = new_game(120, 30, 4);
    game.jump_press();
    game.update(MS_PER_FRAME);
    let standing = game.trex.collision_boxes().len();

    while game.trex.jumping {
        game.update(MS_PER_FRAME);
    }
    game.duck_press();
    assert!(game.trex.ducking);
    assert_eq!(
        game.trex.collision_boxes().len(),
        1,
        "ducking is one flat box"
    );
    assert!(standing > 1);

    game.duck_release();
    assert!(!game.trex.ducking);
}

#[test]
fn pterodactyls_wait_for_chrome_s_unlock_speed() {
    // 40 rows fits Chrome's whole 150px world, jump included.
    let mut game = new_game(300, 40, 11);
    game.jump_press();

    for _ in 0..6000 {
        autopilot(&mut game);
        game.update(MS_PER_FRAME);
        if game.horizon.obstacles.iter().any(|o| o.kind == 2) {
            assert!(
                game.speed >= TYPES[2].min_speed,
                "pterodactyl at speed {}",
                game.speed
            );
            return;
        }
    }
    panic!("no pterodactyl in 6000 frames");
}

/// A fixed seed, a scripted player and a fixed frame time: the whole
/// simulation should land on the same number every run, on every platform.
/// If a deliberate change moves it, update the constant here.
#[test]
fn a_seeded_run_is_reproducible() {
    let mut game = new_game(120, 30, 0x5eed);
    game.jump_press();

    for _ in 0..2400 {
        autopilot(&mut game);
        game.update(MS_PER_FRAME);
    }

    assert_eq!(game.state, State::Running, "the autopilot crashed");
    assert_eq!(game.score(), 156);
    assert_eq!(game.horizon.obstacles[0].x.round() as i64, 26);
}

/// Jump when the next obstacle is about twenty frames away.
fn autopilot(game: &mut Game) {
    let reach = game.speed * 20.0;
    let trex_x = game.trex.x;
    if let Some(obstacle) = game.horizon.obstacles.first() {
        if obstacle.x - trex_x < reach && obstacle.x > trex_x {
            game.jump_press();
        }
    }
}
