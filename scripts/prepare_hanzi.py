#!/usr/bin/env python3
"""Prepares the data of the hanzi mode in assets/hanzi, embedded in the binary.

- words.json: the words of HSK 2.0 levels 1 to 4 (the lists of hsk.academy), from
  drkameleon/complete-hsk-vocabulary (MIT), the most frequent first in each level.
- strokes.json: the strokes of their characters, outlines and medians in stroke order, from
  Make Me a Hanzi (skishore/makemeahanzi, graphics.txt, Arphic Public License).
- audio.bin and audio.json: the recordings of the words, else of their syllables, from
  hugolpz/audio-cmn (CC-BY-SA), 24 kbit/s mp3, packed one after the other.
- the kinds of the strokes in strokes.json ("k", a letter per stroke, see
  src/hanzi/strokes.rs), from cnchar-order (MIT), the names it leaves ambiguous resolved by the
  shapes of the medians: `--kinds`, on the strokes.json there is.

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


if __name__ == "__main__" and len(sys.argv) == 1:
    main()


def subset_font():
    """The hanzi of the words in Noto Sans CJK SC (SIL Open Font License), only those and the
    ones of the texts of src/hanzi (the names of the strokes): the whole font is 16 MB. Needs
    fonttools and the font, NOTO_CJK or the path of Arch Linux."""
    import glob
    import subprocess

    ttc = os.environ.get("NOTO_CJK", "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc")
    words = json.load(open(f"{OUT}/words.json"))
    text = "".join(w["w"] for w in words) + "，。？！、：；“”（）·"
    for path in sorted(glob.glob("src/hanzi/*.rs")):
        text += "".join(sorted({c for c in open(path).read() if "\u3400" <= c <= "\u9fff"}))
    with open(f"{CACHE}/subset.txt", "w") as f:
        f.write(text)
    subprocess.run([
        "pyftsubset", ttc, "--font-number=2", f"--text-file={CACHE}/subset.txt",
        "--unicodes=U+0020-007E", f"--output-file={OUT}/NotoSansSC-HSK.otf",
        "--layout-features=*", "--no-hinting",
    ], check=True)


if __name__ == "__main__" and "--font" in sys.argv:
    subset_font()


CNCHAR = "https://registry.npmjs.org/cnchar-order/-/cnchar-order-3.2.6.tgz"
# The 横斜钩 of 风, 飞 and 气, the other 横折弯钩 of cnchar's "o".
HENG_XIE_GOU = set("风飞气汽疯讽枫")


def angles(median):
    """The directions of the segments of a median, in degrees, y upwards."""
    import math
    return [math.degrees(math.atan2(y1 - y0, x1 - x0)) for (x0, y0), (x1, y1) in zip(median, median[1:])]


def lengths(median):
    import math
    return [math.hypot(x1 - x0, y1 - y0) for (x0, y0), (x1, y1) in zip(median, median[1:])]


def resolve(code, char, i, codes, median):
    """One letter for the names cnchar gives two, by the shape of the median. Checked by eye on
    every stroke of HSK 1 to 4."""
    xs = [p[0] for p in median]
    ys = [p[1] for p in median]
    wide = (max(xs) - min(xs)) / max(1, max(ys) - min(ys))
    # Without the first segment: the median enters the stroke from its tip, at an angle.
    turns = [abs(b - a) for a, b in zip(angles(median)[1:], angles(median)[2:])]
    if code == "e":
        # 横撇 or 横钩: the 横钩 of 欠, else a short tail after the horizontal.
        if codes[i - 1:i] == "s" and codes[i + 1:i + 3] == "sl":
            return "E"
        across = tail = 0.0
        turned = False
        for a, l in zip(angles(median), lengths(median)):
            if not turned and -35 <= a <= 45:
                across += l
            elif a < -60 or turned:
                turned = True
                tail += l
        ratio = tail / max(across, 1)
        # The 横撇 of マ, 甬 and 令 are short, a dot follows them.
        if codes[i + 1:i + 2] == "k" and ratio >= 0.35:
            return "e"
        return "E" if ratio < 0.48 else "e"
    if code == "y":
        # 斜钩 steep, 卧钩 (心) lying.
        return "y" if wide < 1.2 else "Y"
    if code == "w":
        # 横撇弯钩 (阝) narrow, 横折折折钩 (乃) wide.
        return "W" if wide < 0.56 else "w"
    if code == "v":
        # 横折弯 (几 of 殳 and 舟) bends, 横折折 turns.
        return "V" if sum(1 for a in angles(median)[1:] if -80 < a < -20) >= 2 else "v"
    if code == "x":
        return "x" if min(angles(median)[-3:]) < -100 else "X"
    if code == "o":
        return "o" if char in HENG_XIE_GOU else "O"
    if code == "b":
        # 竖折 (山) turns sharply, 竖弯 (四) bends.
        return "B" if turns and max(turns) >= 55 else "b"
    return code


def stroke_kinds():
    """The kinds of the strokes into strokes.json: cnchar's letters, one letter per name."""
    import io
    import tarfile

    tgz = fetch(CNCHAR, "cnchar-order.tgz")
    with tarfile.open(fileobj=io.BytesIO(tgz)) as tar:
        js = tar.extractfile("package/cnchar.order.min.js").read().decode()
    orders = json.loads(re.findall(r"JSON\.parse\('(\{.*?\})'\)", js)[0])
    path = f"{OUT}/strokes.json"
    strokes = json.load(open(path))
    for char, glyph in strokes.items():
        codes = orders.get(char)
        if codes is None or len(codes) != len(glyph["m"]):
            sys.exit(f"cnchar-order: no strokes or not {len(glyph['m'])} for {char}")
        glyph["k"] = "".join(resolve(c, char, i, codes, m) for i, (c, m) in enumerate(zip(codes, glyph["m"])))
    with open(path, "w") as f:
        json.dump(strokes, f, ensure_ascii=False, separators=(",", ":"))


if __name__ == "__main__" and "--kinds" in sys.argv:
    stroke_kinds()
