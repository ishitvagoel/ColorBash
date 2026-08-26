use crate::history::{
    HistoryEntry, HistoryPolicy, HistoryRecorder, HistorySearch, MAX_QUERY_LIMIT,
};
use mbx_protocol::{MAX_MESSAGE_BYTES, escape_field, unescape_field};

pub const MBX2_MAGIC: &str = "MBX2";
pub const RECORD_FIELD_COUNT: usize = 10;
pub const QUERY_FIELD_COUNT: usize = 4;
pub const CANCEL_FIELD_COUNT: usize = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryResponse {
    Pong,
    Ack,
    Result {
        generation: u64,
        commands: Vec<String>,
    },
    Error(String),
}

pub trait HistoryHandler: Send + Sync {
    fn handle(&self, request_id: u64, kind: &str, fields: &[String]) -> HistoryResponse;
}

pub struct HistoryService {
    recorder: Box<dyn HistoryRecorder>,
    search: Box<dyn HistorySearch>,
    policy: Box<dyn HistoryPolicy>,
}

impl HistoryService {
    pub fn new(
        recorder: Box<dyn HistoryRecorder>,
        search: Box<dyn HistorySearch>,
        policy: Box<dyn HistoryPolicy>,
    ) -> Self {
        Self {
            recorder,
            search,
            policy,
        }
    }
}

impl HistoryHandler for HistoryService {
    fn handle(&self, _request_id: u64, kind: &str, fields: &[String]) -> HistoryResponse {
        match kind {
            "PING" if fields.is_empty() => HistoryResponse::Pong,
            "RECORD" => handle_record(self.recorder.as_ref(), self.policy.as_ref(), fields),
            "QUERY" => handle_query(self.search.as_ref(), fields),
            "CANCEL" => handle_cancel(fields),
            "PING" => HistoryResponse::Error("invalid".to_owned()),
            _ => HistoryResponse::Error("unsupported".to_owned()),
        }
    }
}

fn handle_record(
    recorder: &dyn HistoryRecorder,
    policy: &dyn HistoryPolicy,
    fields: &[String],
) -> HistoryResponse {
    if fields.len() != RECORD_FIELD_COUNT {
        return HistoryResponse::Error("invalid".to_owned());
    }
    let entry = match decode_record(fields) {
        Ok(entry) => entry,
        Err(_) => return HistoryResponse::Error("invalid".to_owned()),
    };
    if !policy.allows(&entry) {
        return HistoryResponse::Ack;
    }
    match recorder.record(entry) {
        Ok(()) => HistoryResponse::Ack,
        Err(error) => HistoryResponse::Error(mbx2_error_kind(error.kind().as_str()).to_owned()),
    }
}

fn handle_query(search: &dyn HistorySearch, fields: &[String]) -> HistoryResponse {
    if fields.len() != QUERY_FIELD_COUNT {
        return HistoryResponse::Error("invalid".to_owned());
    }
    let Some(generation) = unescape_field(&fields[0])
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return HistoryResponse::Error("invalid".to_owned());
    };
    let mode = match unescape_field(&fields[1]) {
        Ok(value) => value,
        Err(_) => return HistoryResponse::Error("invalid".to_owned()),
    };
    let text = match unescape_field(&fields[2]) {
        Ok(value) => value,
        Err(_) => return HistoryResponse::Error("invalid".to_owned()),
    };
    let Some(limit) = unescape_field(&fields[3])
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.min(MAX_QUERY_LIMIT))
    else {
        return HistoryResponse::Error("invalid".to_owned());
    };

    let entries = match run_search(search, &mode, &text, limit) {
        Ok(entries) => entries,
        Err(error) => return HistoryResponse::Error(error),
    };
    let commands: Vec<String> = entries
        .into_iter()
        .map(|entry| entry.command_text)
        .collect();
    HistoryResponse::Result {
        generation,
        commands,
    }
}

fn run_search(
    search: &dyn HistorySearch,
    mode: &str,
    text: &str,
    limit: usize,
) -> Result<Vec<HistoryEntry>, String> {
    let result = match mode {
        "prefix" => {
            if text == "-" {
                return Err("invalid".to_owned());
            }
            search.exact_prefix(text, limit)
        }
        "fuzzy" => {
            if text == "-" {
                return Err("invalid".to_owned());
            }
            search.fuzzy(text, limit)
        }
        "cwd" => {
            if text == "-" {
                return Err("invalid".to_owned());
            }
            search.by_cwd(text, limit)
        }
        "repo" => {
            if text == "-" {
                return Err("invalid".to_owned());
            }
            search.by_repo(text, limit)
        }
        "branch" => {
            if text == "-" {
                return Err("invalid".to_owned());
            }
            search.by_branch(text, limit)
        }
        "failed" => {
            if text != "-" {
                return Err("invalid".to_owned());
            }
            search.failed(limit)
        }
        "recent" => {
            if text != "-" {
                return Err("invalid".to_owned());
            }
            search.recent(limit)
        }
        _ => return Err("unsupported query mode".to_owned()),
    };
    result.map_err(|error| mbx2_error_kind(error.kind().as_str()).to_owned())
}

fn handle_cancel(fields: &[String]) -> HistoryResponse {
    if fields.len() != CANCEL_FIELD_COUNT {
        return HistoryResponse::Error("invalid".to_owned());
    }
    if unescape_field(&fields[0])
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .is_none()
    {
        return HistoryResponse::Error("invalid".to_owned());
    }
    HistoryResponse::Ack
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

/// Maps helper failures onto the protocol allowlist. Unknown/untrusted kind
/// text is never echoed (M-048).
pub fn mbx2_error_kind(kind: &str) -> &'static str {
    match kind {
        "unsupported" => "unsupported",
        "unsupported query mode" => "unsupported query mode",
        "queue_full" => "queue_full",
        "storage" | "storage_failure" | "open" | "migrate" | "write" | "read" => "storage",
        _ => "invalid",
    }
}

pub fn encode_mbx2_error(request_id: u64, kind: &str) -> String {
    format!(
        "{MBX2_MAGIC}\t{request_id}\tERROR\t{}",
        escape_field(mbx2_error_kind(kind))
    )
}

/// Encodes a RESULT frame, dropping trailing candidates that would exceed
/// [`MAX_MESSAGE_BYTES`] so the response stays within the protocol bound.
pub fn encode_mbx2_result(request_id: u64, generation: u64, commands: &[String]) -> String {
    let mut included = Vec::with_capacity(commands.len());
    for command in commands {
        let mut candidate = included.clone();
        candidate.push(command.clone());
        let encoded = encode_mbx2_result_exact(request_id, generation, &candidate);
        if encoded.len() > MAX_MESSAGE_BYTES {
            break;
        }
        included = candidate;
    }
    encode_mbx2_result_exact(request_id, generation, &included)
}

fn encode_mbx2_result_exact(request_id: u64, generation: u64, commands: &[String]) -> String {
    let mut message = format!(
        "{MBX2_MAGIC}\t{request_id}\tRESULT\t{generation}\t{}",
        commands.len()
    );
    for command in commands {
        message.push('\t');
        message.push_str(&escape_field(command));
    }
    message
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

    #[derive(Default)]
    struct StubSearch {
        prefix: Mutex<Vec<HistoryEntry>>,
    }

    impl StubSearch {
        fn with_prefix(entries: Vec<HistoryEntry>) -> Self {
            Self {
                prefix: Mutex::new(entries),
            }
        }
    }

    impl HistorySearch for StubSearch {
        fn recent(&self, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
        }

        fn exact_prefix(
            &self,
            _prefix: &str,
            limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            let entries = self.prefix.lock().unwrap();
            Ok(entries.iter().take(limit).cloned().collect())
        }

        fn exact_prefix_in_cwd(
            &self,
            prefix: &str,
            _cwd: &str,
            limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            self.exact_prefix(prefix, limit)
        }

        fn by_cwd(&self, _cwd: &str, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
        }

        fn by_repo(
            &self,
            _repo_root: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
        }

        fn by_branch(
            &self,
            _repo_branch: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
        }

        fn fuzzy(&self, _needle: &str, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
        }

        fn fuzzy_in_cwd(
            &self,
            _needle: &str,
            _cwd: &str,
            _limit: usize,
        ) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
        }

        fn failed(&self, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
            Ok(Vec::new())
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

    fn sample_entry(command: &str) -> HistoryEntry {
        HistoryEntry {
            session_id: "s".to_owned(),
            event_sequence: 1,
            history_number: Some(1),
            command_text: command.to_owned(),
            start_cwd: "/work".to_owned(),
            completed_at: "2026-08-15T10:00:00Z".to_owned(),
            status: 0,
            duration_ms: None,
            host: "host".to_owned(),
            user: "user".to_owned(),
            repo_root: None,
            repo_branch: None,
        }
    }

    fn service_with_search(search: StubSearch) -> HistoryService {
        let (recorder, _) = RecordingRecorder::shared();
        HistoryService::new(Box::new(recorder), Box::new(search), Box::new(AllowAll))
    }

    fn service_allow() -> HistoryService {
        service_with_search(StubSearch::default())
    }

    #[test]
    fn ping_returns_pong() {
        let service = service_allow();
        assert_eq!(service.handle(1, "PING", &[]), HistoryResponse::Pong);
    }

    #[test]
    fn record_decodes_and_enqueues() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(
            Box::new(recorder),
            Box::new(StubSearch::default()),
            Box::new(AllowAll),
        );
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
        let service = HistoryService::new(
            Box::new(recorder),
            Box::new(StubSearch::default()),
            Box::new(DenyAll),
        );
        let response = service.handle(3, "RECORD", &record_fields());
        assert_eq!(response, HistoryResponse::Ack);
        assert!(probe.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn malformed_record_returns_typed_error() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(
            Box::new(recorder),
            Box::new(StubSearch::default()),
            Box::new(AllowAll),
        );
        let mut fields = record_fields();
        fields[1] = "not-a-number".to_owned();
        let response = service.handle(4, "RECORD", &fields);
        assert!(matches!(response, HistoryResponse::Error(_)));
        assert!(probe.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn percent_escaped_fields_round_trip() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(
            Box::new(recorder),
            Box::new(StubSearch::default()),
            Box::new(AllowAll),
        );
        let mut fields = record_fields();
        fields[3] = "echo%20a%09tab".to_owned();
        let response = service.handle(5, "RECORD", &fields);
        assert_eq!(response, HistoryResponse::Ack);
        assert_eq!(probe.entries.lock().unwrap()[0].command_text, "echo a\ttab");
    }

    #[test]
    fn hostile_sql_command_text_is_recorded_as_data() {
        let (recorder, probe) = RecordingRecorder::shared();
        let service = HistoryService::new(
            Box::new(recorder),
            Box::new(StubSearch::default()),
            Box::new(AllowAll),
        );
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
    fn query_prefix_returns_generation_tagged_result() {
        let search =
            StubSearch::with_prefix(vec![sample_entry("git status"), sample_entry("git push")]);
        let service = service_with_search(search);
        let response = service.handle(
            7,
            "QUERY",
            &[
                "42".to_owned(),
                "prefix".to_owned(),
                "git".to_owned(),
                "10".to_owned(),
            ],
        );
        assert_eq!(
            response,
            HistoryResponse::Result {
                generation: 42,
                commands: vec!["git status".to_owned(), "git push".to_owned()],
            }
        );
    }

    #[test]
    fn query_rejects_bad_field_count() {
        let service = service_allow();
        let response = service.handle(8, "QUERY", &["1".to_owned()]);
        assert_eq!(response, HistoryResponse::Error("invalid".to_owned()));
    }

    #[test]
    fn query_failed_requires_dash_text() {
        let service = service_allow();
        let response = service.handle(
            9,
            "QUERY",
            &[
                "1".to_owned(),
                "failed".to_owned(),
                "nope".to_owned(),
                "5".to_owned(),
            ],
        );
        assert_eq!(response, HistoryResponse::Error("invalid".to_owned()));
    }

    #[test]
    fn cancel_acknowledges_generation() {
        let service = service_allow();
        assert_eq!(
            service.handle(10, "CANCEL", &["99".to_owned()]),
            HistoryResponse::Ack
        );
    }

    #[test]
    fn encode_result_escapes_command_fields() {
        let encoded = encode_mbx2_result(3, 7, &["a\tb".to_owned(), "c".to_owned()]);
        assert_eq!(encoded, "MBX2\t3\tRESULT\t7\t2\ta%09b\tc");
    }

    #[test]
    fn encode_result_drops_overflow_candidates() {
        let huge = "x".repeat(MAX_MESSAGE_BYTES);
        let encoded = encode_mbx2_result(1, 1, &[huge, "kept-small".to_owned()]);
        assert!(encoded.len() <= MAX_MESSAGE_BYTES);
        assert!(encoded.contains("RESULT\t1\t0") || encoded.ends_with("\t0"));
        assert!(!encoded.contains("kept-small"));
    }

    #[test]
    fn unknown_kind_is_unsupported_and_does_not_echo() {
        let service = service_allow();
        let huge = "X".repeat(MAX_MESSAGE_BYTES);
        let response = service.handle(1, &huge, &[]);
        assert_eq!(response, HistoryResponse::Error("unsupported".to_owned()));
        let encoded = encode_mbx2_error(1, &format!("unknown MBX2 kind: {huge}"));
        assert!(encoded.len() <= MAX_MESSAGE_BYTES);
        assert_eq!(encoded, "MBX2\t1\tERROR\tinvalid");
        assert!(!encoded.contains("unknown"));
        assert!(!encoded.contains(&huge[..32]));
    }

    #[test]
    fn encode_error_uses_typed_kinds_and_escapes() {
        assert_eq!(
            encode_mbx2_error(4, "queue_full"),
            "MBX2\t4\tERROR\tqueue_full"
        );
        assert_eq!(
            encode_mbx2_error(4, "storage_failure"),
            "MBX2\t4\tERROR\tstorage"
        );
        assert_eq!(
            encode_mbx2_error(4, "unsupported query mode"),
            "MBX2\t4\tERROR\tunsupported query mode"
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
