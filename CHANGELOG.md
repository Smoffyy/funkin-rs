# Changelog

All notable changes to this project are listed here. Each release has its own section, and the release workflow publishes the section matching the version in `Cargo.toml`.

## [1.1.0] - TBD

### Added
- Engine page in Options with an Engine Features switch that turns off all engine-only settings at once, returning the game to base behavior

### Changed
- Opponent note strums, ghost tapping, quick transitions, antialiasing and preload assets moved from Preferences and Graphics to the Engine page

### Fixed
- Girlfriend and other bopping characters no longer skip or repeat a dance beat during a song

## [1.0.1] - 2026-10-24

### Added
- Separate vocals in the chart editor. The song and difficulties that use their own music each get a Separate vocals toggle with Player file and Opponent file fields. The waveform shows the side being charted, and Player and Opponent volume sliders appear when separate vocals are loaded

### Fixed
- Difficulties with separate vocals play their own player and opponent vocals instead of the song's vocals
- Windows release no longer opens a blank terminal window alongside the game
- Windows release no longer needs the Visual C++ runtime installed, fixing the VCRUNTIME140.dll error
- macOS release is packaged as funkin-rs.app, so it opens without a Terminal window
- macOS release runs on Intel Macs as well as Apple Silicon
- Linux release runs on older distributions

## [1.0.0] - 2026-10-24

### Added
- Chart editor with multiple difficulties
- Quick transitions option
- More settings in Options
- Preload Assets option in Options > Graphics, off by default. When on, sprites, audio and charts load in the background at launch for smoother menus and songs
- Executable icon on Windows
- Release builds for Windows, macOS and Linux

### Fixed
- Missed inputs
- Chart editor bug
- Week 6 silhouette
- Note tails at higher scroll speeds