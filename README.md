# Claudralingo

A typing tutor made with [Bevy](https://bevyengine.org) 0.20 and its
[feathers](https://docs.rs/bevy_feathers/0.20.0) widgets. It runs natively and in the browser.

## Roadmap

1. **Touch typing (now).** The exercises and the tutor logic of
   [Klavaro](https://sourceforge.net/projects/klavaro/) 3.14, ported to Rust, on an on-screen
   keyboard: a row staggered QWERTY or a split Corne 3x5+3 (36 keys) with layers.
2. **Chinese hanzi calligraphy (later).** Stroke order and stroke shapes of the hanzi, drawn with a
   mouse, a pen or a touch screen. Keep it in mind while building the touch typing:
   - the exercise is a list of "items to produce", for now characters typed on a keyboard, later
     strokes or characters drawn on a canvas;
   - the per item statistics (misses, timings, best and worst items) are not keyboard specific;
   - the fonts need CJK coverage, the feathers fonts (Fira) do not have it.

## Touch typing

The four Klavaro exercises, with their original texts and goals, and two of Claudralingo:

| Exercise         | Text                                                       | Goal                               |
| ---------------- | ---------------------------------------------------------- | ---------------------------------- |
| Basic course     | 43 lessons of random 5-letter "words" of a few keys        | 95% accuracy to unlock the next    |
| Adaptability     | random weird words, made of your worst keys when needed    | 98% accuracy                       |
| Velocity         | random words of Klavaro's dictionary                       | 95% accuracy, 50 WPM               |
| Fluidness        | paragraphs of a text                                       | 97% accuracy, 50 WPM, 70% fluidity |
| Finger drill     | the keys of one finger, up and down its column             | 95% accuracy                       |
| Shortcuts & keys | Esc, editing shortcuts, arrows, numbers, symbols, Alt+Tab  | every key on the first try         |

Differences to Klavaro:

- **Backspace works in every exercise.** Klavaro only allows it in the fluidness exercises
  (`tutor_eval_forward_backward`), the other ones beep. As in Klavaro's fluidness, a wrong key
  moves forward and blocks the progress until the errors are erased, only the errors can be
  erased, and the characters retyped after a correction count as errors. Ctrl+Backspace erases
  all the errors.
- The first key starts the timer, there is no "press any key" intro.
- The misses and touch times are recorded in the basic course too.
- At the end of an exercise Enter starts the next one, Backspace goes back one basic lesson, as in
  Klavaro.

The basic lessons are masks over the 4x14 key grid of Klavaro's `qwerty_us.kbd`, their characters
are the same for both layouts: on the Corne the digits and most symbols are on the layers.

### The keyboard on the screen

The next key is green, the shift to hold purple, the layer and the modifier keys to hold amber
(with the legends of that layer), backspace red while there are errors to erase. The stripe at
the bottom of each key is the finger that types it.

- **Fade legends** (on by default): every exercise starts with the legends and the hints shown,
  they fade out over the first 20 correct keys, only the layout and the finger colors stay. A
  wrong key, or a pause of 2 seconds, shows the next key again, green and with its legend, until
  it is typed. That is the way to learn a new layout (Dvorak, Colemak, a Corne keymap): from
  memory, with help only when stuck.
- **Layer trainer** (on by default): a character of another layer, or a chord with Ctrl, Alt or
  Super, always shows the keys to hold and the key to press, with the instructions above the
  keyboard, i.e. "Hold Lower (left thumb), then press ! where A is (left pinky, home row)".

### Finger drills

One drill per finger, ‹ and › pick the finger: the pinky goes up and down its column (q a z), the
index fingers also move sideways between their two columns, the thumbs drill is short words
between Space and Enter. At the end of the other exercises, a key missed 3 times or more, or a
finger below 92% accuracy over all the exercises, suggests the drill of that finger: Tab or the
button starts it. The weakest fingers are listed under the keyboard.

### Shortcuts & keys

A round of 21 tasks, each one a key or a chord to press, with the keys to hold shown on the
keyboard (the shift of the symbols is part of the symbol):

- Esc;
- a line of text with a cursor, where the shortcuts do what they do in an editor: Home, Ctrl+→,
  Ctrl+Shift+→ to select a word, Ctrl+C, End, Ctrl+V, Ctrl+Z, Ctrl+Shift+←, Ctrl+X,
  Ctrl+Backspace, Backspace, Ctrl+A, Delete;
- the arrows, the ten digits and ten random symbols, like `. / @ # $ % ^ & * ( ) { } ? + = \ -`,
  in a random order;
- Tab and Shift+Tab;
- Alt+Tab to another window and back: the window notices that it lost the focus with Alt held,
  and times the round trip until it has the focus again. The best round trip is saved.

A wrong chord changes nothing, it counts as a miss of the task and the chord is shown again.

### The Corne keymap

The Corne keymap is a placeholder until it matches the real one:

- Base: QWERTY, home row mods Super, Alt, Ctrl on A S D and ; L K;
- thumbs: `Shift Lower Space` on the left, `Enter Raise Backspace` on the right;
- Lower: digits on the top row, shifted digits on the home row, `` ` ~ ' " \ `` and `| - _ = +`
  on the bottom row;
- Raise: `Esc Tab Home End Del` on the left top row, `[ ] { }` on the right top row, the arrows on
  H J K L, `PgUp PgDn Ins` on N M ,.

See `corne36` in `src/layout.rs`. The QWERTY layout has the usual touch typing fingers, the
modifiers, Esc and the navigation keys.

## Progress

Native builds save the progress to `~/.config/claudralingo/progress.json` (`$XDG_CONFIG_HOME` if
set), the browser to its `localStorage`. Per layout:

- `keys`: the hits, misses and touch times of every character,
- `best_keys` and `most_missed`: the rankings shown in the window,
- `basic_lesson`, the `finger` of the finger drill, the finished exercises in `sessions`,
- `shortcuts`: the hits, misses and times of every chord of the shortcuts, `alt_tab_best`,
- `klavaro`: Klavaro's own error and touch time accumulators (`accuracy.log` and
  `proficiency.log` in Klavaro), they generate the adaptability words.

The theme (dark by default), the layout, the exercise, `fade_legends` and `layer_trainer` are
saved too.

## Build

The `Cargo.toml` has no `bevy/dynamic_linking`: the release binaries are built on a desktop and
copied to other machines.

```sh
cargo run --release                      # native
cargo test                               # the Klavaro port and the layouts
trunk serve --release                    # browser, http://127.0.0.1:8080
trunk build --release                    # browser, into dist/
```

`CLAUDRALINGO_SCREENSHOT=shot.png CLAUDRALINGO_TYPE="fj fk<" cargo run` types the given keys
(`<` is backspace, a tab is Tab, `*` presses the expected key or chord, and fakes the Alt+Tab
round trip) and saves a screenshot, to check the screen without a keyboard.

## Credits and license

- The exercises (`data/klavaro`) and the tutor logic come from Klavaro 3.14, GPL-3.0-or-later,
  Copyright (C) 2005-2021 Felipe Emmanuel Ferreira de Castro and the Klavaro authors
  (`data/klavaro/AUTHORS`). Claudralingo is therefore GPL-3.0-or-later too, see `LICENSE`.
- The look and the animations of the keyboard are inspired by
  [Layer Tutor](https://layer-tutor.pages.dev/), no code was copied.
- The widgets, the Fira fonts and the dark theme come from bevy_feathers.
