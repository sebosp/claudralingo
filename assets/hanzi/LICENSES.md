# The data of the hanzi mode

Prepared by `scripts/prepare_hanzi.py`, embedded in the binary.

- `words.json`: the words of HSK 2.0 levels 1 to 4 with their pinyin and meanings, from
  [complete-hsk-vocabulary](https://github.com/drkameleon/complete-hsk-vocabulary) by Yanis
  Zafirópulos, MIT License.
- `strokes.json`: the stroke outlines and medians of the characters, from `graphics.txt` of
  [Make Me a Hanzi](https://github.com/skishore/makemeahanzi) by Shaunak Kishore, derived from
  the fonts Arphic PL KaitiM GB and Arphic PL UKai. Redistributed under the Arphic Public
  License, see `ARPHICPL.TXT`.
- `audio.bin` and `audio.json`: the recordings of the words (Yue Tan) and of the syllables (Chen
  Wang) of [audio-cmn](https://github.com/hugolpz/audio-cmn) by Hugo Lopez, PLIDAM, INALCO,
  licensed [CC BY-SA](https://creativecommons.org/licenses/by-sa/4.0/); the 24 kbit/s mp3
  files packed one after the other, unchanged. The packed audio is under the same license.
- The stroke matching of `src/hanzi/matcher.rs` is a port of the stroke matching of
  [Hanzi Writer](https://github.com/chanind/hanzi-writer) by David Chanin, MIT License.
- `NotoSansSC-HSK.otf`: the characters of the words in Noto Sans CJK SC, by Adobe and Google,
  SIL Open Font License 1.1, see `OFL.txt`; subset by `scripts/prepare_hanzi.py --font`.
