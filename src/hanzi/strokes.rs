//! The kinds of the strokes (笔画): the nine basic strokes and the strokes made of them, with
//! how a brush writes them, for the brush guide and the breakdown of a word. The kind of every
//! stroke is in the data, see `scripts/prepare_hanzi.py --kinds`.

/// A basic stroke: the parts the names of the strokes are made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basic {
    Heng,
    Shu,
    Pie,
    Na,
    Dian,
    Ti,
    Gou,
    Zhe,
    Wan,
}

impl Basic {
    pub const ALL: [Basic; 9] =
        [Basic::Heng, Basic::Shu, Basic::Pie, Basic::Na, Basic::Dian, Basic::Ti, Basic::Gou, Basic::Zhe, Basic::Wan];

    pub fn hanzi(self) -> &'static str {
        match self {
            Basic::Heng => "横",
            Basic::Shu => "竖",
            Basic::Pie => "撇",
            Basic::Na => "捺",
            Basic::Dian => "点",
            Basic::Ti => "提",
            Basic::Gou => "钩",
            Basic::Zhe => "折",
            Basic::Wan => "弯",
        }
    }

    pub fn pinyin(self) -> &'static str {
        match self {
            Basic::Heng => "héng",
            Basic::Shu => "shù",
            Basic::Pie => "piě",
            Basic::Na => "nà",
            Basic::Dian => "diǎn",
            Basic::Ti => "tí",
            Basic::Gou => "gōu",
            Basic::Zhe => "zhé",
            Basic::Wan => "wān",
        }
    }

    pub fn english(self) -> &'static str {
        match self {
            Basic::Heng => "horizontal",
            Basic::Shu => "vertical",
            Basic::Pie => "left-falling",
            Basic::Na => "right-falling",
            Basic::Dian => "dot",
            Basic::Ti => "rising",
            Basic::Gou => "hook",
            Basic::Zhe => "turn",
            Basic::Wan => "bend",
        }
    }

    /// Its shape.
    pub fn shape(self) -> &'static str {
        match self {
            Basic::Heng => "Left to right, rising very slightly.",
            Basic::Shu => "Top to bottom, straight.",
            Basic::Pie => "From the top right down to the left, curving a little.",
            Basic::Na => "From the top left down to the right, ending in a wide flat foot.",
            Basic::Dian => "A short stroke from the top left down to the right (left dots fall to the left).",
            Basic::Ti => "Short, from the bottom left up to the right.",
            Basic::Gou => "A quick flick at the end of another stroke: of a 横, a 竖, a 弯.",
            Basic::Zhe => "A sharp corner between two directions, like the corner of a box.",
            Basic::Wan => "A smooth curve between two directions, no corner.",
        }
    }

    /// How a brush writes it: where it presses, thick, and where it lifts, thin.
    pub fn brush(self) -> &'static str {
        match self {
            Basic::Heng => {
                "The tip goes in against the direction, to the upper left, then presses down-right: a thick start. Glide \
                 right a little lighter, thinner in the middle; at the end press down-right again and turn the tip back \
                 into the stroke (回锋)."
            }
            Basic::Shu => {
                "The tip goes in to the upper left and presses: a thick top. Go straight down with even pressure. End with a \
                 pause and turn back up, a round end like a dewdrop (垂露), or lift while still moving, a sharp end like a \
                 hanging needle (悬针) when it is the last stroke of the character."
            }
            Basic::Pie => "Press at the top, thick, then sweep down to the left faster and faster while lifting: it thins to a sharp point.",
            Basic::Na => {
                "Start light and thin, press more and more on the way down, pause at the foot where it is widest, then turn \
                 flat to the right and lift out to a point."
            }
            Basic::Dian => "Touch lightly with the tip, thin, then press so the dot swells and turn the tip back into it: round and heavy at the end.",
            Basic::Ti => "Press at the start, thick, then flick up to the right, lifting quickly to a sharp point.",
            Basic::Gou => {
                "Slow down and press where the hook starts, the brush squats (蹲锋), then flick out in the direction of the \
                 hook and lift to a point."
            }
            Basic::Zhe => {
                "Lift a little at the end of the first part, then press at the corner (顿笔) to turn the brush: the corner \
                 has a small shoulder. Go on in the new direction."
            }
            Basic::Wan => "Keep the pressure even and turn the wrist smoothly through the curve, without a pause.",
        }
    }

    /// The basic stroke of a character of a name.
    fn of(c: char) -> Option<Basic> {
        Some(match c {
            '横' => Basic::Heng,
            '竖' => Basic::Shu,
            '撇' => Basic::Pie,
            '捺' => Basic::Na,
            '点' => Basic::Dian,
            '提' => Basic::Ti,
            '钩' => Basic::Gou,
            '折' => Basic::Zhe,
            // The slanted and the lying hooks are long curves.
            '弯' | '斜' | '卧' => Basic::Wan,
            _ => return None,
        })
    }
}

/// The kind of a stroke.
#[derive(Debug, PartialEq, Eq)]
pub struct Kind {
    /// Its letter in the data.
    pub code: char,
    pub hanzi: &'static str,
    pub pinyin: &'static str,
    pub english: &'static str,
    /// How a brush writes it.
    pub brush: &'static str,
}

impl Kind {
    /// The basic strokes of its name, in order: 横折钩 is 横, 折, 钩.
    pub fn parts(&self) -> Vec<Basic> {
        self.hanzi.chars().filter_map(Basic::of).collect()
    }

    /// The tip goes in against the direction at its start (藏锋), the strokes starting with a 横
    /// or a 竖, and the 提.
    pub fn hidden_entry(&self) -> bool {
        self.hanzi.starts_with(['横', '竖', '提'])
    }
}

const fn kind(code: char, hanzi: &'static str, pinyin: &'static str, english: &'static str, brush: &'static str) -> Kind {
    Kind { code, hanzi, pinyin, english, brush }
}

/// The kinds, their letters are cnchar's, the names it gives two have an uppercase letter for
/// the second.
pub static KINDS: [Kind; 33] = [
    kind('j', "横", "héng", "horizontal", "Tip in to the upper left and press, glide right a little lighter, press down at the end and turn back in."),
    kind('f', "竖", "shù", "vertical", "Tip in to the upper left and press, go straight down evenly, pause and turn back up (or thin out to a needle point when it ends the character)."),
    kind('s', "撇", "piě", "left-falling", "Press at the top, then sweep down-left, faster and lighter, to a sharp point."),
    kind('k', "点", "diǎn", "dot", "Touch lightly, press down-right so it swells, then turn back into the dot."),
    kind('d', "左点", "zuǒdiǎn", "left dot", "Touch lightly, press down-left so it swells, then turn back into the dot."),
    kind('l', "捺", "nà", "right-falling", "Start thin, press more and more down-right, pause at the widest foot, then lift out flat to the right."),
    kind('i', "提", "tí", "rising", "Press at the bottom left, then flick up-right, lifting fast to a point."),
    kind('c', "横折", "héngzhé", "horizontal turn", "A 横, lift a little and press at the corner, then go straight down, a little heavier."),
    kind('e', "横撇", "héngpiě", "horizontal left-falling", "A short 横, press at the corner, then sweep down-left to a point like a 撇."),
    kind('E', "横钩", "hénggōu", "horizontal hook", "A 横, press at its end, then flick down-left to a short point."),
    kind('r', "横折钩", "héngzhégōu", "horizontal turn hook", "A 横, press at the corner, go down, then squat on the brush and flick the hook to the left."),
    kind('g', "竖钩", "shùgōu", "vertical hook", "A 竖, slow down and press at the bottom, then flick the hook up-left."),
    kind('n', "撇折", "piězhé", "left-falling turn", "A short 撇, press at the corner, then rise up-right like a 提."),
    kind('u', "竖弯钩", "shùwāngōu", "vertical bend hook", "A 竖, bend smoothly into a long 横, press at its end, then flick the hook straight up."),
    kind('h', "竖提", "shùtí", "vertical rising", "A 竖, press at the bottom, then flick up-right like a 提."),
    kind('y', "斜钩", "xiégōu", "slanted hook", "Even pressure along a long slanted curve down-right, then press and flick the hook up."),
    kind('Y', "卧钩", "wògōu", "lying hook", "A shallow curve lying down-right, lighter in the middle, then press and flick the hook up-left."),
    kind('b', "竖弯", "shùwān", "vertical bend", "A 竖, bend smoothly to the right without a corner, end with a pause."),
    kind('B', "竖折", "shùzhé", "vertical turn", "A 竖, press at the corner, then go right, end with a pause."),
    kind('a', "横折折撇", "héngzhézhépiě", "horizontal turn turn left-falling", "A short 横, press, a short stroke down-left, press, a 横, press, then a long 撇 to a point."),
    kind('p', "横折提", "héngzhétí", "horizontal turn rising", "A 横, press at the corner, go down, press, then flick up-right."),
    kind('w', "横折折折钩", "héngzhézhézhégōu", "horizontal turn turn turn hook", "A 横, press, a short 撇, press, a 横, press, then go down and flick the hook left."),
    kind('W', "横撇弯钩", "héngpiěwāngōu", "horizontal left-falling bend hook", "A short 横, press, a short 撇, then a round bend down and flick the hook left: the ear 阝."),
    kind('z', "竖折折钩", "shùzhézhégōu", "vertical turn turn hook", "A 竖, press, a 横, press, then go down and flick the hook left."),
    kind('o', "横斜钩", "héngxiégōu", "horizontal slanted hook", "A 横, press at the corner, a long slanted curve down-right, then flick the hook up."),
    kind('O', "横折弯钩", "héngzhéwāngōu", "horizontal turn bend hook", "A 横, press at the corner, go down and bend smoothly to the right, then flick the hook up."),
    kind('m', "撇点", "piědiǎn", "left-falling dot", "A 撇 down-left, press at the bottom, then a long dot down-right."),
    kind('t', "弯钩", "wāngōu", "bend hook", "A curve bowing out to the right with even pressure, then press and flick the hook up-left."),
    kind('v', "横折折", "héngzhézhé", "horizontal turn turn", "A 横, press, go down, press, then a 横 to the right."),
    kind('V', "横折弯", "héngzhéwān", "horizontal turn bend", "A 横, press at the corner, go down and bend smoothly to the right, end with a pause."),
    kind('x', "竖折撇", "shùzhépiě", "vertical turn left-falling", "A short 竖, press, a 横, press, then a 撇 to a point."),
    kind('X', "竖折折", "shùzhézhé", "vertical turn turn", "A 竖, press, a 横, press, then go down."),
    kind('q', "横折折折", "héngzhézhézhé", "horizontal turn turn turn", "A 横, press, go down, press, a 横, press, then go down."),
];

/// The kind of a letter of the data.
pub fn kind_of(code: char) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stroke_has_a_kind() {
        let data = super::super::data::HanziData::load();
        for w in &data.words {
            for c in w.word.chars() {
                let glyph = data.glyph(c).expect("glyph");
                assert_eq!(glyph.kinds.len(), glyph.medians.len(), "{c}");
            }
        }
        let names = |c: char| data.glyph(c).unwrap().kinds.iter().map(|k| k.hanzi).collect::<Vec<_>>();
        assert_eq!(names('永'), ["点", "横折钩", "横撇", "撇", "捺"]);
        assert_eq!(names('你'), ["撇", "竖", "撇", "横钩", "竖钩", "撇", "点"]);
        assert_eq!(names('山'), ["竖", "竖折", "竖"]);
        assert_eq!(names('心'), ["左点", "卧钩", "点", "点"]);
        assert_eq!(names('九'), ["撇", "横折弯钩"]);
        assert_eq!(names('那')[4], "横撇弯钩");
    }

    #[test]
    fn the_parts_of_a_name() {
        assert_eq!(kind_of('r').unwrap().parts(), [Basic::Heng, Basic::Zhe, Basic::Gou]);
        assert_eq!(kind_of('u').unwrap().parts(), [Basic::Shu, Basic::Wan, Basic::Gou]);
        assert!(KINDS.iter().all(|k| !k.parts().is_empty()));
        assert!(kind_of('j').unwrap().hidden_entry());
        assert!(!kind_of('s').unwrap().hidden_entry());
    }
}
