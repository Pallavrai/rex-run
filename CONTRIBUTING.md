# Contributing to rex-run

Thanks for taking a look. Bug reports, terminal-compatibility notes and
patches are all welcome.

## The one rule that shapes everything

**rex-run is a port, not a remake.** Chrome's offline game is open source
([`components/neterror/resources/dino_game/`][source] in Chromium), and every
constant here -- gravity, jump velocity, obstacle gaps, collision boxes,
animation timings -- comes from that code. If a change makes the game feel
different from the browser's, it needs a reason beyond "it plays better this
way", and a comment saying what the terminal forced.

Where the terminal genuinely cannot follow Chrome, the deviation is documented
in the [README](README.md#how-it-works) and in a comment at the code that does
it. There are only a handful; keep it that way.

[source]: https://chromium.googlesource.com/chromium/src/+/main/components/neterror/resources/dino_game/

## Getting set up

```bash
git clone https://github.com/Pallavrai/rex-run
cd rex-run
cargo run
```

Before opening a pull request:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same three on Linux, macOS and Windows.

## Sprites are generated

`src/sprites.rs` is produced from Chromium's sprite sheet by
`tools/gen_sprites.py` and committed, so nothing is generated at build time and
contributors need no Python. Do not hand-edit it. If you change how sprites are
extracted:

```bash
pip install pillow
python3 tools/gen_sprites.py
```

CI regenerates the file and fails if the committed copy differs.

## Tests

`tests/game_logic.rs` covers the parts that break quietly: collision maths, the
score's rate, obstacle spacing, sprite unpacking, and a seeded 2400-frame run
whose final score is asserted exactly. That last one will fail on any change to
the physics -- which is the point. If your change is deliberate, update the
number in the same commit and say so in the pull request.

The game logic never touches the terminal, so all of it runs headless.

## Looking at the game without a terminal

`tools/screenshot.py` plays the real binary inside a pseudo-terminal and either
prints the frame as text or saves it as a PNG:

```bash
cargo build --release
python3 tools/screenshot.py --ascii --bot --cols 120 --rows 30 --seconds 20
```

Handy for rendering bugs, and for the README images. The demo GIF is recorded
from a real terminal instead:

```bash
asciinema rec demo.cast -c "cargo run --release"
agg demo.cast assets/demo.gif
```

## Things that are deliberately absent

Sound, arcade mode and alternative game modes are out of scope. So are new
dependencies: the game uses `crossterm` and nothing else, and a few lines of
standard library beat a crate for random numbers, argument parsing and finding
a data directory.
