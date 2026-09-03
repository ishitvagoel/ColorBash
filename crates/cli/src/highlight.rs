use std::fmt::Write as _;

const RL_START: char = '\u{1}';
const RL_END: char = '\u{2}';
const MAX_HIGHLIGHT_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind {
    Comment,
    SingleQuoted,
    DoubleQuoted,
    Backtick,
    Variable,
    Operator,
    Keyword,
    Number,
    Word,
    Whitespace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
    "in", "function", "select", "time", "coproc", "return", "local", "export", "readonly",
    "declare", "typeset", "unset", "shift", "exit", "break", "continue", "trap", "source", ".",
    "true", "false",
];

fn keyword_kind(word: &str) -> TokenKind {
    if KEYWORDS.contains(&word) {
        TokenKind::Keyword
    } else {
        TokenKind::Word
    }
}

fn lex(input: &str) -> Vec<Token> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        let byte = bytes[index];
        match byte {
            b'#' => {
                index = bytes.len();
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    start,
                    end: index,
                });
                break;
            }
            b'\'' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'\'' {
                    index += 1;
                }
                if index < bytes.len() {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::SingleQuoted,
                    start,
                    end: index,
                });
            }
            b'"' => {
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'\\' && index + 1 < bytes.len() {
                        skip_escaped(input, &mut index);
                        continue;
                    }
                    if bytes[index] == b'"' {
                        index += 1;
                        break;
                    }
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::DoubleQuoted,
                    start,
                    end: index,
                });
            }
            b'`' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'`' {
                    if bytes[index] == b'\\' && index + 1 < bytes.len() {
                        skip_escaped(input, &mut index);
                        continue;
                    }
                    index += 1;
                }
                if index < bytes.len() {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Backtick,
                    start,
                    end: index,
                });
            }
            b'$' => {
                index += 1;
                if index < bytes.len() && bytes[index] == b'{' {
                    index += 1;
                    while index < bytes.len() && bytes[index] != b'}' {
                        index += 1;
                    }
                    if index < bytes.len() {
                        index += 1;
                    }
                } else if index < bytes.len() && (bytes[index] == b'(' || bytes[index] == b'[') {
                    let open = bytes[index];
                    let close = if open == b'(' { b')' } else { b']' };
                    index += 1;
                    while index < bytes.len() && bytes[index] != close {
                        index += 1;
                    }
                    if index < bytes.len() {
                        index += 1;
                    }
                } else {
                    while index < bytes.len() {
                        let ch = bytes[index];
                        if ch.is_ascii_alphanumeric() || ch == b'_' {
                            index += 1;
                        } else {
                            break;
                        }
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Variable,
                    start,
                    end: index,
                });
            }
            b'0'..=b'9' => {
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Number,
                    start,
                    end: index,
                });
            }
            b if b.is_ascii_whitespace() => {
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Whitespace,
                    start,
                    end: index,
                });
            }
            b'|' | b'&' | b';' | b'(' | b')' | b'{' | b'}' | b'[' | b']' | b'<' | b'>' | b'!'
            | b'=' | b'+' | b'-' | b'*' | b'/' | b'%' | b'?' | b':' | b',' | b'~' | b'^' => {
                index += 1;
                if index < bytes.len()
                    && matches!(
                        (byte, bytes[index]),
                        (b'|', b'|')
                            | (b'&', b'&')
                            | (b';', b';')
                            | (b'|', b'&')
                            | (b'<', b'<')
                            | (b'>', b'>')
                            | (b'!', b'=')
                            | (b'=', b'=')
                    )
                {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Operator,
                    start,
                    end: index,
                });
            }
            _ => {
                while index < bytes.len() {
                    let ch = bytes[index];
                    if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'.' {
                        index += 1;
                    } else {
                        break;
                    }
                }
                if index == start {
                    let step = input[index..].chars().next().map_or(1, |ch| ch.len_utf8());
                    index += step;
                }
                let word = &input[start..index];
                tokens.push(Token {
                    kind: keyword_kind(word),
                    start,
                    end: index,
                });
            }
        }
    }
    tokens
}

fn sgr_for(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Comment => "90",
        TokenKind::SingleQuoted | TokenKind::DoubleQuoted | TokenKind::Backtick => "32",
        TokenKind::Variable => "33",
        TokenKind::Operator => "35",
        TokenKind::Keyword => "1;34",
        TokenKind::Number => "36",
        TokenKind::Word => "0",
        TokenKind::Whitespace => "0",
    }
}

fn render(input: &str, plain_point: usize) -> (String, usize) {
    let tokens = lex(input);
    let mut output = String::new();
    let mut styled_point = plain_point.min(input.len());
    let mut mapped = false;

    for token in tokens {
        let text = &input[token.start..token.end];
        let styled_start = output.len();
        if token.kind == TokenKind::Whitespace || token.kind == TokenKind::Word {
            output.push_str(text);
        } else {
            let sgr = sgr_for(token.kind);
            let _ = write!(output, "{RL_START}\x1b[{sgr}m{RL_END}");
            output.push_str(text);
            let _ = write!(output, "{RL_START}\x1b[0m{RL_END}");
        }
        if !mapped && plain_point >= token.start && plain_point <= token.end {
            let within = plain_point - token.start;
            if token.kind == TokenKind::Whitespace || token.kind == TokenKind::Word {
                styled_point = styled_start + within;
            } else {
                let open_prefix = format!("{RL_START}\x1b[{}m{RL_END}", sgr_for(token.kind));
                styled_point = styled_start + open_prefix.len() + within;
            }
            mapped = true;
        } else if !mapped && plain_point < token.start {
            styled_point = styled_start;
            mapped = true;
        }
    }
    if !mapped {
        styled_point = output.len();
    }
    (output, styled_point)
}

fn skip_escaped(input: &str, index: &mut usize) {
    *index += 1;
    if *index >= input.len() {
        return;
    }
    let step = input[*index..].chars().next().map_or(1, |ch| ch.len_utf8());
    *index += step;
}

fn contains_c0_or_del(input: &str) -> bool {
    input.bytes().any(|byte| byte < 0x20 || byte == 0x7f)
}

/// Bash `READLINE_POINT` / `${#var}` are Unicode scalar counts (ADR 0015).
fn char_offset_to_byte(input: &str, chars: usize) -> usize {
    input
        .char_indices()
        .nth(chars)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

fn byte_offset_to_char(input: &str, byte: usize) -> usize {
    let mut byte = byte.min(input.len());
    while byte > 0 && !input.is_char_boundary(byte) {
        byte -= 1;
    }
    input[..byte].chars().count()
}

/// `plain_point` is a Unicode scalar count, matching Bash `READLINE_POINT`.
/// The returned styled point is also a scalar count into the styled string.
pub fn highlight_line(input: &str, plain_point: usize, color: bool) -> Option<(String, usize)> {
    if input.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }
    if contains_c0_or_del(input) {
        return None;
    }
    let char_point = plain_point.min(input.chars().count());
    if !color {
        return Some((input.to_owned(), char_point));
    }
    let byte_point = char_offset_to_byte(input, char_point);
    let (output, styled_byte) = render(input, byte_point);
    let styled_point = byte_offset_to_char(&output, styled_byte);
    Some((output, styled_point))
}

#[cfg(test)]
pub(crate) fn strip_to_plain(value: &str) -> String {
    let mut plain = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            0x01 => {
                index += 1;
                while index < bytes.len() && bytes[index] != 0x02 {
                    index += 1;
                }
                if index < bytes.len() {
                    index += 1;
                }
            }
            0x02 => {
                index += 1;
            }
            0x1b if index + 1 < bytes.len() && bytes[index + 1] == b'[' => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'm' {
                    index += 1;
                }
                if index < bytes.len() {
                    index += 1;
                }
            }
            _ => {
                let step = value[index..].chars().next().map_or(1, |ch| ch.len_utf8());
                plain.push_str(&value[index..index + step]);
                index += step;
            }
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, char_offset_to_byte, highlight_line, lex, strip_to_plain};

    const HOSTILE_HIGHLIGHT_CORPUS: &[&str] = &[
        "if echo \"$HOME\"; then true; fi # note",
        "cmd `whoami` $(id) ${HOME}",
        "printf '%s\\n' 'test$`\\'",
        "echo \"unclosed",
        "echo 'unclosed",
        "git commit -m \"'; rm -rf /\"",
        "100%_done",
        "ls /tmp/中文/café",
        "export PATH=/usr/bin:$PATH",
        "# comment only",
        "a=b c=d",
    ];

    #[test]
    fn hostile_corpus_strip_round_trips_with_color() {
        for input in HOSTILE_HIGHLIGHT_CORPUS {
            let (styled, _) = highlight_line(input, 0, true).expect("highlight");
            assert_eq!(
                strip_to_plain(&styled),
                *input,
                "strip must recover exact bytes for {input:?}"
            );
        }
    }

    #[test]
    fn hostile_corpus_cursor_maps_at_start_middle_and_end() {
        for input in HOSTILE_HIGHLIGHT_CORPUS {
            let char_len = input.chars().count();
            let mid = char_len / 2;
            for point in [0, mid, char_len] {
                let (styled, styled_point) = highlight_line(input, point, true).expect("highlight");
                assert_eq!(strip_to_plain(&styled), *input);
                let end = char_offset_to_byte(&styled, styled_point);
                let plain_prefix = strip_to_plain(&styled[..end]);
                assert_eq!(
                    plain_prefix.chars().count(),
                    point,
                    "cursor drift at point {point} for {input:?}"
                );
            }
        }
    }

    #[test]
    fn lexer_advances_past_utf8_bytes() {
        let tokens = lex("ls /tmp/中文/café");
        assert!(!tokens.is_empty());
        assert_eq!(
            tokens.last().map(|token| token.end),
            Some("ls /tmp/中文/café".len())
        );
    }

    #[test]
    fn lexer_classifies_incomplete_double_quote() {
        let tokens = lex("echo \"hi");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, TokenKind::Word);
        assert_eq!(tokens[2].kind, TokenKind::DoubleQuoted);
    }

    #[test]
    fn highlight_preserves_plain_bytes_when_stripped() {
        let input = "if echo \"$HOME\"; then true; fi # note";
        let (styled, _) = highlight_line(input, 0, true).unwrap();
        assert_eq!(strip_to_plain(&styled), input);
    }

    #[test]
    fn highlight_maps_plain_point_inside_word() {
        let input = "echo hi";
        let (_, point) = highlight_line(input, 5, true).unwrap();
        assert_eq!(point, 5);
    }

    #[test]
    fn highlight_maps_character_point_past_a_multibyte_scalar() {
        let input = "echo 中x";
        assert_eq!(input.len(), 9);
        assert_eq!(input.chars().count(), 7);
        let (styled, styled_point) = highlight_line(input, 6, true).expect("highlight");
        let end = char_offset_to_byte(&styled, styled_point);
        assert_eq!(strip_to_plain(&styled[..end]), "echo 中");
        let (styled, past_end) = highlight_line(input, 99, true).expect("highlight");
        let end = char_offset_to_byte(&styled, past_end);
        assert_eq!(strip_to_plain(&styled[..end]), input);
    }

    #[test]
    fn oversized_input_is_rejected() {
        let input = "a".repeat(4097);
        assert!(highlight_line(&input, 0, true).is_none());
    }

    #[test]
    fn c0_or_del_input_is_rejected() {
        assert!(highlight_line("\u{1}secret\u{2}", 0, true).is_none());
        assert!(highlight_line("echo \x1b[31mred", 0, true).is_none());
        assert!(highlight_line("tab\there", 0, false).is_none());
        assert!(highlight_line("ok", 0, true).is_some());
    }

    #[test]
    fn escaped_multibyte_in_quotes_does_not_split_a_scalar() {
        let tokens = lex("echo \"\\中x\"");
        assert!(
            tokens
                .iter()
                .all(|token| token.end <= "echo \"\\中x\"".len()),
            "token ends must stay on char boundaries"
        );
        let (styled, _) = highlight_line("echo \"\\中x\"", 0, true).expect("highlight");
        assert_eq!(strip_to_plain(&styled), "echo \"\\中x\"");
    }
}
