//! Versioned, bounded line protocol shared by Bash-facing MBX transports.
//!
//! Each message is one UTF-8 line. Fields are tab-separated and percent-escaped,
//! which keeps paths containing tabs/newlines unambiguous without a JSON parser in
//! the Bash integration. The protocol is deliberately tiny for the foundation
//! prototype; incompatible changes require a new magic/version prefix.

use std::fmt;

pub const MAGIC: &str = "MBX1";
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

pub const FLAG_NO_COLOR: u32 = 1 << 0;
pub const FLAG_ASCII_ICONS: u32 = 1 << 1;
pub const FLAG_NERD_ICONS: u32 = 1 << 2;
pub const FLAG_SSH: u32 = 1 << 3;
pub const FLAG_PRODUCTION: u32 = 1 << 4;
pub const FLAG_DISABLE_GIT: u32 = 1 << 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: u64,
    pub kind: RequestKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestKind {
    Ping,
    Prompt(PromptRequest),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptRequest {
    pub cwd: String,
    pub status: u8,
    pub duration_ms: Option<u64>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    pub id: u64,
    pub kind: ResponseKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponseKind {
    Pong,
    Prompt(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolError(String);

impl ProtocolError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ProtocolError {}

impl Request {
    pub fn encode(&self) -> String {
        match &self.kind {
            RequestKind::Ping => format!("{MAGIC}\t{}\tPING", self.id),
            RequestKind::Prompt(prompt) => format!(
                "{MAGIC}\t{}\tPROMPT\t{}\t{}\t{}\t{}",
                self.id,
                escape_field(&prompt.cwd),
                prompt.status,
                prompt
                    .duration_ms
                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                prompt.flags
            ),
        }
    }

    pub fn decode(line: &str) -> Result<Self, ProtocolError> {
        validate_line(line)?;
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 || fields[0] != MAGIC {
            return Err(ProtocolError::new(
                "unsupported or malformed protocol message",
            ));
        }

        let id = fields[1]
            .parse::<u64>()
            .map_err(|_| ProtocolError::new("invalid request id"))?;
        let kind = match fields[2] {
            "PING" if fields.len() == 3 => RequestKind::Ping,
            "PROMPT" if fields.len() == 7 => {
                let cwd = unescape_field(fields[3])?;
                let status = fields[4]
                    .parse::<u8>()
                    .map_err(|_| ProtocolError::new("invalid exit status"))?;
                let duration_ms = if fields[5] == "-" {
                    None
                } else {
                    Some(
                        fields[5]
                            .parse::<u64>()
                            .map_err(|_| ProtocolError::new("invalid duration"))?,
                    )
                };
                let flags = fields[6]
                    .parse::<u32>()
                    .map_err(|_| ProtocolError::new("invalid flags"))?;
                RequestKind::Prompt(PromptRequest {
                    cwd,
                    status,
                    duration_ms,
                    flags,
                })
            }
            _ => return Err(ProtocolError::new("unknown request or wrong field count")),
        };

        Ok(Self { id, kind })
    }
}

impl Response {
    pub fn encode(&self) -> String {
        match &self.kind {
            ResponseKind::Pong => format!("{MAGIC}\t{}\tPONG", self.id),
            ResponseKind::Prompt(prompt) => {
                format!("{MAGIC}\t{}\tPROMPT\t{}", self.id, escape_field(prompt))
            }
            ResponseKind::Error(message) => {
                format!("{MAGIC}\t{}\tERROR\t{}", self.id, escape_field(message))
            }
        }
    }

    pub fn decode(line: &str) -> Result<Self, ProtocolError> {
        validate_line(line)?;
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 || fields[0] != MAGIC {
            return Err(ProtocolError::new(
                "unsupported or malformed protocol response",
            ));
        }
        let id = fields[1]
            .parse::<u64>()
            .map_err(|_| ProtocolError::new("invalid response id"))?;
        let kind = match fields[2] {
            "PONG" if fields.len() == 3 => ResponseKind::Pong,
            "PROMPT" if fields.len() == 4 => ResponseKind::Prompt(unescape_field(fields[3])?),
            "ERROR" if fields.len() == 4 => ResponseKind::Error(unescape_field(fields[3])?),
            _ => return Err(ProtocolError::new("unknown response or wrong field count")),
        };
        Ok(Self { id, kind })
    }
}

pub fn escape_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '%' => escaped.push_str("%25"),
            character if character.is_control() => {
                let mut encoded = [0_u8; 4];
                for byte in character.encode_utf8(&mut encoded).as_bytes() {
                    escaped.push('%');
                    escaped.push(hex_digit(byte >> 4));
                    escaped.push(hex_digit(byte & 0x0f));
                }
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

pub fn unescape_field(value: &str) -> Result<String, ProtocolError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(ProtocolError::new("truncated percent escape"));
            }
            let high = from_hex(bytes[index + 1])?;
            let low = from_hex(bytes[index + 2])?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| ProtocolError::new("field is not valid UTF-8"))
}

fn validate_line(line: &str) -> Result<(), ProtocolError> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::new("protocol message exceeds 64 KiB"));
    }
    if line
        .chars()
        .any(|character| character.is_control() && character != '\t')
    {
        return Err(ProtocolError::new(
            "protocol message contains an unescaped control character",
        ));
    }
    Ok(())
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'A' + value - 10),
        _ => unreachable!(),
    }
}

fn from_hex(value: u8) -> Result<u8, ProtocolError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(ProtocolError::new("invalid percent escape")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trip_preserves_hostile_path_characters() {
        let request = Request {
            id: 42,
            kind: RequestKind::Prompt(PromptRequest {
                cwd: "/tmp/a\tb\n%/東京".to_owned(),
                status: 127,
                duration_ms: Some(2_345),
                flags: FLAG_NO_COLOR | FLAG_SSH,
            }),
        };
        assert_eq!(Request::decode(&request.encode()).unwrap(), request);
    }

    #[test]
    fn response_round_trip_preserves_prompt_escapes() {
        let response = Response {
            id: 7,
            kind: ResponseKind::Prompt("\\[\\e[31m\\]error%name\\n› ".to_owned()),
        };
        assert_eq!(Response::decode(&response.encode()).unwrap(), response);
    }

    #[test]
    fn malformed_messages_are_rejected() {
        assert!(Request::decode("MBX2\t1\tPING").is_err());
        assert!(Request::decode("MBX1\tbogus\tPING").is_err());
        assert!(Request::decode("MBX1\t1\tPROMPT\t/tmp\t0\t-\t0\textra").is_err());
        assert!(Request::decode("MBX1\t1\tPROMPT\tbad\u{1b}path\t0\t-\t0").is_err());
        assert!(unescape_field("bad%0Z").is_err());
    }
}
