//! Half-block renderer over a 1-bit framebuffer.
//!
//! A terminal cell is roughly twice as tall as it is wide, so drawing `▀`
//! with a foreground and a background colour gives two square pixels per
//! cell. The whole game is one ink colour on one paper colour -- exactly
//! like Chrome's sprite sheet -- so the framebuffer is a bitmap and night
//! mode is a colour swap rather than a second set of assets.

use std::io::{self, Write};

use crossterm::style::Color;
use crossterm::{cursor, queue, style};

use crate::font;

/// Marks a cell whose painted contents are unknown, forcing a repaint.
const DIRTY: u8 = 0xff;

/// Chrome's `#535353` on `#f7f7f7`, and the same two colours inverted --
/// which is what the browser does to the canvas when night mode kicks in.
const DAY: (Rgb, Rgb) = ((0x53, 0x53, 0x53), (0xf7, 0xf7, 0xf7));
const NIGHT: (Rgb, Rgb) = ((0xac, 0xac, 0xac), (0x08, 0x08, 0x08));

/// Milliseconds to cross-fade between day and night.
const FADE_MS: f64 = 400.0;

type Rgb = (u8, u8, u8);

/// A 1-bit sprite with transparency: `mask` says whether a pixel is drawn at
/// all, `ink` whether it is ink or paper. Bits are row-major, MSB first.
pub struct Sprite {
    pub w: u16,
    pub h: u16,
    pub mask: &'static [u8],
    pub ink: &'static [u8],
}

impl Sprite {
    #[inline]
    fn bit(data: &[u8], i: usize) -> bool {
        data[i >> 3] & (0x80 >> (i & 7)) != 0
    }

    /// `None` where the sprite is transparent, otherwise whether it is ink.
    #[inline]
    pub fn pixel(&self, x: u16, y: u16) -> Option<bool> {
        let i = y as usize * self.w as usize + x as usize;
        Self::bit(self.mask, i).then(|| Self::bit(self.ink, i))
    }
}

pub struct Screen {
    cols: u16,
    rows: u16,
    /// `cols * rows * 2` pixels, true where ink.
    px: Vec<bool>,
    /// Last painted state per cell, or [`DIRTY`].
    prev: Vec<u8>,
    /// Text drawn over the pixels, collected during a frame.
    hud: Vec<(u16, u16, String)>,
    night: f64,
    night_target: f64,
}

impl Screen {
    pub fn new(cols: u16, rows: u16) -> Self {
        let mut screen = Screen {
            cols: 0,
            rows: 0,
            px: Vec::new(),
            prev: Vec::new(),
            hud: Vec::new(),
            night: 0.0,
            night_target: 0.0,
        };
        screen.resize(cols, rows);
        screen
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.px = vec![false; cols as usize * rows as usize * 2];
        self.prev = vec![DIRTY; cols as usize * rows as usize];
    }

    /// Height of the framebuffer in pixels.
    pub fn height(&self) -> i32 {
        self.rows as i32 * 2
    }

    pub fn clear(&mut self) {
        self.px.fill(false);
        self.hud.clear();
    }

    pub fn set_night(&mut self, night: bool) {
        self.night_target = if night { 1.0 } else { 0.0 };
    }

    #[inline]
    fn set(&mut self, x: i32, y: i32, ink: bool) {
        if x >= 0 && y >= 0 && x < self.cols as i32 && y < self.height() {
            self.px[y as usize * self.cols as usize + x as usize] = ink;
        }
    }

    /// Draw text into the framebuffer with the pixel font, `scale` pixels
    /// per font pixel. Returns nothing: it clips like any other blit.
    pub fn blit_text(&mut self, x: i32, y: i32, text: &str, scale: i32) {
        for (i, c) in text.chars().enumerate() {
            let left = x + i as i32 * font::ADVANCE * scale;
            for (row, pixels) in font::glyph(c).iter().enumerate() {
                for (col, pixel) in pixels.chars().enumerate() {
                    if pixel != ' ' {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                let px = left + col as i32 * scale + dx;
                                self.set(px, y + row as i32 * scale + dy, true);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draw a sprite with its top-left corner at `(x, y)`, clipped.
    pub fn blit(&mut self, sprite: &Sprite, x: i32, y: i32) {
        for sy in 0..sprite.h {
            let py = y + sy as i32;
            if py < 0 || py >= self.height() {
                continue;
            }
            for sx in 0..sprite.w {
                if let Some(ink) = sprite.pixel(sx, sy) {
                    self.set(x + sx as i32, py, ink);
                }
            }
        }
    }

    /// Queue text at a cell position. Drawn over the pixels each frame.
    pub fn text(&mut self, col: u16, row: u16, text: &str) {
        if row < self.rows {
            self.hud.push((col, row, text.to_string()));
        }
    }

    /// Same, horizontally centred.
    pub fn text_centered(&mut self, row: u16, text: &str) {
        let col = (self.cols as usize).saturating_sub(text.chars().count()) / 2;
        self.text(col as u16, row, text);
    }

    fn colors(&self) -> (Color, Color) {
        let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * self.night).round() as u8;
        let mix = |a: Rgb, b: Rgb| Color::Rgb {
            r: lerp(a.0, b.0),
            g: lerp(a.1, b.1),
            b: lerp(a.2, b.2),
        };
        (mix(DAY.0, NIGHT.0), mix(DAY.1, NIGHT.1))
    }

    /// Paint every cell that changed since the last frame.
    pub fn present(&mut self, out: &mut impl Write, dt: f64) -> io::Result<()> {
        if self.night != self.night_target {
            let step = dt / FADE_MS;
            self.night = if self.night < self.night_target {
                (self.night + step).min(self.night_target)
            } else {
                (self.night - step).max(self.night_target)
            };
            // Every cell's colour just moved; nothing can be reused.
            self.prev.fill(DIRTY);
        }

        let (ink, paper) = self.colors();
        let mut pen: Option<(Color, Color)> = None;
        let mut at: Option<(u16, u16)> = None;

        for row in 0..self.rows {
            for col in 0..self.cols {
                let cell = row as usize * self.cols as usize + col as usize;
                let top = self.px[row as usize * 2 * self.cols as usize + col as usize];
                let bottom = self.px[(row as usize * 2 + 1) * self.cols as usize + col as usize];
                let state = top as u8 | (bottom as u8) << 1;
                if self.prev[cell] == state {
                    continue;
                }
                self.prev[cell] = state;

                if at != Some((col, row)) {
                    queue!(out, cursor::MoveTo(col, row))?;
                }
                let colors = (
                    if top { ink } else { paper },
                    if bottom { ink } else { paper },
                );
                if pen != Some(colors) {
                    queue!(
                        out,
                        style::SetForegroundColor(colors.0),
                        style::SetBackgroundColor(colors.1)
                    )?;
                    pen = Some(colors);
                }
                queue!(out, style::Print('▀'))?;
                at = Some((col + 1, row));
            }
        }

        for (col, row, text) in std::mem::take(&mut self.hud) {
            queue!(
                out,
                cursor::MoveTo(col, row),
                style::SetForegroundColor(ink),
                style::SetBackgroundColor(paper),
                style::Print(&text)
            )?;
            let start = row as usize * self.cols as usize + col as usize;
            let len = text.chars().count().min(self.cols as usize - col as usize);
            self.prev[start..start + len].fill(DIRTY);
            pen = None;
        }
        let _ = pen;

        out.flush()
    }
}
