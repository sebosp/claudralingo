# TODO

The ideas waiting for their turn. The research behind the learning rules (pacing, feedback,
the tutors, the penmanship pedagogy) is summed up at the end.

## Penmanship (Spencerian, Copperplate)

- **Pen pressure for the shades** (the flex of the nib): the ink thick on the downstrokes as the
  pen presses, thin on the hairlines; waiting for Bevy to read the pens on Wayland (the tablet
  protocol), XWayland only today. Then compare the shades with the profiles of the 1874 Theory:
  t type (heavy at the top), p type (heavy at the bottom, square stop), l type (loops), y type
  (even), oval type (heaviest in the middle of the curve); the second of a doubled letter half.
- **Capitals**, by class: the direct oval (O E D C), the reversed oval (X W Q Z V U Y I J), the
  capital stem (A N M T F K S L P B R H G); the principles 5, 6 and 7 as drills first.
- **Words and joins**: the letters joined on the connective slant (cam, ruin, some, dim, mew,
  max, mice), the spacing between the stems (one space) checked.
- **Flourishes** on the capitals, later.
- **More drills** from the copybooks and the IAMPETH lessons: the compressed ovals, the
  connected push-pulls, the loop rows, the "write 5, keep the best, move on at 3 of 5" loop
  with the best write saved.
- **The count**: a beat per stroke (1-2-3), spoken or clicked, the last stroke of a letter the
  "one" of the next, to write at an even tempo.
- **Checks beyond the matcher**: the height against the lines (1, 1¼, 2, 3 spaces), the loop
  crossing one space above the base, the stems parallel; the slant within about 5°.
- **The letter models**: compare the shapes with the plates of the 1874 Theory and the 1875
  Key (the scans have no engraved models); the x and the k are approximations.
- **Copperplate letters of its own**: the strokes of the Copperplate teachers (the underturn,
  the overturn, the compound curve, the stem loops) and their groups (i u w t l b j; n m h y p
  v; the ovals; the rest), the Spencerian skeletons on the 55° sheet for now.
- A printable guide sheet (PDF or SVG) with the x-height in millimeters (2.82 mm, 9 lines per
  inch, the "standard size" of the compendium) and the other presets.
- Stats of the penmanship: the slant and the first-try strokes per lesson over the days.

## Touch typing

- **Pacing of the new keys**: at most two new-key lessons a day (about 4 keys), a review lesson
  without new keys after every two, a new layer only once the base layer passed on two days.
- **Adaptive mode** after keybr: a confidence per key (an average of its times, α 0.1, against a
  target speed), one new key when all keys reached it and the first-try accuracy is 95%, every
  word with the weakest key; pseudo-words from the letters when fewer than 15 words match.
- **Retry a failed line** at most 3 times, then on with the focus on its weak key; a line of
  the missed words (up to 20).
- **Accuracy two ways**: the first try per character (now) and per keystroke, both in the stats.
- **Mastered keys**: their legend hidden on the on-screen keyboard (97% over their last three
  sessions), a blind mode without the keyboard.
- **Overspeed bursts** against a plateau: 30-60 s at +10-15% of the usual pace, 90% floor.
- **Sessions**: suggest 15-30 minutes, a soft word at 45, the streak of days rewarded rather
  than long sessions, the start hour of the day a setting.
- **After a long break**: a review before any new key, nothing reset.
- **Your text**: open a .txt file, keep a few texts, the books of Project Gutenberg as Typelit
  does.
- The word lines of the layer course too (the words with the symbols of the layer).
- Clicks of the keys as a sound option (off by default, the keyboard has its own); the error
  bell fading as the keys are mastered.

## Hanzi

- Sounds for a stroke right or missed (the setting "Sounds").

## Research

The notes and the report are kept out of git (`research_notes/`, `reports/`). The main points:

- Spaced practice wins: the post office typists learned the keyboard in 35 hours at one hour a
  day, 50 hours at two sessions of two hours, and preferred the long sessions (Baddeley &
  Longman 1978). About 30% of the speed was lost after nine months without practice.
- No study sets the number of new keys a day; the courses add 2 keys per lesson (the same finger
  of both hands), a review every 2-3 lessons, and pass at 95%.
- Feedback on every attempt helps now and hurts retention (the guidance hypothesis, Winstein &
  Schmidt 1990): the hints come after a pause, later on each row, and only for the keys not met
  yet in the exercise.
- The sounds: little evidence either way; keybr has them off by default.
- Penmanship: the 1874 Theory gives each small letter as a sequence of four principles at 52°
  (main) and 30° (connective), on a 2:1:2 sheet; the copybooks teach the principles, then i u w
  n m v x, o a e c, r s, t d p q, the loops, then the capitals by class; about two new letters
  a day.
