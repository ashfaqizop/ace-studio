//! In-game text. Formats use a MiniMessage subset (<aqua>, <#ff8800>, <bold>, </tag>, <reset>) and legacy & codes.
//! Placeholder values are inserted as literal text, so a Discord user typing `<red>` can never style the line.
//! Rendered to a Pumpkin `TextComponent` with one child per styled run.

use pumpkin_plugin_api::common::{NamedColor, RgbColor};
use pumpkin_plugin_api::text::TextComponent;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Named(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Style {
    pub color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Seg {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Default, Debug)]
pub struct Rich(pub Vec<Seg>);

pub enum Value {
    Plain(String),
    Rich(Rich),
}

const NAMES: [&str; 16] = [
    "black", "dark_blue", "dark_green", "dark_aqua", "dark_red", "dark_purple", "gold", "gray", "dark_gray", "blue",
    "green", "aqua", "red", "light_purple", "yellow", "white",
];
const LEGACY: &[u8; 16] = b"0123456789abcdef";

fn named(i: u8) -> NamedColor {
    match i {
        0 => NamedColor::Black,
        1 => NamedColor::DarkBlue,
        2 => NamedColor::DarkGreen,
        3 => NamedColor::DarkAqua,
        4 => NamedColor::DarkRed,
        5 => NamedColor::DarkPurple,
        6 => NamedColor::Gold,
        7 => NamedColor::Gray,
        8 => NamedColor::DarkGray,
        9 => NamedColor::Blue,
        10 => NamedColor::Green,
        11 => NamedColor::Aqua,
        12 => NamedColor::Red,
        13 => NamedColor::LightPurple,
        14 => NamedColor::Yellow,
        _ => NamedColor::White,
    }
}

fn color_by_name(name: &str) -> Option<Color> {
    let n = name.to_ascii_lowercase().replace("grey", "gray");
    if let Some(hex) = n.strip_prefix('#') {
        return parse_hex(hex);
    }
    NAMES.iter().position(|c| *c == n).map(|i| Color::Named(i as u8))
}

pub fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

impl Rich {
    pub fn plain_text(text: impl Into<String>) -> Self {
        Rich(vec![Seg { text: text.into(), style: Style::default() }])
    }

    fn push(&mut self, text: &str, style: &Style) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.0.last_mut() {
            if last.style == *style {
                last.text.push_str(text);
                return;
            }
        }
        self.0.push(Seg { text: text.to_string(), style: style.clone() });
    }

    pub fn plain(&self) -> String {
        self.0.iter().map(|s| s.text.as_str()).collect()
    }

    pub fn to_component(&self) -> TextComponent {
        let mut root = TextComponent::text("");
        for seg in &self.0 {
            if seg.text.is_empty() {
                continue;
            }
            let mut c = TextComponent::text(&seg.text);
            c = match seg.style.color {
                Some(Color::Named(i)) => c.color_named(named(i)),
                Some(Color::Rgb(r, g, b)) => c.color_rgb(RgbColor { r, g, b }),
                None => c,
            };
            if seg.style.bold {
                c = c.bold(true);
            }
            if seg.style.italic {
                c = c.italic(true);
            }
            if seg.style.underlined {
                c = c.underlined(true);
            }
            if seg.style.strikethrough {
                c = c.strikethrough(true);
            }
            if seg.style.obfuscated {
                c = c.obfuscated(true);
            }
            if let Some(url) = &seg.style.link {
                c = c.click_open_url(url);
            }
            root = root.add_child(c);
        }
        root
    }
}

/// Applies a tag such as `aqua`, `#ff8800`, `color:red`, `bold`; returns None for unknown tags.
fn apply_tag(tag: &str, style: &Style) -> Option<Style> {
    let mut s = style.clone();
    let t = tag.to_ascii_lowercase();
    match t.as_str() {
        "bold" | "b" => s.bold = true,
        "italic" | "i" | "em" => s.italic = true,
        "underlined" | "u" => s.underlined = true,
        "strikethrough" | "st" => s.strikethrough = true,
        "obfuscated" | "obf" => s.obfuscated = true,
        _ => {
            let name = t.strip_prefix("color:").or_else(|| t.strip_prefix("c:")).unwrap_or(&t);
            s.color = Some(color_by_name(name)?);
        }
    }
    Some(s)
}

/// Applies legacy code `c` (after & or §) to the current style.
fn apply_legacy(c: u8, style: &mut Style) -> bool {
    let c = c.to_ascii_lowercase();
    if let Some(i) = LEGACY.iter().position(|x| *x == c) {
        *style = Style { color: Some(Color::Named(i as u8)), link: style.link.clone(), ..Style::default() };
        return true;
    }
    match c {
        b'l' => style.bold = true,
        b'o' => style.italic = true,
        b'n' => style.underlined = true,
        b'm' => style.strikethrough = true,
        b'k' => style.obfuscated = true,
        b'r' => *style = Style::default(),
        _ => return false,
    }
    true
}

/// Parses a trusted format string with tags, legacy codes and `%placeholders%`.
pub fn render(format: &str, vars: &[(&str, Value)]) -> Rich {
    parse(format, vars, true)
}

/// Parses untrusted text that may only use & colour codes (Discord members with a colour-code role).
pub fn legacy(text: &str) -> Rich {
    parse(text, &[], false)
}

fn parse(input: &str, vars: &[(&str, Value)], tags: bool) -> Rich {
    let mut out = Rich::default();
    let mut stack = vec![Style::default()];
    let mut buf = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    macro_rules! flush {
        () => {
            let top = stack.last().cloned().unwrap_or_default();
            out.push(&buf, &top);
            buf.clear();
        };
    }
    while i < bytes.len() {
        let b = bytes[i];
        if tags && b == b'<' {
            if let Some(end) = input[i + 1..].find('>') {
                let tag = &input[i + 1..i + 1 + end];
                if let Some(closing) = tag.strip_prefix('/') {
                    let _ = closing;
                    flush!();
                    if stack.len() > 1 {
                        stack.pop();
                    }
                    i += end + 2;
                    continue;
                }
                if tag.eq_ignore_ascii_case("reset") {
                    flush!();
                    stack = vec![Style::default()];
                    i += end + 2;
                    continue;
                }
                let top = stack.last().cloned().unwrap_or_default();
                if let Some(next) = apply_tag(tag, &top) {
                    flush!();
                    stack.push(next);
                    i += end + 2;
                    continue;
                }
            }
        } else if (b == b'&' || input[i..].starts_with('§')) && i + 1 < bytes.len() {
            let skip = if b == b'&' { 1 } else { '§'.len_utf8() };
            if let Some(&code) = bytes.get(i + skip) {
                // &#rrggbb
                if code == b'#' {
                    if let Some(color) = input.get(i + skip + 1..i + skip + 7).and_then(parse_hex) {
                        flush!();
                        let top = stack.last_mut().expect("style stack is never empty");
                        *top = Style { color: Some(color), ..Style::default() };
                        i += skip + 7;
                        continue;
                    }
                }
                let mut top = stack.last().cloned().unwrap_or_default();
                if apply_legacy(code, &mut top) {
                    flush!();
                    if let Some(last) = stack.last_mut() {
                        *last = top;
                    }
                    i += skip + 1;
                    continue;
                }
            }
        } else if b == b'%' && !vars.is_empty() {
            if let Some(end) = input[i + 1..].find('%') {
                let name = &input[i + 1..i + 1 + end];
                if let Some((_, value)) = vars.iter().find(|(k, _)| *k == name) {
                    flush!();
                    let top = stack.last().cloned().unwrap_or_default();
                    match value {
                        Value::Plain(text) => out.push(text, &top),
                        Value::Rich(rich) => {
                            for seg in &rich.0 {
                                let style = if seg.style == Style::default() { top.clone() } else { seg.style.clone() };
                                out.push(&seg.text, &style);
                            }
                        }
                    }
                    i += end + 2;
                    continue;
                }
            }
        }
        // copy one UTF-8 character
        let ch_len = input[i..].chars().next().map_or(1, char::len_utf8);
        buf.push_str(&input[i..i + ch_len]);
        i += ch_len;
    }
    flush!();
    out
}

/// Removes & and § codes and MiniMessage tags, for text going to Discord or the console.
pub fn strip(text: &str) -> String {
    let rich = parse(text, &[], true);
    rich.plain()
}

/// Turns a role colour (Discord's integer) into a `<#rrggbb>` tag.
pub fn role_color_tag(color: u32) -> String {
    if color == 0 { String::new() } else { format!("<#{color:06x}>") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_literal() {
        let r = render("<aqua>[D]</aqua> %name% » %message%", &[
            ("name", Value::Plain("Bob".into())),
            ("message", Value::Plain("<red>hi &a".into())),
        ]);
        assert_eq!(r.plain(), "[D] Bob » <red>hi &a");
        assert_eq!(r.0[0].style.color, Some(Color::Named(11)));
    }

    #[test]
    fn legacy_and_hex() {
        let r = legacy("&aGreen &#ff0000Red &rPlain");
        assert_eq!(r.plain(), "Green Red Plain");
        assert_eq!(r.0[1].style.color, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn strips() {
        assert_eq!(strip("&a<bold>Hi</bold> §cthere"), "Hi there");
    }
}
