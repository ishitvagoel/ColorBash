use crate::history::{HistoryEntry, HistoryPolicy, HistoryRecorder};
use mbx_protocol::{MAX_MESSAGE_BYTES, unescape_field};

pub const MBX2_MAGIC: &str = "MBX2";
pub const RECORD_FIELD_COUNT: usize = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryResponse {
    Pong,
    Ack,
    Error(String),
}

pub trait HistoryHandler: Send + Sync {
    fn handle(&self, request_id: u64, kind: &str, fields: &[String]) -> HistoryResponse;
}

pub struct HistoryService {
    recorder: Box<dyn HistoryRecorder>,
    policy: Box<dyn HistoryPolicy>,
}

impl HistoryService {
    pub fn new(recorder: Box<dyn HistoryRecorder>, policy: Box<dyn HistoryPolicy>) -> Self {
        Self { recorder, policy }
    }
}

impl HistoryHandler for HistoryService {
    fn handle(&self, _request_id: u64, kind: &str, fields: &[String]) -> HistoryResponse {
        match kind {
            "PING" if fields.is_empty() => HistoryResponse::Pong,
            "RECORD" => handle_record(self.recorder.as_ref(), self.policy.as_ref(), fields),
            "PING" => HistoryResponse::Error("invalid field count".to_owned()),
            _ => HistoryResponse::Error(format!("unknown MBX2 kind: {kind}")),
        }
    }
}

fn handle_record(
    recorder: &dyn HistoryRecorder,
    policy: &dyn HistoryPolicy,
    fields: &[String],
) -> HistoryResponse {
    if fields.len() != RECORD_FIELD_COUNT {
        return HistoryResponse::Error("invalid field count".to_owned());
    }
    let entry = match decode_record(fields) {
        Ok(entry) => entry,
        Err(error) => return HistoryResponse::Error(error),
    };
    if !policy.allows(&entry) {
        return HistoryResponse::Ack;
    }
    match recorder.record(entry) {
        Ok(()) => HistoryResponse::Ack,
        Err(error) => HistoryResponse::Error(error.kind().as_str().to_owned()),
    }
}

fn decode_record(fields: &[String]) -> Result<HistoryEntry, String> {
    let mut decoded = Vec::with_capacity(fields.len());
    for field in fields {
        decoded.push(unescape_field(field).map_err(|error| error.to_string())?);
    }
    let [
        session_id,
        sequence,
        history_number,
        command_text,
        cwd,
        completed_at,
        status,
        duration_ms,
        host,
        user,
    ] = <[String; RECORD_FIELD_COUNT]>::try_from(decoded)
        .map_err(|_| "invalid record field count".to_owned())?;
    let sequence = sequence
        .parse::<u64>()
        .map_err(|_| "invalid event sequence".to_owned())?;
    let history_number = if history_number == "-" {
        None
    } else {
        Some(
            history_number
                .parse::<i64>()
                .map_err(|_| "invalid history number".to_owned())?,
        )
    };
    let status = status
        .parse::<i32>()
        .map_err(|_| "invalid status".to_owned())?;
    let duration_ms = if duration_ms == "-" {
        None
    } else {
        Some(
            duration_ms
                .parse::<u64>()
                .map_err(|_| "invalid duration".to_owned())?,
        )
    };
    if command_text.len() > MAX_MESSAGE_BYTES {
        return Err("oversized command text".to_owned());
    }
    Ok(HistoryEntry {
        session_id,
        event_sequence: sequence,
        history_number,
        command_text,
        start_cwd: cwd,
        completed_at,
        status,
        duration_ms,
        host,
        user,
        repo_root: None,
        repo_branch: None,
    })
}

pub fn encode_mbx2(request_id: u64, kind: &str) -> String {
    format!("{MBX2_MAGIC}\t{request_id}\t{kind}")
}

pub fn encode_mbx2_error(request_id: u64, kind: &str) -> String {
    format!("{MBX2_MAGIC}\t{request_id}\tERROR\t{kind}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryEntry, HistoryError};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingRecorder {
        entries: Mutex<Vec<HistoryEntry>>,
    }

    impl RecordingRecorder {
        fn shared() -> (Arc<Self>, Arc<Self>) {
            let recorder = Arc::new(Self::default());
            (recorder.clone(), recorder)
        }
    }

    impl HistoryRecorder for RecordingRecorder {
        fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError> {
            self.entries.lock().unwrap().push(entry);
            Ok(())
        }
    }

    impl HistoryRecorder for Arc<RecordingRecorder> {
        fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError> {
            self.entries.lock().unwrap().push(entry);
            Ok(())
        }
    }

    struct AllowAll;

    impl HistoryPolicy for AllowAll {
        fn disabled(&self) -> bool {
            false
        }

        fn allows(&self, _entry: &HistoryEntry) -> bool {
            true
        }
    }

    struct DenyAll;

    impl HistoryPolicy for DenyAll {
        fn disabled(&self) -> bool {
            false
        }

        fn allows(&self, _entry: &HistoryEntry) -> bool {
            false
        }
    }

    fn record_fields() -> Vec<String> {
        vec![
            "session-1".to_owned(),
            "7".to_owned(),
            "9".to_owned(),
            "echo hello".to_owned(),
            "/work".to_owned(),
            "2026-08-15T10:00:00Z".to_owned(),
            "0".to_owned(),
            "2500".to_owned(),
            "host".to_owned(),
            "user".to_owned(),
        ]
    }

    #[test]
    fn ping_returns_pong() {
        let (recorder, _) = RecordingRecorder::shared();
        let service = HistoryService::new(Box::new(recorder), Box::new(AllowAll));
        assert_eq!(service.handle(1, "PING", &[]), HistoryResponse::Pong);
    }

    #[test]
    fn record_decodes_and_enqueues() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(Box::new(recorder), Box::new(AllowAll));
        let response = service.handle(2, "RECORD", &record_fields());
        assert_eq!(response, HistoryResponse::Ack);
        let entries = probe.entries.lock().unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.session_id, "session-1");
        assert_eq!(entry.event_sequence, 7);
        assert_eq!(entry.command_text, "echo hello");
        assert_eq!(entry.duration_ms, Some(2500));
    }

    #[test]
    fn policy_exclusion_acknowledges_without_recording() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(Box::new(recorder), Box::new(DenyAll));
        let response = service.handle(3, "RECORD", &record_fields());
        assert_eq!(response, HistoryResponse::Ack);
        assert!(probe.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn malformed_record_returns_typed_error() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(Box::new(recorder), Box::new(AllowAll));
        let mut fields = record_fields();
        fields[1] = "not-a-number".to_owned();
        let response = service.handle(4, "RECORD", &fields);
        assert!(matches!(response, HistoryResponse::Error(_)));
        assert!(probe.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn percent_escaped_fields_round_trip() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(Box::new(recorder), Box::new(AllowAll));
        let mut fields = record_fields();
        fields[3] = "echo%20a%09tab".to_owned();
        let response = service.handle(5, "RECORD", &fields);
        assert_eq!(response, HistoryResponse::Ack);
        assert_eq!(probe.entries.lock().unwrap()[0].command_text, "echo a\ttab");
    }

    #[test]
    fn hostile_sql_command_text_is_recorded_as_data() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(Box::new(recorder), Box::new(AllowAll));
        let mut fields = record_fields();
        fields[3] = "'; DROP TABLE history;--".to_owned();
        let response = service.handle(6, "RECORD", &fields);
        assert_eq!(response, HistoryResponse::Ack);
        assert_eq!(
            probe.entries.lock().unwrap()[0].command_text,
            "'; DROP TABLE history;--"
        );
    }

    #[test]
    fn substitute_error_is_constructible() {
        use crate::history::HistoryErrorKind;
        let error = HistoryError::new(HistoryErrorKind::StorageFailure, "substitute failure");
        assert_eq!(error.kind().as_str(), "storage_failure");
        assert_eq!(
            error.to_string(),
            "history storage_failure: substitute failure"
        );
    }
}
