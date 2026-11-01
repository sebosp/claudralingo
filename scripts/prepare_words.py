#!/usr/bin/env python3
# Builds data/scowl/english.txt from the "final" folder of a SCOWL release:
#   python3 scripts/prepare_words.py scowl-2020.12.07/final data/scowl/english.txt
import re, sys
final, out = sys.argv[1], sys.argv[2]
block = set("""ass asses arse arses bitch bitches bastard bastards butt butts crap craps cunt cunts damn damned dick dicks dildo dildos dyke dykes fag fags faggot faggots fuck fucks fucked fucker fuckers fucking hell homo homos jap japs jizz kike kikes nazi nazis nigger niggers piss pissed porn porno prick pricks pussy pussies rape raped rapes rapist rapists retard retards semen sex sexy shit shits shitty slut sluts spic spics tit tits titties turd turds twat twats vagina vaginas wank wanker whore whores""".split())
seen, lines = set(), ["# The common English words of SCOWL 2020.12.07 (see Copyright): the levels 10, 20 and 35",
    "# of english-words and american-words with the contractions, lowercase, without the possessives.",
    "# A line '= <level>' starts the words of a level, the most common first."]
for level in (10, 20, 35):
    words = []
    for name in (f"english-words.{level}", f"american-words.{level}", f"english-contractions.{level}"):
        try:
            data = open(f"{final}/{name}", encoding="latin-1").read().split()
        except FileNotFoundError:
            continue
        for w in data:
            if re.fullmatch(r"[a-z][a-z']*[a-z]", w) and not w.endswith("'s") and w not in block and w not in seen:
                seen.add(w); words.append(w)
    lines.append(f"= {level}")
    lines += sorted(words)
open(out, "w").write("\n".join(lines) + "\n")
print(len(seen))
