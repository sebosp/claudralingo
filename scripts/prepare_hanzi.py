#!/usr/bin/env python3
"""Prepares the data of the hanzi mode in assets/hanzi, embedded in the binary.

- words.json: the words of HSK 2.0 levels 1 to 4 (the lists of hsk.academy), from
  drkameleon/complete-hsk-vocabulary (MIT), the most frequent first in each level.
- strokes.json: the strokes of their characters, outlines and medians in stroke order, from
  Make Me a Hanzi (skishore/makemeahanzi, graphics.txt, Arphic Public License).
- audio.bin and audio.json: the recordings of the words, else of their syllables, from
  hugolpz/audio-cmn (CC-BY-SA), 24 kbit/s mp3, packed one after the other.

Run it from the repository root: python3 scripts/prepare_hanzi.py
"""
import concurrent.futures
import json
import os
import re
import sys
import unicodedata
import urllib.parse
import urllib.request

OUT = "assets/hanzi"
LEVELS = ["o1", "o2", "o3", "o4"]
HSK = "https://raw.githubusercontent.com/drkameleon/complete-hsk-vocabulary/main/complete.min.json"
GRAPHICS = "https://raw.githubusercontent.com/skishore/makemeahanzi/master/graphics.txt"
AUDIO = "https://raw.githubusercontent.com/hugolpz/audio-cmn/master/24k-abr/"
CACHE = os.environ.get("HANZI_CACHE", "/tmp/claudralingo-hanzi-cache")


def fetch(url, cache_name=None):
    path = os.path.join(CACHE, cache_name) if cache_name else None
    if path and os.path.exists(path):
        with open(path, "rb") as f:
            return f.read()
    try:
        with urllib.request.urlopen(url, timeout=60) as r:
            data = r.read()
    except urllib.error.HTTPError as err:
        if err.code == 404:
            data = b""
        else:
            raise
    if path:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
    return data


def numbered(syllable):
    """`hǎo` -> `hao3`, the names of the syllable recordings."""
    tone = 5
    base = ""
    for c in unicodedata.normalize("NFD", syllable.lower()):
        mark = {"̄": 1, "́": 2, "̌": 3, "̀": 4}.get(c)
        if mark:
            tone = mark
        elif c == "̈":
            # ü: `lü` is `lv`.
            base = base[:-1] + "v" if base.endswith("u") else base
        else:
            base += c
    base = re.sub(r"[^a-zv]", "", base)
    return f"{base}{1 if tone == 5 else tone}" if base else ""


def main():
    words = []
    seen = set()
    for entry in json.loads(fetch(HSK, "complete.json")):
        levels = [l for l in entry["l"] if l in LEVELS]
        if not levels or entry["s"] in seen:
            continue
        seen.add(entry["s"])
        form = entry["f"][0]
        words.append({
            "w": entry["s"],
            "p": form["i"]["y"],
            "m": "; ".join(form["m"][:3]),
            "l": LEVELS.index(min(levels)) + 1,
            "q": entry.get("q", 10**9),
        })
    words.sort(key=lambda w: (w["l"], w["q"]))
    for w in words:
        del w["q"]
    chars = sorted({c for w in words for c in w["w"]})
    print(f"{len(words)} words, {len(chars)} characters", file=sys.stderr)

    strokes = {}
    for line in fetch(GRAPHICS, "graphics.txt").decode().splitlines():
        g = json.loads(line)
        if g["character"] in chars:
            medians = [[[round(x), round(y)] for x, y in m] for m in g["medians"]]
            strokes[g["character"]] = {"s": g["strokes"], "m": medians}
    missing = [c for c in chars if c not in strokes]
    if missing:
        print(f"no strokes for {''.join(missing)}", file=sys.stderr)

    def audio(word):
        name = "hsk/cmn-" + word["w"] + ".mp3"
        data = fetch(AUDIO + urllib.parse.quote(name), "audio/" + name)
        if data:
            return word["w"], [data]
        # The syllables one after the other.
        parts = []
        for s in word["p"].replace("'", " ").split():
            n = numbered(s)
            if not n:
                continue
            sname = "syllabs/cmn-" + n + ".mp3"
            d = fetch(AUDIO + urllib.parse.quote(sname), "audio/" + sname)
            if d:
                parts.append(d)
        return word["w"], parts

    os.makedirs(OUT, exist_ok=True)
    index = {}
    blob = bytearray()
    with concurrent.futures.ThreadPoolExecutor(16) as pool:
        for w, parts in pool.map(audio, words):
            spans = []
            for data in parts:
                spans.append([len(blob), len(data)])
                blob += data
            if spans:
                index[w] = spans
    print(f"audio for {len(index)} words, {len(blob) // 1024} KiB", file=sys.stderr)

    with open(f"{OUT}/words.json", "w") as f:
        json.dump(words, f, ensure_ascii=False, separators=(",", ":"))
    with open(f"{OUT}/strokes.json", "w") as f:
        json.dump(strokes, f, ensure_ascii=False, separators=(",", ":"))
    with open(f"{OUT}/audio.json", "w") as f:
        json.dump(index, f, ensure_ascii=False, separators=(",", ":"))
    with open(f"{OUT}/audio.bin", "wb") as f:
        f.write(blob)


if __name__ == "__main__":
    main()


def subset_font():
    """The hanzi of the words in Noto Sans CJK SC (SIL Open Font License), only those: the
    whole font is 16 MB. Needs fonttools and the font, NOTO_CJK or the path of Arch Linux."""
    import subprocess

    ttc = os.environ.get("NOTO_CJK", "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc")
    words = json.load(open(f"{OUT}/words.json"))
    text = "".join(w["w"] for w in words) + "，。？！、：；“”（）·"
    with open(f"{CACHE}/subset.txt", "w") as f:
        f.write(text)
    subprocess.run([
        "pyftsubset", ttc, "--font-number=2", f"--text-file={CACHE}/subset.txt",
        "--unicodes=U+0020-007E", f"--output-file={OUT}/NotoSansSC-HSK.otf",
        "--layout-features=*", "--no-hinting",
    ], check=True)


if __name__ == "__main__" and "--font" in sys.argv:
    subset_font()
