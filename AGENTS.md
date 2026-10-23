# AGENTS.md

Guidance for AI agents and contributors working in this repository.

## What this is

A Rust remake of Friday Night Funkin'. The goal is a 1:1 recreation of how the game plays and looks:

- Game logic follows the legacy **0.2.8** source code (Apache-2.0). Port behavior from it, including its quirks, rather than inventing new behavior.
- Art, music, sounds and charts come from the **0.2.7.1** release (Apache-2.0) and live in `assets/`. When 0.2.8 code expects an asset that 0.2.7.1 lacks or names differently, adapt the code to the 0.2.7.1 asset (for example the note atlas prefixes `green0` and `arrowLEFT`, the `iconGrid` health icons, and the story week titles inside `campaign_menu_UI_assets`). Do not draw replacement art or generate placeholder assets.
- Engine behavior mirrors HaxeFlixel 4.9 and flixel-addons 2.10, since 0.2.8 is built on them.

## Rules

- Never write code comments of any kind: no inline or block comments, doc comments, TODOs or commented-out code.
- Never use em dashes in code, docs or messages.
- Every file must end right after its last visible character, with no trailing newline.
- Do not use Python. Use native OS tools and the Rust toolchain.
- Write plain, idiomatic Rust that matches the surrounding code. Avoid clever abstractions, nested ternaries and defensive boilerplate for unlikely cases.
- Keep the build free of warnings. Delete code that becomes unused instead of silencing it.
- Keep credits intact. `NOTICE` lists the original creators and must stay accurate.
- The frame rate is intentionally uncapped with vsync off. Do not add a frame limiter or an FPS setting.

## Layout

```
src/main.rs        Window setup (title, icon, vsync off)
src/engine/        HaxeFlixel-style engine
  app.rs           App loop, State trait, Ctx, state switching, transitions, sound tray, FPS counter
  gfx.rs           Batched quad renderer, camera layers, color transform and effect shader
  atlas.rs         Frames: single image, grid, Sparrow XML (lenient parser), packer TXT
  anim.rs          FlxAnimation and FlxAnimationController semantics
  sprite.rs        FlxSprite semantics: hitbox, offsets, scroll factor, motion, tweens, clipping
  camera.rs        Camera follow (LockOn with deadzone), zoom, flash and fade
  text.rs          FlxText-like text rendering
  tween.rs         Tweens and the easings in use
  timer.rs         FlxTimer
  audio.rs         kira based Sound and Audio, music, one-shots, flixel volume beep
  input.rs         Keyboard state with just pressed and just released
  assets.rs        Asset roots and caching
  save.rs          JSON save file
  math.rs          Lerp helpers and the random generator
src/game/          The game
  mod.rs           Boot, asset roots, GameData
  data.rs          Prefs, controls, highscores, session state
  play.rs          PlayState: chart loading, notes, input, scoring, camera, song events, cutscenes
  stage.rs         All 0.2.8 stages, background props and their beat logic, Thorns trail
  character.rs     Characters loaded from content/data/characters.json
  note.rs          Notes and strum arrows, including pixel variants
  dialogue.rs      Week 6 dialogue box
  alphabet.rs      Alphabet (menu letters) and AtlasText (options menu text)
  title.rs, main_menu.rs, story.rs, freeplay.rs, options.rs, pause.rs,
  game_over.rs, gitaroo.rs   Menus and sub screens
  weeks.rs         Week list, freeplay colors and icons
  song.rs, conductor.rs, library.rs, widgets.rs   Charts, timing, asset loading helpers, health bar and icons
assets/            Official 0.2.7.1 assets, do not modify
content/           VT323 font and the character table
```

## Character data

`content/data/characters.json` holds every character. Each animation is `[name, prefix, offsetX, offsetY, indices or null, loop, fps]`. Offsets come from the 0.2.7.1 `Character.hx`. Other keys: `image`, `dance` (`idle`, `leftright`, `none`), `sing_duration`, `icon_frames` (indices into `iconGrid`), `idle_loop`, `finish_to`, `hitbox_shrink`, `scale`, `antialiasing`, `flip_x`, `player_swaps`.

## Building and testing

```
cargo build
cargo run --release
```

A running `funkin-rs.exe` locks the binary on Windows, so stop it before rebuilding. F6 toggles autoplay in PlayState, which is the quickest way to watch a song without playing it. Freeplay reaches every song, and story mode is needed to see the week 5 and week 6 cutscenes.