//! MBX2 HIGHLIGHT/STYLED frame handling (ADR 0014).
//!
//! This is deliberately a sibling of `history_service`, not a variant folded
//! into it: highlighting has no privacy, storage, or persistence contract,
//! and `MBX_HIGHLIGHT=1` does not require `MBX_HISTORY=1`. Gating it behind
//! `HistoryHandler` would make highlighting depend on the history opt-in for
//! no reason. `transport::handle_mbx2_line` dispatches to this handler by
//! frame kind, independently of whether a `HistoryHandler` is present.

use crate::highlight::highlight_line;
use crate::history_service::{MBX2_MAGIC, encode_mbx2_error};
use mbx_protocol::{MAX_MESSAGE_BYTES, escape_field, unescape_field};

/// generation, color, point, text.
pub const HIGHLIGHT_FIELD_COUNT: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HighlightResponse {
    Styled {
        generation: u64,
        point: usize,
        line: String,
    },
    Error(String),
}

pub trait HighlightHandler: Send + Sync {
    fn handle(&self, request_id: u64, kind: &str, fields: &[String]) -> HighlightResponse;
}

#[derive(Default)]
pub struct HighlightService;

impl HighlightHandler for HighlightService {
    fn handle(&self, _request_id: u64, kind: &str, fields: &[String]) -> HighlightResponse {
        match kind {
            "HIGHLIGHT" => handle_highlight(fields),
            _ => HighlightResponse::Error("unsupported".to_owned()),
        }
    }
}

fn handle_highlight(fields: &[String]) -> HighlightResponse {
    if fields.len() != HIGHLIGHT_FIELD_COUNT {
        return HighlightResponse::Error("invalid".to_owned());
    }
    let Some(generation) = unescape_field(&fields[0])
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return HighlightResponse::Error("invalid".to_owned());
    };
    let Some(color) = unescape_field(&fields[1])
        .ok()
        .and_then(|value| match value.as_str() {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        })
    else {
        return HighlightResponse::Error("invalid".to_owned());
    };
    let Some(point) = unescape_field(&fields[2])
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return HighlightResponse::Error("invalid".to_owned());
    };
    let text = match unescape_field(&fields[3]) {
        Ok(value) => value,
        Err(_) => return HighlightResponse::Error("invalid".to_owned()),
    };
    match highlight_line(&text, point, color) {
        Some((line, styled_point)) => HighlightResponse::Styled {
            generation,
            point: styled_point,
            line,
        },
        None => HighlightResponse::Error("invalid".to_owned()),
    }
}

/// Encodes a STYLED frame. `highlight_line` bounds input to a few KiB, so the
/// percent-escaped, styled result cannot approach the 64 KiB frame limit in
/// practice; the check still fails closed rather than assume that headroom.
pub fn encode_mbx2_styled(request_id: u64, generation: u64, point: usize, line: &str) -> String {
    let encoded = format!(
        "{MBX2_MAGIC}\t{request_id}\tSTYLED\t{generation}\t{point}\t{}",
        escape_field(line)
    );
    if encoded.len() > MAX_MESSAGE_BYTES {
        return encode_mbx2_error(request_id, "invalid");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| escape_field(value)).collect()
    }

    #[test]
    fn highlight_request_rejects_wrong_field_count() {
        let response = handle_highlight(&f(&["1", "1"]));
        assert_eq!(response, HighlightResponse::Error("invalid".to_owned()));
    }

    #[test]
    fn highlight_request_rejects_non_binary_color() {
        let response = handle_highlight(&f(&["1", "2", "0", "echo hi"]));
        assert_eq!(response, HighlightResponse::Error("invalid".to_owned()));
    }

    #[test]
    fn highlight_request_styles_plain_text_when_color_enabled() {
        let response = handle_highlight(&f(&["7", "1", "0", "if true; then :; fi"]));
        match response {
            HighlightResponse::Styled {
                generation, line, ..
            } => {
                assert_eq!(generation, 7);
                assert_ne!(line, "if true; then :; fi", "color=1 must style keywords");
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn highlight_request_returns_plain_bytes_when_color_disabled() {
        let response = handle_highlight(&f(&["1", "0", "0", "if true; then :; fi"]));
        assert_eq!(
            response,
            HighlightResponse::Styled {
                generation: 1,
                point: 0,
                line: "if true; then :; fi".to_owned(),
            }
        );
    }

    #[test]
    fn highlight_request_rejects_input_over_the_byte_cap() {
        let oversized = "a".repeat(5 * 1024);
        let response = handle_highlight(&f(&["1", "0", "0", &oversized]));
        assert_eq!(response, HighlightResponse::Error("invalid".to_owned()));
    }

    #[test]
    fn styled_frame_round_trips_generation_point_and_line() {
        let encoded = encode_mbx2_styled(9, 3, 2, "ab");
        assert_eq!(encoded, "MBX2\t9\tSTYLED\t3\t2\tab");
    }

    #[test]
    fn unsupported_kind_is_rejected() {
        let service = HighlightService;
        let response = service.handle(1, "QUERY", &[]);
        assert_eq!(response, HighlightResponse::Error("unsupported".to_owned()));
    }
}
