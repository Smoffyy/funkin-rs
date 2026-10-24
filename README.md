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
| Chart editor | 7 (in freeplay or during a song) |

Note and menu keys can be rebound in Options. The pause menu has a **Settings** entry that opens the options mid-song; when you go back, the song counts down 3, 2, 1 and continues from where you paused, with any changes applied.

## Differences from 0.2.8

- Week 7 is not included because its assets are not part of 0.2.7.1.
- There are no note splashes.
- The game uses VT323 in place of the VCR OSD Mono and Pixel Arial fonts, which are not included.
- The frame rate is uncapped and vsync is off.
- Newgrounds, Discord and the old debug screens are left out. F6 toggles autoplay instead.
- Two optional settings, both off by default: **Opponent Note Strums** lights up the opponent's arrows when they hit notes, and **Ghost Tapping** stops presses without a note from counting as misses.
- A new chart editor replaces the 0.2.8 one.

## Chart editor

Open it from **Options > Chart Editor** to start a new chart, press **7** on a song in freeplay, or press **7** during a song to edit that song. Press **F1** inside the editor for every shortcut.

- Click a lane to place a note and drag down to make it a hold. Right click deletes, right drag box selects, and dragging a selection moves it in time and across lanes.
- Snap from 1/4 to 1/192, zoom, a song minimap, waveforms, hit sounds, a metronome and a playback rate from 0.25x to 2x.
- Undo and redo, copy, cut, paste and duplicate, swapping sides, flipping lanes, and section tools (camera focus, BPM changes, alt animations, clear, duplicate, copy from earlier sections).
- Live charting: press 1-8 while the song plays to drop notes on the playhead.
- Opponent and player previews that sing the notes as the playhead passes them, whether the song is playing or you are scrolling.
- Playtest from the start (Enter) or from the playhead (Shift+Enter). Leaving the playtest returns you to the editor with your changes intact.
- Custom difficulties: press **+** next to the difficulty tabs (or open the **Diff** tab) to add one, optionally starting from the current notes. Each difficulty can use its own music and its own BPM and events, and difficulties can be renamed, copied from one another or deleted.
- Saves with Ctrl+S, plus an autosave every two minutes when there are unsaved changes.

## Charts

Charts are saved to a `charts` folder, one folder per song, next to `assets`. A chart folder holds `chart.json` and, for new songs, `Inst.ogg` and `Voices.ogg`. With Separate vocals turned on in the editor, the song's player and opponent vocals are named in `chart.json` by `player_voices` and `opponent_voices`, usually `Voices-player.ogg` and `Voices-opponent.ogg`. Charts in this folder override the official charts, and new songs show up in freeplay automatically. The official 0.2.7.1 charts are read directly from `assets` and converted on load, so they are never modified.

`chart.json` is a readable JSON file:

```json
{
	"format": "funkin-rs-chart",
	"version": 2,
	"song": {
		"name": "Bopeebo",
		"artist": "",
		"charter": "",
		"stage": "stage",
		"player": "bf",
		"opponent": "dad",
		"voices": true,
		"offset": 0
	},
	"timing": [
		{"time": 0, "bpm": 100}
	],
	"events": [
		{"time": 0, "name": "focus", "target": "opponent"},
		{"time": 9600, "name": "play_animation", "target": "player", "animation": "hey"}
	],
	"difficulties": [
		{
			"name": "normal",
			"speed": 1.4,
			"notes": [
				{"time": 600, "side": "opponent", "lane": "left", "length": 0},
				{"time": 1200, "side": "player", "lane": "up", "length": 300, "type": "alt"}
			]
		},
		{
			"name": "nightmare",
			"speed": 2.1,
			"audio": {"inst": "Inst-nightmare.ogg", "voices": "Voices-nightmare.ogg"},
			"timing": [
				{"time": 0, "bpm": 180}
			],
			"events": [],
			"notes": []
		}
	]
}
```

Times and lengths are in milliseconds. `stage` and `girlfriend` are optional and fall back to the song's original stage. `timing` lists BPM changes.

`difficulties` is a list, so a song can have any number of them with any names. `easy`, `normal` and `hard` are what the official songs and story mode use, and anything else (like `nightmare`) is a custom difficulty that appears in freeplay and the pause menu. Each difficulty has its own scroll speed and notes, and can optionally have:

- `audio`: its own `inst` and `voices` files in the chart folder, so a difficulty can play a different song underneath. Without it the difficulty uses the song's music. Optional `player_voices` and `opponent_voices` files hold separate vocals for that difficulty.
- `timing` and `events`: its own BPM changes and events, for music with a different tempo. Without them it uses the song-level ones.

Events are a time, a name and any parameters. The built-in ones are:

| Event | Parameters | Effect |
| --- | --- | --- |
| `focus` | `target`: `player` or `opponent` | Moves the camera, replacing the old per-section setting |
| `play_animation` | `target`: `player`, `opponent` or `girlfriend`, `animation` | Plays an animation on a character |
| `camera_bump` | `game`, `hud` | Adds a zoom bump like the beat zoom |
| `camera_zoom` | `zoom` | Changes the stage's resting zoom |
| `gf_speed` | `speed` | Sets how often GF bops |

## Project layout

```
src/engine   HaxeFlixel-style engine: sprites, atlases, animation, cameras,
             tweens, timers, text, audio, input, saving and state switching
src/game     The game: states, PlayState, stages, characters, notes, menus,
             and the chart editor in src/game/editor
assets       Official 0.2.7.1 assets
content      Bundled font and the character table
charts       Saved and custom charts (created by the editor)
```

## License

The code is licensed under the Apache License 2.0, see `LICENSE`. Credits for the game, its assets and the bundled font are in `NOTICE`.