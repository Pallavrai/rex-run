# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-31

First release.

### Added

- Chrome's offline T-Rex game in the terminal, drawn with the browser's own
  sprites at half size and rendered with half-block characters.
- Physics, obstacle tables, collision boxes and speed curve ported from
  Chromium's `dino_game`.
- Cacti, pterodactyls, clouds, moon phases, stars and night mode.
- Score and high score, kept between sessions.
- Key-release support (short hops, quick ducks) on terminals that speak the
  kitty keyboard protocol, with an auto-repeat fallback elsewhere.
- `--slow` for Chrome's slow mode.

[Unreleased]: https://github.com/Pallavrai/rex-run/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Pallavrai/rex-run/releases/tag/v0.1.0
