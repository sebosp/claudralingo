//! The exercise "Your text": a text pasted by the user, typed part by part. The typographic
//! quotes and dashes become the ones of the keyboard, the characters the keyboard does not type
//! are left out, the paragraphs are wrapped in the lines of the basic lessons.
use crate::klavaro::UPSYM;

/// The lines of a part, about one exercise of the basic course.
pub const PART_LINES: usize = 6;
const LINE_LENGTH: usize = 60;

/// The characters of the text as the keyboard types them.
fn plain(c: char) -> Option<&'static str> {
    Some(match c {
        '‘' | '’' | '‚' | '′' => "'",
        '“' | '”' | '„' | '″' | '«' | '»' => "\"",
        '–' | '—' | '−' | '‐' => "-",
        '…' => "...",
        '\t' | '\u{a0}' | '\u{2009}' | '\u{202f}' => " ",
        _ => return None,
    })
}

/// The lines of the text, ending with the paragraph symbol: `typeable` tells the characters
/// the keyboard has.
pub fn lines(raw: &str, typeable: impl Fn(char) -> bool) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in raw.split('\n') {
        let mut clean = String::new();
        for c in paragraph.chars() {
            match plain(c) {
                Some(s) => clean.push_str(s),
                None if c == ' ' || typeable(c) => clean.push(c),
                None => {}
            }
        }
        let words: Vec<&str> = clean.split_whitespace().collect();
        let mut line = String::new();
        for word in words {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > LINE_LENGTH {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

/// The number of parts of the text.
pub fn parts(lines: &[String]) -> usize {
    lines.len().div_ceil(PART_LINES).max(1)
}

/// The part to type, 0 based, in the format of the exercises.
pub fn part(lines: &[String], part: usize) -> String {
    let mut text = String::new();
    for line in lines.iter().skip(part * PART_LINES).take(PART_LINES) {
        text.push_str(line);
        text.push(UPSYM);
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pasted_text_in_lines_of_the_keyboard() {
        let raw = "“Don’t panic” — it said…\tKöln\n\n\nA second paragraph, a bit longer than one line of sixty characters.";
        let lines = lines(raw, |c| c.is_ascii_graphic());
        assert_eq!(lines[0], "\"Don't panic\" - it said... Kln");
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.chars().count() <= LINE_LENGTH));
        let text = part(&lines, 0);
        assert_eq!(text.matches(UPSYM).count(), 3);
        assert!(text.ends_with('\n'));
        assert_eq!(parts(&lines), 1);
        assert_eq!(parts(&[]), 1);
    }
}
