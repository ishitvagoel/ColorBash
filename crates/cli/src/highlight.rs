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
    if KEYWORDS.iter().any(|keyword| *keyword == word) {
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
                        index += 2;
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
                        index += 2;
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
        let styled_end = output.len();
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
        let _ = styled_end;
    }
    if !mapped {
        styled_point = output.len();
    }
    (output, styled_point)
}

pub fn highlight_line(input: &str, plain_point: usize, color: bool) -> Option<(String, usize)> {
    if input.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }
    if input.contains('\0') {
        return None;
    }
    if !color {
        let point = plain_point.min(input.len());
        return Some((input.to_owned(), point));
    }
    Some(render(input, plain_point.min(input.len())))
}

#[allow(dead_code)]
pub fn strip_to_plain(value: &str) -> String {
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
            byte => {
                plain.push(byte as char);
                index += 1;
            }
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, highlight_line, lex, strip_to_plain};

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
    fn oversized_input_is_rejected() {
        let input = "a".repeat(4097);
        assert!(highlight_line(&input, 0, true).is_none());
    }
}
