//! A 3x5 pixel font for the score and the game-over panel.
//!
//! Chrome draws its HUD from the sprite sheet, but those glyphs are 10x13 --
//! at the half size everything else is drawn at they turn to mush. So the
//! text is drawn in the framebuffer with a font small enough to survive:
//! solid blocks, no anti-aliasing, and it inverts with night mode like the
//! rest of the world.

/// Width of a glyph in pixels; [`ADVANCE`] adds the gap between them.
pub const WIDTH: i32 = 3;
pub const HEIGHT: i32 = 5;
pub const ADVANCE: i32 = WIDTH + 1;

/// Pixel width of `text` at a given scale.
pub fn width(text: &str, scale: i32) -> i32 {
    let chars = text.chars().count() as i32;
    (chars * ADVANCE - 1).max(0) * scale
}

/// The rows of a glyph, top to bottom. Unknown characters draw as a space.
pub fn glyph(c: char) -> &'static [&'static str; HEIGHT as usize] {
    const SPACE: &[&str; 5] = &["   ", "   ", "   ", "   ", "   "];
    GLYPHS
        .iter()
        .find(|(ch, _)| *ch == c.to_ascii_uppercase())
        .map_or(SPACE, |(_, rows)| rows)
}

#[rustfmt::skip]
static GLYPHS: &[(char, [&str; HEIGHT as usize])] = &[
    ('0', ["###", "# #", "# #", "# #", "###"]),
    ('1', [" # ", "## ", " # ", " # ", "###"]),
    ('2', ["###", "  #", "###", "#  ", "###"]),
    ('3', ["###", "  #", "###", "  #", "###"]),
    ('4', ["# #", "# #", "###", "  #", "  #"]),
    ('5', ["###", "#  ", "###", "  #", "###"]),
    ('6', ["###", "#  ", "###", "# #", "###"]),
    ('7', ["###", "  #", "  #", "  #", "  #"]),
    ('8', ["###", "# #", "###", "# #", "###"]),
    ('9', ["###", "# #", "###", "  #", "###"]),
    ('A', [" # ", "# #", "###", "# #", "# #"]),
    ('B', ["## ", "# #", "## ", "# #", "## "]),
    ('C', ["###", "#  ", "#  ", "#  ", "###"]),
    ('D', ["## ", "# #", "# #", "# #", "## "]),
    ('E', ["###", "#  ", "###", "#  ", "###"]),
    ('F', ["###", "#  ", "###", "#  ", "#  "]),
    ('G', ["###", "#  ", "# #", "# #", "###"]),
    ('H', ["# #", "# #", "###", "# #", "# #"]),
    ('I', ["###", " # ", " # ", " # ", "###"]),
    ('J', ["  #", "  #", "  #", "# #", "###"]),
    ('K', ["# #", "# #", "## ", "# #", "# #"]),
    ('L', ["#  ", "#  ", "#  ", "#  ", "###"]),
    ('M', ["# #", "###", "###", "# #", "# #"]),
    ('N', ["# #", "## ", "###", " ##", "# #"]),
    ('O', ["###", "# #", "# #", "# #", "###"]),
    ('P', ["###", "# #", "###", "#  ", "#  "]),
    ('Q', ["###", "# #", "# #", "###", "  #"]),
    ('R', ["###", "# #", "## ", "# #", "# #"]),
    ('S', ["###", "#  ", "###", "  #", "###"]),
    ('T', ["###", " # ", " # ", " # ", " # "]),
    ('U', ["# #", "# #", "# #", "# #", "###"]),
    ('V', ["# #", "# #", "# #", "# #", " # "]),
    ('W', ["# #", "# #", "###", "###", "# #"]),
    ('X', ["# #", "# #", " # ", "# #", "# #"]),
    ('Y', ["# #", "# #", " # ", " # ", " # "]),
    ('Z', ["###", "  #", " # ", "#  ", "###"]),
    ('·', ["   ", "   ", " # ", "   ", "   "]),
    ('.', ["   ", "   ", "   ", "   ", " # "]),
    ('-', ["   ", "   ", "###", "   ", "   "]),
    (':', ["   ", " # ", "   ", " # ", "   "]),
];
