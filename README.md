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

The four Klavaro exercises, with their original texts and goals:

| Exercise     | Text                                                     | Goal                               |
| ------------ | -------------------------------------------------------- | ---------------------------------- |
| Basic course | 43 lessons of random 5-letter "words" of a few keys      | 95% accuracy to unlock the next    |
| Adaptability | random weird words, made of your worst keys when needed  | 98% accuracy                       |
| Velocity     | random words of Klavaro's dictionary                     | 95% accuracy, 50 WPM               |
| Fluidness    | paragraphs of a text                                     | 97% accuracy, 50 WPM, 70% fluidity |

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
are the same for both layouts: on the Corne the digits and most symbols are on the layers. The
on-screen keyboard shows the next key in green, the shift to hold in purple, the layer key to hold
in amber (and the legends of that layer), and backspace in red while there are errors to erase.
The stripe at the bottom of each key is the finger that types it.

The Corne keymap is a placeholder until it matches the real one: QWERTY on the base layer,
`Shift Lower Space` on the left thumbs, `Enter Raise Backspace` on the right ones, digits and
shifted digits on Lower, brackets on Raise. See `corne36` in `src/layout.rs`.

## Progress

Native builds save the progress to `~/.config/claudralingo/progress.json` (`$XDG_CONFIG_HOME` if
set), the browser to its `localStorage`. Per layout:

- `keys`: the hits, misses and touch times of every character,
- `best_keys` and `most_missed`: the rankings shown in the window,
- `basic_lesson`, the finished exercises in `sessions`,
- `klavaro`: Klavaro's own error and touch time accumulators (`accuracy.log` and
  `proficiency.log` in Klavaro), they generate the adaptability words.

The theme (dark by default), the layout and the exercise are saved too.

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
(`<` is backspace) and saves a screenshot, to check the screen without a keyboard.

## Credits and license

- The exercises (`data/klavaro`) and the tutor logic come from Klavaro 3.14, GPL-3.0-or-later,
  Copyright (C) 2005-2021 Felipe Emmanuel Ferreira de Castro and the Klavaro authors
  (`data/klavaro/AUTHORS`). Claudralingo is therefore GPL-3.0-or-later too, see `LICENSE`.
- The look and the animations of the keyboard are inspired by
  [Layer Tutor](https://layer-tutor.pages.dev/), no code was copied.
- The widgets, the Fira fonts and the dark theme come from bevy_feathers.
