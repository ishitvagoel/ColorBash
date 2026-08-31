use crate::highlight_service::HighlightHandler;
use crate::history_service::{HistoryHandler, MBX2_MAGIC};
use crate::service::RequestHandler;
use crate::telemetry::trace_message;
use mbx_protocol::{MAX_MESSAGE_BYTES, Request, Response, ResponseKind, validate_message_line};
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn serve_stdio(
    handler: &dyn RequestHandler,
    history: Option<Box<dyn HistoryHandler>>,
    highlight: Option<Box<dyn HighlightHandler>>,
) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_connection(
        &mut stdin.lock(),
        &mut BufWriter::new(stdout.lock()),
        handler,
        history.as_deref(),
        highlight.as_deref(),
    )
}

pub fn serve_socket(
    path: &Path,
    handler: &dyn RequestHandler,
    history: Option<Box<dyn HistoryHandler>>,
    highlight: Option<Box<dyn HighlightHandler>>,
) -> Result<(), String> {
    let (listener, _cleanup) = bind_socket(path)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = serve_socket_connection(
                    stream,
                    handler,
                    history.as_deref(),
                    highlight.as_deref(),
                ) {
                    trace_message(&format!("socket_client_error detail={error}"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(())
}

fn serve_socket_connection(
    stream: UnixStream,
    handler: &dyn RequestHandler,
    history: Option<&dyn HistoryHandler>,
    highlight: Option<&dyn HighlightHandler>,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(io_error)?;
    let reader_stream = stream.try_clone().map_err(io_error)?;
    serve_connection(
        &mut BufReader::new(reader_stream),
        &mut BufWriter::new(stream),
        handler,
        history,
        highlight,
    )
}

/// Shared request loop used by every server-side stream adapter. MBX2 frames
/// dispatch by kind to whichever optional handler owns that kind (history
/// handles RECORD/PING/QUERY/CANCEL; highlight handles HIGHLIGHT,
/// independently of whether history is enabled — ADR 0014). MBX1 frames go to
/// the prompt request handler.
fn serve_connection(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    handler: &dyn RequestHandler,
    history: Option<&dyn HistoryHandler>,
    highlight: Option<&dyn HighlightHandler>,
) -> Result<(), String> {
    let mut line = String::new();
    loop {
        if read_bounded_line(reader, &mut line)? == 0 {
            return Ok(());
        }
        if line.starts_with(&format!("{MBX2_MAGIC}\t")) {
            let response = handle_mbx2_line(&line, history, highlight);
            write_message(writer, &response)?;
            continue;
        }
        let response = match Request::decode(&line) {
            Ok(request) => Response {
                id: request.id,
                kind: handler.handle(request),
            },
            Err(error) => Response {
                id: 0,
                kind: ResponseKind::Error(error.to_string()),
            },
        };
        write_message(writer, &response.encode())?;
    }
}

/// Best-effort peek at the MBX2 `kind` field, tolerant of a line too
/// malformed to have one. Used only to route to the right optional handler
/// before either handler's own validation runs; it changes nothing about how
/// a malformed frame is rejected.
fn mbx2_kind_hint(line: &str) -> &str {
    line.split('\t').nth(2).unwrap_or_default()
}

fn handle_mbx2_line(
    line: &str,
    history: Option<&dyn HistoryHandler>,
    highlight: Option<&dyn HighlightHandler>,
) -> String {
    let request_id = mbx2_request_id(line);
    if mbx2_kind_hint(line) == "HIGHLIGHT" {
        let Some(handler) = highlight else {
            return crate::history_service::encode_mbx2_error(request_id, "unsupported");
        };
        return dispatch_mbx2(line, request_id, |id, kind, rest| {
            match handler.handle(id, kind, rest) {
                crate::highlight_service::HighlightResponse::Styled {
                    generation,
                    point,
                    line,
                } => crate::highlight_service::encode_mbx2_styled(id, generation, point, &line),
                crate::highlight_service::HighlightResponse::Error(kind) => {
                    crate::history_service::encode_mbx2_error(id, &kind)
                }
            }
        });
    }
    let Some(handler) = history else {
        return crate::history_service::encode_mbx2_error(request_id, "unsupported");
    };
    dispatch_mbx2(line, request_id, |id, kind, rest| {
        match handler.handle(id, kind, rest) {
            crate::history_service::HistoryResponse::Pong => {
                crate::history_service::encode_mbx2(id, "PONG")
            }
            crate::history_service::HistoryResponse::Ack => {
                crate::history_service::encode_mbx2(id, "ACK")
            }
            crate::history_service::HistoryResponse::Result {
                generation,
                commands,
            } => crate::history_service::encode_mbx2_result(id, generation, &commands),
            crate::history_service::HistoryResponse::Error(kind) => {
                crate::history_service::encode_mbx2_error(id, &kind)
            }
        }
    })
}

/// Shared validation and field-splitting for both MBX2 handler branches: the
/// magic must match, the request id must decode, and the rest of the frame is
/// handed to `respond` as owned fields.
fn dispatch_mbx2(
    line: &str,
    request_id: u64,
    respond: impl FnOnce(u64, &str, &[String]) -> String,
) -> String {
    if validate_message_line(line).is_err() {
        return crate::history_service::encode_mbx2_error(request_id, "invalid");
    }
    let mut fields = line.split('\t');
    let magic = fields.next().unwrap_or_default();
    if magic != MBX2_MAGIC {
        return crate::history_service::encode_mbx2_error(request_id, "invalid");
    }
    let request_id = match fields.next().and_then(|value| value.parse::<u64>().ok()) {
        Some(id) => id,
        None => return crate::history_service::encode_mbx2_error(request_id, "invalid"),
    };
    let kind = fields.next().unwrap_or_default();
    let rest: Vec<String> = fields.map(str::to_owned).collect();
    respond(request_id, kind, &rest)
}

fn mbx2_request_id(line: &str) -> u64 {
    let mut fields = line.split('\t');
    if fields.next() != Some(MBX2_MAGIC) {
        return 0;
    }
    fields
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

pub struct ClientSession<R, W> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> ClientSession<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    /// Exchanges one request and centrally enforces framing and correlation IDs.
    pub fn exchange(&mut self, request: &Request) -> Result<Response, String> {
        write_message(&mut self.writer, &request.encode())?;
        let mut line = String::new();
        if read_bounded_line(&mut self.reader, &mut line)? == 0 {
            return Err("socket server closed before sending a response".to_owned());
        }
        let response = Response::decode(&line).map_err(|error| error.to_string())?;
        if response.id != request.id {
            return Err(format!(
                "response id {} does not match request id {}",
                response.id, request.id
            ));
        }
        Ok(response)
    }

    #[cfg(test)]
    fn into_parts(self) -> (R, W) {
        (self.reader, self.writer)
    }
}

pub struct SocketClient {
    session: ClientSession<BufReader<UnixStream>, BufWriter<UnixStream>>,
}

impl SocketClient {
    pub fn connect(path: &Path) -> Result<Self, String> {
        let stream = UnixStream::connect(path).map_err(io_error)?;
        let reader_stream = stream.try_clone().map_err(io_error)?;
        Ok(Self {
            session: ClientSession::new(BufReader::new(reader_stream), BufWriter::new(stream)),
        })
    }

    pub fn exchange(&mut self, request: &Request) -> Result<Response, String> {
        self.session.exchange(request)
    }
}

fn ensure_socket_path_available(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => Err(format!(
            "socket already exists at {}; remove it only after confirming no server is active",
            path.display()
        )),
        Ok(_) => Err(format!(
            "refusing to replace non-socket path: {}",
            path.display()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn bind_socket(path: &Path) -> Result<(UnixListener, SocketCleanup), String> {
    ensure_socket_path_available(path)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)
                .map_err(io_error)?;
        }
    }
    let listener = UnixListener::bind(path).map_err(io_error)?;
    let cleanup = SocketCleanup(path.to_path_buf());
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_error)?;
    Ok((listener, cleanup))
}

fn read_bounded_line(reader: &mut impl BufRead, line: &mut String) -> Result<usize, String> {
    line.clear();
    let mut limited = reader.take((MAX_MESSAGE_BYTES + 2) as u64);
    let bytes = limited.read_line(line).map_err(io_error)?;
    trim_line_ending(line);
    if line.len() > MAX_MESSAGE_BYTES {
        return Err("protocol message exceeds 64 KiB".to_owned());
    }
    Ok(bytes)
}

fn write_message(writer: &mut impl Write, encoded: &str) -> Result<(), String> {
    if encoded.len() > MAX_MESSAGE_BYTES {
        return Err("encoded message exceeds protocol limit".to_owned());
    }
    writeln!(writer, "{encoded}").map_err(io_error)?;
    writer.flush().map_err(io_error)
}

fn trim_line_ending(value: &mut String) {
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}

struct SocketCleanup(PathBuf);

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mbx_protocol::RequestKind;
    use std::cell::RefCell;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    struct RecordingHandler {
        requests: RefCell<Vec<Request>>,
        response: ResponseKind,
    }

    impl RecordingHandler {
        fn returning(response: ResponseKind) -> Self {
            Self {
                requests: RefCell::new(Vec::new()),
                response,
            }
        }
    }

    impl RequestHandler for RecordingHandler {
        fn handle(&self, request: Request) -> ResponseKind {
            self.requests.borrow_mut().push(request);
            self.response.clone()
        }
    }

    #[test]
    fn shared_server_loop_uses_handler_content_and_owns_response_ids() {
        let input = b"MBX1\tbad\tPING\nMBX1\t7\tPING\n";
        let mut reader = Cursor::new(input);
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("sentinel".to_owned()));

        serve_connection(&mut reader, &mut writer, &handler, None, None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX1\t0\tERROR\tinvalid request id\nMBX1\t7\tPROMPT\tsentinel\n"
        );
        assert_eq!(
            handler.requests.borrow().as_slice(),
            &[Request {
                id: 7,
                kind: RequestKind::Ping,
            }]
        );
    }

    #[test]
    fn shared_server_loop_rejects_oversized_handler_content_before_writing() {
        let mut reader = Cursor::new(b"MBX1\t7\tPING\n".to_vec());
        let mut writer = Vec::new();
        let handler =
            RecordingHandler::returning(ResponseKind::Prompt("x".repeat(MAX_MESSAGE_BYTES)));

        assert_eq!(
            serve_connection(&mut reader, &mut writer, &handler, None, None).unwrap_err(),
            "encoded message exceeds protocol limit"
        );
        assert!(writer.is_empty());
        assert_eq!(handler.requests.borrow().len(), 1);
    }

    #[test]
    fn mbx2_frames_dispatch_to_the_history_handler() {
        use crate::history_service::HistoryResponse;
        use std::sync::Mutex;

        struct RecordingHistory {
            calls: Mutex<Vec<(u64, String, usize)>>,
        }

        impl HistoryHandler for RecordingHistory {
            fn handle(&self, id: u64, kind: &str, fields: &[String]) -> HistoryResponse {
                self.calls
                    .lock()
                    .unwrap()
                    .push((id, kind.to_owned(), fields.len()));
                if kind == "PING" {
                    HistoryResponse::Pong
                } else {
                    HistoryResponse::Ack
                }
            }
        }

        let input = b"MBX2\t3\tRECORD\ta\t1\t2\tcmd\t/w\tnow\t0\t-\th\tu\nMBX2\t4\tPING\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let history = RecordingHistory {
            calls: Mutex::new(Vec::new()),
        };

        serve_connection(&mut reader, &mut writer, &handler, Some(&history), None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t3\tACK\nMBX2\t4\tPONG\n"
        );
        assert!(handler.requests.borrow().is_empty());
        assert_eq!(
            history.calls.lock().unwrap().as_slice(),
            &[(3, "RECORD".to_owned(), 10), (4, "PING".to_owned(), 0)]
        );
    }

    #[test]
    fn mbx2_query_frames_encode_result_responses() {
        use crate::history_service::HistoryResponse;

        struct QueryHistory;

        impl HistoryHandler for QueryHistory {
            fn handle(&self, id: u64, kind: &str, fields: &[String]) -> HistoryResponse {
                assert_eq!(id, 5);
                assert_eq!(kind, "QUERY");
                assert_eq!(fields.len(), 4);
                HistoryResponse::Result {
                    generation: 42,
                    commands: vec!["git status".to_owned(), "a\tb".to_owned()],
                }
            }
        }

        let input = b"MBX2\t5\tQUERY\t42\tprefix\tgit\t5\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let history = QueryHistory;

        serve_connection(&mut reader, &mut writer, &handler, Some(&history), None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t5\tRESULT\t42\t2\tgit status\ta%09b\n"
        );
    }

    #[test]
    fn mbx2_cancel_frames_encode_ack_responses() {
        use crate::history_service::HistoryResponse;

        struct CancelHistory;

        impl HistoryHandler for CancelHistory {
            fn handle(&self, id: u64, kind: &str, fields: &[String]) -> HistoryResponse {
                assert_eq!(id, 6);
                assert_eq!(kind, "CANCEL");
                assert_eq!(fields, &["99".to_owned()]);
                HistoryResponse::Ack
            }
        }

        let input = b"MBX2\t6\tCANCEL\t99\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let history = CancelHistory;

        serve_connection(&mut reader, &mut writer, &handler, Some(&history), None).unwrap();

        assert_eq!(String::from_utf8(writer).unwrap(), "MBX2\t6\tACK\n");
    }

    #[test]
    fn mbx2_highlight_frames_dispatch_to_the_highlight_handler_independently_of_history() {
        use crate::highlight_service::HighlightResponse;

        struct StubHighlight;

        impl HighlightHandler for StubHighlight {
            fn handle(&self, id: u64, kind: &str, fields: &[String]) -> HighlightResponse {
                assert_eq!(id, 5);
                assert_eq!(kind, "HIGHLIGHT");
                assert_eq!(fields.len(), 4);
                HighlightResponse::Styled {
                    generation: 42,
                    point: 3,
                    line: "styled".to_owned(),
                }
            }
        }

        // No history handler at all: HIGHLIGHT must not require MBX_HISTORY=1.
        let input = b"MBX2\t5\tHIGHLIGHT\t42\t1\t0\tplain\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let highlight = StubHighlight;

        serve_connection(&mut reader, &mut writer, &handler, None, Some(&highlight)).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t5\tSTYLED\t42\t3\tstyled\n"
        );
    }

    #[test]
    fn mbx2_without_highlight_handler_fails_closed_even_with_history_present() {
        use crate::history_service::HistoryResponse;

        struct AnyHistory;
        impl HistoryHandler for AnyHistory {
            fn handle(&self, _id: u64, _kind: &str, _fields: &[String]) -> HistoryResponse {
                panic!("a HIGHLIGHT frame must never reach the history handler");
            }
        }

        let input = b"MBX2\t5\tHIGHLIGHT\t1\t1\t0\tplain\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let history = AnyHistory;

        serve_connection(&mut reader, &mut writer, &handler, Some(&history), None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t5\tERROR\tunsupported\n"
        );
    }

    #[test]
    fn mbx2_without_history_handler_fails_closed() {
        let input = b"MBX2\t3\tRECORD\ta\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));

        serve_connection(&mut reader, &mut writer, &handler, None, None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t3\tERROR\tunsupported\n"
        );
        assert!(handler.requests.borrow().is_empty());
    }

    #[test]
    fn mbx2_invalid_frame_fails_closed() {
        let input = b"MBX2\tbogus\tPING\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let history = RecordingHistoryStub;

        serve_connection(&mut reader, &mut writer, &handler, Some(&history), None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t0\tERROR\tinvalid\n"
        );
        assert!(handler.requests.borrow().is_empty());
    }

    #[test]
    fn mbx2_unescaped_control_fails_closed_with_request_id() {
        let input = b"MBX2\t5\tQUERY\t1\tprefix\t\x01needle\t8\n";
        let mut reader = Cursor::new(input.to_vec());
        let mut writer = Vec::new();
        let handler = RecordingHandler::returning(ResponseKind::Prompt("x".to_owned()));
        let history = RecordingHistoryStub;

        serve_connection(&mut reader, &mut writer, &handler, Some(&history), None).unwrap();

        assert_eq!(
            String::from_utf8(writer).unwrap(),
            "MBX2\t5\tERROR\tinvalid\n"
        );
        assert!(handler.requests.borrow().is_empty());
    }

    struct RecordingHistoryStub;

    impl HistoryHandler for RecordingHistoryStub {
        fn handle(
            &self,
            _id: u64,
            _kind: &str,
            _fields: &[String],
        ) -> crate::history_service::HistoryResponse {
            panic!("invalid magic must not reach the history handler")
        }
    }

    #[test]
    fn client_session_writes_request_and_validates_matching_response() {
        let reader = Cursor::new(b"MBX1\t9\tPONG\n".to_vec());
        let writer = Vec::new();
        let mut client = ClientSession::new(reader, writer);
        let request = Request {
            id: 9,
            kind: RequestKind::Ping,
        };

        assert_eq!(client.exchange(&request).unwrap().kind, ResponseKind::Pong);
        let (_, writer) = client.into_parts();
        assert_eq!(String::from_utf8(writer).unwrap(), "MBX1\t9\tPING\n");
    }

    #[test]
    fn client_session_rejects_mismatched_response_ids() {
        let reader = Cursor::new(b"MBX1\t10\tPONG\n".to_vec());
        let mut client = ClientSession::new(reader, Vec::new());
        let request = Request {
            id: 9,
            kind: RequestKind::Ping,
        };

        assert_eq!(
            client.exchange(&request).unwrap_err(),
            "response id 10 does not match request id 9"
        );
    }

    #[test]
    fn line_reader_applies_the_same_payload_limit_to_eof_lf_and_crlf() {
        let endings: [(&str, &[u8]); 3] = [("EOF", b""), ("LF", b"\n"), ("CRLF", b"\r\n")];

        for payload_length in [MAX_MESSAGE_BYTES - 1, MAX_MESSAGE_BYTES] {
            for (ending_name, ending) in endings {
                let mut input = vec![b'a'; payload_length];
                input.extend_from_slice(ending);
                let input_length = input.len();
                let mut reader = Cursor::new(input);
                let mut line = String::new();

                assert_eq!(
                    read_bounded_line(&mut reader, &mut line).unwrap(),
                    input_length,
                    "{ending_name} rejected a {payload_length}-byte payload"
                );
                assert_eq!(line.len(), payload_length);
            }
        }

        for (ending_name, ending) in endings {
            let mut input = vec![b'a'; MAX_MESSAGE_BYTES + 1];
            input.extend_from_slice(ending);
            let mut reader = Cursor::new(input);
            let mut line = String::new();

            assert_eq!(
                read_bounded_line(&mut reader, &mut line).unwrap_err(),
                "protocol message exceeds 64 KiB",
                "{ending_name} accepted a payload over the limit"
            );
        }
    }

    #[test]
    fn socket_binding_refuses_existing_files_and_sockets() {
        let directory = TestDirectory::new();
        let file_path = directory.path().join("file");
        fs::write(&file_path, b"keep me").unwrap();

        let file_error = bind_socket_error(&file_path);
        assert!(file_error.contains("refusing to replace non-socket path"));
        assert_eq!(fs::read(&file_path).unwrap(), b"keep me");

        let socket_path = directory.path().join("socket");
        let existing_listener = UnixListener::bind(&socket_path).unwrap();
        let socket_error = bind_socket_error(&socket_path);
        assert!(socket_error.contains("socket already exists"));
        assert!(
            fs::symlink_metadata(&socket_path)
                .unwrap()
                .file_type()
                .is_socket()
        );
        drop(existing_listener);
    }

    #[test]
    fn socket_binding_sets_user_only_mode_and_cleanup_removes_the_socket() {
        let directory = TestDirectory::new();
        let socket_path = directory.path().join("socket");
        let (listener, cleanup) = bind_socket(&socket_path).unwrap();

        let mode = fs::symlink_metadata(&socket_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);

        drop(listener);
        drop(cleanup);
        assert_eq!(
            fs::symlink_metadata(&socket_path).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    fn spawn_socket_server_that_reads_request_then_writes_response(
        listener: UnixListener,
        encoded_response: &str,
    ) -> thread::JoinHandle<()> {
        let encoded_response = encoded_response.to_owned();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = BufWriter::new(stream);
            let mut line = String::new();
            read_bounded_line(&mut reader, &mut line).expect("client request");
            write_message(&mut writer, &encoded_response).expect("server response");
        })
    }

    #[test]
    fn socket_client_rejects_a_mismatched_response_id() {
        let directory = TestDirectory::new();
        let socket_path = directory.path().join("socket");
        let (listener, cleanup) = bind_socket(&socket_path).unwrap();
        let server =
            spawn_socket_server_that_reads_request_then_writes_response(listener, "MBX1\t10\tPONG");
        let mut client = SocketClient::connect(&socket_path).unwrap();
        let request = Request {
            id: 9,
            kind: RequestKind::Ping,
        };

        assert_eq!(
            client.exchange(&request).unwrap_err(),
            "response id 10 does not match request id 9"
        );

        drop(client);
        server.join().unwrap();
        drop(cleanup);
    }

    #[test]
    fn socket_client_rejects_a_mismatched_response_id_after_request_handshake() {
        let directory = TestDirectory::new();
        let socket_path = directory.path().join("socket");
        let (listener, cleanup) = bind_socket(&socket_path).unwrap();
        let (request_observed, request_observed_rx) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = BufWriter::new(stream);
            let mut line = String::new();
            read_bounded_line(&mut reader, &mut line).expect("client request");
            assert_eq!(line, "MBX1\t9\tPING");
            request_observed.send(()).expect("request handshake");
            write_message(&mut writer, "MBX1\t10\tPONG").expect("server response");
        });
        let mut client = SocketClient::connect(&socket_path).unwrap();
        let request = Request {
            id: 9,
            kind: RequestKind::Ping,
        };

        assert_eq!(
            client.exchange(&request).unwrap_err(),
            "response id 10 does not match request id 9"
        );
        request_observed_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("server must observe the client request before responding");

        drop(client);
        server.join().unwrap();
        drop(cleanup);
    }

    fn bind_socket_error(path: &Path) -> String {
        match bind_socket(path) {
            Ok(_) => panic!("socket binding unexpectedly replaced an existing path"),
            Err(error) => error,
        }
    }

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            loop {
                let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("mbx-transport-{}-{sequence}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("failed to create transport test directory: {error}"),
                }
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
