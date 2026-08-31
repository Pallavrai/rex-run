# assets

`100-offline-sprite.png` is Chromium's offline game sprite sheet, copied
unmodified from
`components/neterror/resources/images/default_100_percent/offline/`.
It is the source for `src/sprites.rs`; regenerate with:

```bash
python3 tools/gen_sprites.py
```

The sheet is BSD-3-Clause licensed by The Chromium Authors -- see the
[NOTICE](../NOTICE) file at the repository root.

`screenshot.png` is captured from the real binary by `tools/screenshot.py`,
which plays the game inside a pseudo-terminal and renders the cells it emits:

```bash
cargo build --release
python3 tools/screenshot.py --bot --action --cols 120 --rows 30 --scale 6 \
    -o assets/screenshot.png
```
