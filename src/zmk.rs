//! A small reader of ZMK keymap bindings, enough for the keymaps of the supported keyboards:
//! `&kp KEY`, `AS(KEY)` (the auto shift hold-tap of the Typeractive keymaps), `&mo LAYER`,
//! `&trans` and `&bt`. The key codes are the US ones of `dt-bindings/zmk/keys.h`.
use crate::layout::{KeyPos, KeySpec, Output, Row, Special};

/// What a key does on a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// Types the key of the layer below, `&trans`.
    Trans,
    /// `&none`, or a behavior this reader does not know.
    None,
    Char(Output),
    Special(Special),
}

enum Code {
    Char(char),
    Special(Special),
}

fn code(name: &str) -> Option<Code> {
    use Special::*;
    if name.len() == 1 && name.chars().all(|c| c.is_ascii_uppercase()) {
        return name.chars().next().map(|c| Code::Char(c.to_ascii_lowercase()));
    }
    if let Some(digit) = name.strip_prefix('N').filter(|d| d.len() == 1 && d.chars().all(|c| c.is_ascii_digit())) {
        return digit.chars().next().map(Code::Char);
    }
    let c = match name {
        "SQT" | "SINGLE_QUOTE" | "APOS" => '\'',
        "DQT" | "DOUBLE_QUOTES" => '"',
        "DOT" | "PERIOD" => '.',
        "COMMA" => ',',
        "FSLH" | "SLASH" => '/',
        "BSLH" | "BACKSLASH" => '\\',
        "GRAVE" => '`',
        "TILDE" => '~',
        "MINUS" | "KP_MINUS" => '-',
        "UNDER" | "UNDERSCORE" => '_',
        "EQUAL" => '=',
        "PLUS" | "KP_PLUS" => '+',
        "LBKT" | "LEFT_BRACKET" => '[',
        "RBKT" | "RIGHT_BRACKET" => ']',
        "LBRC" | "LEFT_BRACE" => '{',
        "RBRC" | "RIGHT_BRACE" => '}',
        "LPAR" | "LEFT_PARENTHESIS" => '(',
        "RPAR" | "RIGHT_PARENTHESIS" => ')',
        "COLON" => ':',
        "SEMI" | "SEMICOLON" => ';',
        "EXCL" | "EXCLAMATION" => '!',
        "AT" | "AT_SIGN" => '@',
        "HASH" | "POUND" => '#',
        "DLLR" | "DOLLAR" => '$',
        "PRCNT" | "PERCENT" => '%',
        "CARET" => '^',
        "AMPS" | "AMPERSAND" => '&',
        "ASTRK" | "STAR" | "ASTERISK" | "KP_MULTIPLY" => '*',
        "PIPE" => '|',
        "QMARK" | "QUESTION" => '?',
        "LT" | "LESS_THAN" => '<',
        "GT" | "GREATER_THAN" => '>',
        _ => {
            return Some(Code::Special(match name {
                "SPACE" | "SPC" => Space,
                "RET" | "ENTER" | "RETURN" => Enter,
                "BSPC" | "BACKSPACE" => Backspace,
                "TAB" => Tab,
                "ESC" | "ESCAPE" => Escape,
                "DEL" | "DELETE" => Delete,
                "INS" | "INSERT" => Insert,
                "HOME" => Home,
                "END" => End,
                "PG_UP" | "PAGE_UP" => PageUp,
                "PG_DN" | "PAGE_DOWN" => PageDown,
                "LEFT" | "LEFT_ARROW" => Left,
                "RIGHT" | "RIGHT_ARROW" => Right,
                "UP" | "UP_ARROW" => Up,
                "DOWN" | "DOWN_ARROW" => Down,
                "LCTL" | "LCTRL" | "RCTL" | "RCTRL" | "LEFT_CONTROL" | "RIGHT_CONTROL" => Ctrl,
                "LALT" | "RALT" | "LEFT_ALT" | "RIGHT_ALT" => Alt,
                "LGUI" | "RGUI" | "LEFT_GUI" | "RIGHT_GUI" | "LCMD" | "RCMD" | "LWIN" | "RWIN" => Super,
                "LSHFT" | "LSHIFT" | "RSHFT" | "RSHIFT" | "LEFT_SHIFT" | "RIGHT_SHIFT" => Shift,
                "CAPS" | "CAPSLOCK" => Caps,
                _ => return None,
            }));
        }
    };
    Some(Code::Char(c))
}

/// The character typed with shift on a US layout.
pub fn us_shift(c: char) -> char {
    const PAIRS: [(char, char); 21] = [
        ('1', '!'), ('2', '@'), ('3', '#'), ('4', '$'), ('5', '%'), ('6', '^'), ('7', '&'),
        ('8', '*'), ('9', '('), ('0', ')'), ('-', '_'), ('=', '+'), ('[', '{'), (']', '}'),
        ('\\', '|'), (';', ':'), ('\'', '"'), (',', '<'), ('.', '>'), ('/', '?'), ('`', '~'),
    ];
    if c.is_ascii_lowercase() {
        return c.to_ascii_uppercase();
    }
    PAIRS.iter().find(|(lo, _)| *lo == c).map_or(c, |(_, up)| *up)
}

fn binding(code: Option<Code>, auto_shift: bool) -> Binding {
    match code {
        Some(Code::Char(c)) if auto_shift => Binding::Char(Output { lo: c, up: us_shift(c) }),
        Some(Code::Char(c)) => Binding::Char(Output { lo: c, up: c }),
        Some(Code::Special(s)) => Binding::Special(s),
        None => Binding::None,
    }
}

/// The bindings of a layer, in the order of the keys.
pub fn parse_bindings(layer: &str) -> Vec<Binding> {
    parse_bindings_with(layer, false)
}

/// With `kp_shifts`, a keyboard with Shift keys: `&kp N1` types `!` with Shift.
pub fn parse_bindings_with(layer: &str, kp_shifts: bool) -> Vec<Binding> {
    let mut tokens = layer.split_whitespace();
    let mut bindings = Vec::new();
    while let Some(token) = tokens.next() {
        let parsed = if let Some(inner) = token.strip_prefix("AS(").and_then(|t| t.strip_suffix(')')) {
            binding(code(inner), true)
        } else {
            match token {
                "&kp" => binding(tokens.next().and_then(code), kp_shifts),
                "&mo" => tokens
                    .next()
                    .and_then(|n| n.parse().ok())
                    .map_or(Binding::None, |n| Binding::Special(Special::Layer(n))),
                "&trans" => Binding::Trans,
                // The behaviors with a parameter, as the power of the lights.
                "&ext_power" | "&rgb_ug" | "&bl" | "&out" => {
                    tokens.next();
                    Binding::None
                }
                "&bt" => match tokens.next() {
                    Some("BT_SEL") => tokens
                        .next()
                        .and_then(|n| n.parse().ok())
                        .map_or(Binding::None, |n| Binding::Special(Special::Bluetooth(n))),
                    Some("BT_CLR") => Binding::Special(Special::BluetoothClear),
                    _ => Binding::None,
                },
                _ => Binding::None,
            }
        };
        bindings.push(parsed);
    }
    bindings
}

/// The key `index` of the layers, `&trans` types what the base layer types.
pub fn key_spec(layers: &[Vec<Binding>], index: usize, finger: u8, row: Row, left: bool, pos: KeyPos) -> KeySpec {
    let base = layers[0].get(index).copied().unwrap_or(Binding::None);
    let resolved: Vec<Binding> = layers
        .iter()
        .map(|layer| match layer.get(index).copied().unwrap_or(Binding::None) {
            Binding::Trans => base,
            b => b,
        })
        .collect();
    KeySpec {
        outputs: resolved.iter().map(|b| if let Binding::Char(out) = b { Some(*out) } else { None }).collect(),
        special: if let Binding::Special(s) = base { Some(s) } else { None },
        layer_specials: resolved
            .iter()
            .enumerate()
            .map(|(l, b)| match b {
                Binding::Special(s) if l > 0 => Some(*s),
                _ => None,
            })
            .collect(),
        hold: None,
        inherited: layers.iter().map(|layer| layer.get(index) == Some(&Binding::Trans)).collect(),
        finger,
        row,
        left,
        pos,
    }
}

/// The layers of a `.keymap` file of ZMK: the `bindings` of its layers in order, and their
/// names, `display-name` or the name of the node.
pub fn parse_keymap(text: &str) -> Vec<(String, Vec<Binding>)> {
    // Without the comments.
    let mut clean = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("/*") {
            rest = after.find("*/").map_or("", |end| &after[end + 2..]);
        } else if rest.starts_with("//") {
            rest = rest.find('\n').map_or("", |end| &rest[end..]);
        } else {
            let c = rest.chars().next().unwrap_or(' ');
            clean.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    let kp_shifts = !clean.contains("AS(");
    let mut layers = Vec::new();
    let mut search = 0;
    while let Some(found) = clean[search..].find("bindings") {
        let at = search + found;
        search = at + "bindings".len();
        // A property `bindings = <...>`, not the sensor bindings nor `dt-bindings`.
        let before = clean[..at].chars().next_back();
        let is_property = before.is_none_or(char::is_whitespace) && clean[search..].trim_start().starts_with('=');
        if !is_property {
            continue;
        }
        let Some(open) = clean[at..].find('<').map(|o| at + o + 1) else {
            break;
        };
        let Some(close) = clean[open..].find('>').map(|c| open + c) else {
            break;
        };
        // The name: the display-name of the node, else the node.
        let node_start = clean[..at].rfind('{').unwrap_or(0);
        let header = &clean[..node_start];
        let node = header.split_whitespace().last().unwrap_or("layer").to_string();
        let body = &clean[node_start..at];
        let name = body
            .find("display-name")
            .and_then(|d| body[d..].split('"').nth(1))
            .map(str::to_string)
            .unwrap_or(node);
        layers.push((name, parse_bindings_with(&clean[open..close], kp_shifts)));
        search = close;
    }
    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings() {
        let b = parse_bindings("AS(Q) &kp LCTL &mo 1 &trans &bt BT_SEL 2 &bt BT_CLR AS(N1) &kp KP_MULTIPLY");
        assert_eq!(
            b,
            vec![
                Binding::Char(Output { lo: 'q', up: 'Q' }),
                Binding::Special(Special::Ctrl),
                Binding::Special(Special::Layer(1)),
                Binding::Trans,
                Binding::Special(Special::Bluetooth(2)),
                Binding::Special(Special::BluetoothClear),
                Binding::Char(Output { lo: '1', up: '!' }),
                Binding::Char(Output { lo: '*', up: '*' }),
            ]
        );
    }
}
