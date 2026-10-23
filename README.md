# funkin-rs

A Rust remake of Friday Night Funkin'. The game logic is ported from the legacy 0.2.8 source code and plays the same way, while the art, music, sounds and charts come from the 0.2.7.1 release. Both are licensed under the Apache License 2.0.

Everything from 0.2.8 up to week 6 is here: the title screen, main menu, story mode, freeplay, options, pause menu, game over, all seven stages with their background animations, the week 5 and week 6 cutscenes, the week 6 dialogue and pixel UI, and the song-specific events.

## Running

```
cargo run --release
```

The game reads two folders:

- `assets` holds the official images, music, sounds and charts.
- `content` holds the bundled font and `data/characters.json`, the character table (atlas, animation prefixes, offsets and health icon frames).

During development both are read from the crate directory. A shipped build looks for them next to the executable. Extra folders can be added with the `FUNKIN_CONTENT` environment variable (a path list). Files in those folders override the built-in ones.

Settings, key bindings and high scores are saved to `%APPDATA%\funkin-rs\save.json` on Windows, or `~/.local/share/funkin-rs/save.json` elsewhere.

## Controls

| Action | Keys |
| --- | --- |
| Notes | Arrow keys or WASD |
| Accept | Enter, Space, Z |
| Back | Escape, Backspace, X |
| Pause | Enter, Escape, P |
| Reset (instant game over) | R |
| Volume up / down / mute | `=` / `-` / `0` |
| Fullscreen | F11 (F on the title screen) |
| Old boyfriend health icon | 9 |
| Autoplay | F6 |

Note and menu keys can be rebound in Options.

## Differences from 0.2.8

- Week 7 is not included because its assets are not part of 0.2.7.1.
- There are no note splashes.
- The game uses VT323 in place of the VCR OSD Mono and Pixel Arial fonts, which are not included.
- The frame rate is uncapped and vsync is off.
- Newgrounds, Discord, the chart editor and the other debug screens are left out. F6 toggles autoplay instead.

## Project layout

```
src/engine   HaxeFlixel-style engine: sprites, atlases, animation, cameras,
             tweens, timers, text, audio, input, saving and state switching
src/game     The game: states, PlayState, stages, characters, notes, menus
assets       Official 0.2.7.1 assets
content      Bundled font and the character table
```

## License

The code is licensed under the Apache License 2.0, see `LICENSE`. Credits for the game, its assets and the bundled font are in `NOTICE`.