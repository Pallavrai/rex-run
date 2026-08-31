//! Chrome's offline T-Rex game, ported to the terminal.
//!
//! The game logic is terminal-free and lives in [`game`], [`trex`],
//! [`obstacle`] and [`horizon`]; [`render`] turns a 1-bit framebuffer into
//! half-block characters. Sprites in [`sprites`] are generated from
//! Chromium's sprite sheet by `tools/gen_sprites.py`.

pub mod game;
pub mod horizon;
pub mod obstacle;
pub mod render;
pub mod score;
pub mod sprites;
pub mod trex;
