# Claudralingo

A typing tutor made with [Bevy](https://bevyengine.org) 0.20 and its
[feathers](https://docs.rs/bevy_feathers/0.20.0) widgets. It runs natively and in the browser.

## Roadmap

1. **Touch typing.** The exercises and the tutor logic of
   [Klavaro](https://sourceforge.net/projects/klavaro/) 3.14, ported to Rust, on an on-screen
   keyboard: a row staggered QWERTY, the split Corne 3x5+3 and 3x6+3, the Lily58, or your own
   ZMK keymap, with layers.
2. **Chinese hanzi (now, HSK 1 to 4).** The words written stroke by stroke with a pen, a mouse or
   a finger, then typed in pinyin, see [Hanzi](#hanzi).

## Touch typing

The four Klavaro exercises, with their original texts and goals, and two of Claudralingo:

| Exercise         | Text                                                       | Goal                               |
| ---------------- | ---------------------------------------------------------- | ---------------------------------- |
| Basic course     | 43 lessons of random 5-letter "words" of a few keys        | 95% accuracy to unlock the next    |
| Adaptability     | random weird words, made of your worst keys when needed    | 98% accuracy                       |
| Velocity         | random words of Klavaro's dictionary                       | 95% accuracy, 50 WPM               |
| Fluidness        | paragraphs of a text                                       | 97% accuracy, 50 WPM, 70% fluidity |
| Finger drill     | the keys of one finger, up and down its column             | 95% accuracy                       |
| Finger bounce    | middle and index of one hand in turns, alternating case    | 95% accuracy, slower is expected   |
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
  Klavaro. A basic lesson at 95% or more unlocks the next one by itself, as in Klavaro.
- The end of an exercise shows how it went: fireworks for a perfect one, sparkles when the goal is
  reached, and when it is not, the most missed keys red and breathing on the keyboard (with their
  characters, on any layer) until the next exercise, named in the comment too. Reduce motion
  keeps the red keys still and skips the fireworks.

The basic lessons are masks over the 4x14 key grid of Klavaro's `qwerty_us.kbd`, their characters
are the same for both layouts: on the Corne the digits and most symbols are on the layers.

### The keyboard on the screen

The next key is green, the shift to hold purple, the layer and the modifier keys to hold amber
(with the legends of that layer), backspace red while there are errors to erase. The stripe at
the bottom of each key is the finger that types it. The hands under the keyboard have the same
colors: the finger of the next key lifts and glows, the fingers holding a shift, a layer or a
modifier key are outlined in amber.

- **Fade legends** (on by default): every exercise starts with the legends and the hints shown,
  they fade out over the first 20 correct keys, only the layout and the finger colors stay. A
  wrong key, or a pause of 2 seconds, shows the next key again, green and with its legend, until
  it is typed. That is the way to learn a new layout (Dvorak, Colemak, a Corne keymap): from
  memory, with help only when stuck.
- **Layer trainer** (on by default): a character of another layer, or a chord with Ctrl, Alt or
  Super, always shows the keys to hold and the key to press, with the instructions above the
  keyboard, i.e. "Hold Lower (left thumb), then press ! where A is (left pinky, home row)".

### Colors, background and settings

The finger colors and the colors of the Lower and Raise layers are the validated categorical
palette of the dataviz reference (blue, orange, aqua, yellow, magenta for the pinky to the thumb,
green for Lower, violet for Raise), in the order of the physical neighbors: neighboring fingers and
the thumb layers stay apart for the color blind (CVD ΔE ≥ 8.4) and for normal vision (ΔE ≥ 19).
Red stays the erase hint. The colors are never alone: the legends and the texts name the fingers
and the layers.

The background is the edges of a slowly drifting low poly mesh, its triangles join and dissolve.
Its vertices wear the colors of the most missed keys: the finger color, or the layer color when the
key is on Lower or Raise; "Focus" under "Your keys" names them. Under the text to type and the
hands the lines blur away.

Settings (toolbar): the keyboard, dark mode, the background (off, subtle, lively), fade legends,
layer trainer, the hands, reduce motion (no drift, sparks, shakes nor breathing keys), a large
text to type and the zoom. By default the window is scaled to fit its size, a big screen shows
everything bigger; the zoom adds to that (50% to 300% in all, saved) with the buttons, Ctrl and the
mouse wheel, a trackpad pinch, Ctrl+= Ctrl+- Ctrl+0 (in the browser its own zoom does that). The
wheel scrolls the window when it does not fit.

Basic course on a split keyboard: the lessons of Klavaro pick the keys by their QWERTY place, the
Corne takes the places of the keys under the same fingers, so the lessons train its base layer
first (lesson 4 is `a '`, the home row pinky); the number row and the symbols come in the later
lessons, on the layers. A modifier in a place (Ctrl where QWERTY has `,`) brings the home keys of
the fingers instead. The layer trainer highlights a layer combination on the keyboard, the words
above it are for the auto shift and the modifiers.

Lessons (toolbar): the 43 lessons of the basic course with the best and the last accuracy and the
times typed, green when the best reached 95%, red below; a click practices a lesson again. The
progress is saved in `~/.config/claudralingo/progress.json` (in the browser, its local storage).

At the end of an exercise the accuracy is green when the goal is reached, red otherwise. A line
typed without an error clears like a row of Tetris with a small firework; a perfect exercise ends
with fireworks, sparks and congratulations.

### Finger drills

One drill per finger, ‹ and › pick the finger: the pinky goes up and down its column (q a z), the
index fingers also move sideways between their two columns, the thumbs drill is short words
between Space and Enter. At the end of the other exercises, a key missed 3 times or more, or a
finger below 92% accuracy over all the exercises, suggests the drill of that finger: Tab or the
button starts it. The weakest fingers are listed under the keyboard.

### Finger bounce

"Left hand finger bounce" and "Right hand finger bounce" train the independence of the middle and
the index fingers, ‹ › switch the hand. The middle finger presses its top row key, the index runs
down its home column (top, home, bottom row), the middle finger goes down one row, the index runs
again, then the same for the inner column of the index: `eRfV DrFv cRfV EtGb dTgB CtGb` on the
left hand. The next lines bounce in a random order. The case alternates on every letter: on the
Corne every other key is held a moment longer for its auto shift, the speed drops, that is part
of the exercise.

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

The Corne 3x5+3 is the default 5 column keymap of the Typeractive wireless Corne
(<https://docs.typeractive.xyz/build-guides/corne-wireless/firmware>, `config/corne.keymap` of
[corne-wireless-5-col-zmk-config](https://github.com/typeractivexyz/corne-wireless-5-col-zmk-config)),
read from its ZMK bindings by `src/zmk.rs`, with the key positions of ZMK's
`foostan_corne_5col_layout`: the column stagger, the inner thumbs 1.5 keys tall and turned by 12°
and 24°.

- Base: QWERTY with auto shift (a key held a moment longer types its shifted character, the key
  turns purple), `' .` on the right pinky, Ctrl and Alt on the right bottom row;
- thumbs: `GUI Lower Space` on the left, `Enter Raise Backspace` on the right;
- Lower: `! @ # $ % ^ & * ( )`, the Bluetooth profiles, the arrows on H J K L, Esc and Tab on the
  right thumbs;
- Raise: the digits, `- = { [ ( ) ] } / \`, `` ` , : ; . ``, Esc on the left thumb, Tab on the
  right one.

The keyboard on the screen is an on-screen keyboard too: a click types the key, a long press
(200 ms, the tapping term of the auto shift) types it shifted. A click on Lower or Raise shows
and uses its layer until the next click, holding it uses the layer while held (with a second
finger on a touch screen). The keys that only fall through to the base layer (`&trans`, like the
right pinky `.` on Raise, next to the `.` of Raise on M) are dimmed. When the next key is not on
the layer shown, the hint points at the layer key to click, not at the key that types something
else there. A click on Ctrl, Alt, Win or Shift applies it to the next clicked key: the clicks
only make key presses for the game, nothing is sent to the system, Win does not open its menu. The keymap has no Shift, Home, End nor Delete key: the shortcuts round uses ↑, ↓ and
Backspace for Home, End and Delete, and does the Ctrl+Shift selections for you.

The keys are drawn by a small UI material (`src/key.wgsl`) that anti-aliases their edges inside
the node: bevy_ui draws the shapes up to the edges of their quads, rotated quads get jagged.
Other keyboards (Lily58, Corne 6 columns, tenkeyless, full size) are a physical layout and a
keymap away, see `corne36` in `src/layout.rs`.

## Layer course, real words and warm-up

- **Layer course** (mode): the keys of each layer row by row (Lower: top row, home row...), then
  the layer whole, then all the layers. A keyboard without layers has its symbols instead.
- **Real words**: the basic lessons end with two lines of words of Klavaro's dictionary made of
  the keys learned so far, once there are enough of them.
- **Welcome back**: at the start, continue where the last session stopped (Enter), or warm up
  first on the keys that need it most, their misses weighing more when they were not practiced
  for a while (a spaced repetition), then back to the lesson. The warm-up is also in the menu.
- **Locked lessons** (setting, on by default): a lesson of the basic and the layer course, and a
  hanzi level, opens when the previous one is passed.
- **Auto shift**: a capital by a key held too long, or a capital missed, is named above the
  keyboard and counted in the stats. On the screen, a bar under a pressed key fills to the
  tapping term of the auto shift.
- **Encouragements**: a line without errors clears like a row of Tetris with a word ("GREAT!",
  "Are you a bot?"), a line with one slip clears in orange ("Almost!", "So close!"); a miss
  streams lines out of the finger that should have pressed the key, in its color.

## At the end of an exercise

A wrong key moves on, the keys typed after it stay red until Backspace brings the cursor back
to the last right character: the motion to learn on a Corne. A slip is one error, however many
keys follow it. Settings, "Fix mistakes with Backspace" off: the cursor waits on the missed key
instead. At the end, the bar at the top shows the
words per minute, the accuracy and the fluidness, with ▲ (better), ▼ (worse) or ● (the same)
against the last time of the same lesson; a new best speed for the lesson is congratulated with
the gain and the date of the previous record. The keys of the moment (Enter, Backspace, Tab,
← → for the lessons) are in the bar at the bottom.

## Themes and the compact view

Settings, Theme: Dark, Light, Tokyo Night, Gruvbox (dark) and Matrix (green on black, the code
falling in the background, no hands). "Compact" (on by default on Android and iOS) hides the
keyboard: the finger tips sit at the bottom with the layer to hold for the next key between them.
Enter is not a space: only Space moves to the next group of letters, Enter ends the lines (¶).

## Install on Arch Linux

`packaging/arch/PKGBUILD` builds `claudralingo-git` from the sources of GitHub,
`packaging/arch-bin/PKGBUILD` packages `claudralingo-bin` from the Linux build of a release
(nothing to compile, for the laptops). Both install a menu entry and its icon:

```sh
cd packaging/arch && makepkg -si        # or packaging/arch-bin
```

## Pens

The Wayland backend of Bevy 0.20 (winit 0.30) has no tablet support, and KDE sends it nothing
from a pen. With a pen or a stylus plugged in (`/proc/bus/input/devices`) and XWayland, the
window opens on X11, where the pen is a mouse. `CLAUDRALINGO_WAYLAND=1` keeps Wayland,
`CLAUDRALINGO_X11=1` forces X11, `CLAUDRALINGO_INPUT_LOG=1` prints the mouse, touch and picking
events.

## Keyboards

"QWERTY laptop (US)" has no navigation block nor arrows. The Corne 3x5+3 has the Typeractive
default keymap, the Corne 3x6+3 and the Lily58 the default
keymaps of ZMK (`corne.keymap`, `lily58.keymap`). The fingers and the rows come from the physical
layouts of ZMK. "My ZMK keymap": paste your `.keymap` file (Settings, Paste ZMK keymap) or drop
it on the window; a keymap of 36, 42 or 58 keys takes the matching physical layout, it is kept
with the progress.

## Stats

The Stats button: today's minutes against the daily goal (10 minutes) and the streak of days that
met it, the calendars of this month and the previous one (green: a lesson passed that day), the
words per minute and the accuracy of the last 30 days (point at a day for its value, the goal of
95% dashed), the exercises by type, and your best and most missed keys. The days are UTC days.

## Hanzi

The words of HSK 2.0 levels 1 to 4 (the lists of hsk.academy, 1193 words, the most frequent
first), in levels of ten words: HSK 1 is 15 levels, HSK 2 15, HSK 3 30 and HSK 4 61. One word at a
time, in five steps:

1. trace the strokes in order over the whole outline, the next stroke highlighted,
2. over a faded outline,
3. fill in the strokes taken out of the character,
4. on the empty grid, from memory,
5. type its pinyin, letters or the nine keys of a phone (T9, in the settings), and pick the word.

A stroke is compared with the strokes of the character as Hanzi Writer does (a port of its
matcher): its distance, start and end, direction, shape (Fréchet distance) and length. A stroke
drawn backwards or out of order is named; after three misses the stroke shows. A step with three
misses is done again. The words are spoken (Listen, or Space while drawing). The pinyin
candidates are the HSK words, then the words of the `inputx-pinyin` dictionary, with the
characters of the embedded font. The data is made by `scripts/prepare_hanzi.py`, see
`assets/hanzi/LICENSES.md`.

## Progress

Native builds save the progress to `~/.config/claudralingo/progress.json` (`$XDG_CONFIG_HOME` if
set), the browser to its `localStorage` (key `claudralingo.progress`, compact JSON). The progress
is saved at the end of every exercise and every 5 seconds after a change, a closed browser tab
loses at most those seconds; the last 5000 sessions are kept per layout. Per layout:

- `keys`: the hits, misses and touch times of every character,
- `best_keys` and `most_missed`: the rankings shown in the window,
- `basic_lesson`, the `finger` of the finger drill, the finished exercises in `sessions`,
- `shortcuts`: the hits, misses and times of every chord of the shortcuts, `alt_tab_best`,
- `klavaro`: Klavaro's own error and touch time accumulators (`accuracy.log` and
  `proficiency.log` in Klavaro), they generate the adaptability words.

The theme (dark by default), the layout, the exercise, `fade_legends` and `layer_trainer` are
saved too.

## Build

Every native build links with mold (`.cargo/config.toml`, clang and mold are needed). `cargo dev`
is `cargo run --features dynamic`: bevy as a shared library, for the local runs only. The release
binaries are built without it and copied to other machines, they do not depend on
`libbevy_dylib.so`.

```sh
cargo dev                                # native, bevy linked dynamically, fast rebuilds
cargo run --release                      # native
cargo test                               # the Klavaro port and the layouts
trunk serve --release                    # browser, http://127.0.0.1:8080
trunk build --release                    # browser, into dist/
```

`CLAUDRALINGO_SCREENSHOT=shot.png CLAUDRALINGO_TYPE="fj fk<" cargo dev` types the given keys
(`<` is backspace, a tab is Tab, `*` presses the expected key or chord, and fakes the Alt+Tab
round trip, `^` clicks the on-screen key the hints point at) and saves a screenshot, to check the
screen without a keyboard. `CLAUDRALINGO_PREVIEW_LAYER=2` shows a layer, `CLAUDRALINGO_HELP=1`
keeps the help of the first start open, `CLAUDRALINGO_LESSONS=1` and `CLAUDRALINGO_STATS=1` open
those panels, `CLAUDRALINGO_HANZI_STROKES=n` draws n strokes of the hanzi on their medians and
`CLAUDRALINGO_HANZI_INPUT=33` types the pinyin step. The keys of the real keyboard are ignored
during these runs.

The help (`?` in the toolbar or F1) sums up the modes, the buttons and the keys, it opens by
itself on the first start.

## Credits and license

- The exercises (`data/klavaro`) and the tutor logic come from Klavaro 3.14, GPL-3.0-or-later,
  Copyright (C) 2005-2021 Felipe Emmanuel Ferreira de Castro and the Klavaro authors
  (`data/klavaro/AUTHORS`). Claudralingo is therefore GPL-3.0-or-later too, see `LICENSE`.
- The look and the animations of the keyboard are inspired by
  [Layer Tutor](https://layer-tutor.pages.dev/), no code was copied.
- The widgets, the Fira fonts and the dark theme come from bevy_feathers.
- The hanzi data: HSK words (MIT), Make Me a Hanzi strokes (Arphic Public License), audio-cmn
  recordings (CC BY-SA), Noto Sans CJK SC (OFL), the stroke matching of Hanzi Writer (MIT), see
  `assets/hanzi/LICENSES.md`; the pinyin dictionary of `inputx-pinyin` (MIT or Apache-2.0).
