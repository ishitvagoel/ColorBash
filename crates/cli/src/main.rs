use mbx_protocol::{
    FLAG_ASCII_ICONS, FLAG_DISABLE_GIT, FLAG_NERD_ICONS, FLAG_NO_COLOR, FLAG_PRODUCTION, FLAG_SSH,
    MAX_MESSAGE_BYTES, PromptRequest, Request, RequestKind, Response, ResponseKind,
};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mbx: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("handshake") => {
            println!("mbx/{VERSION} ready");
            Ok(())
        }
        Some("prompt") => {
            let prompt = prompt_from_cli(&args[2..])?;
            let started = Instant::now();
            print!("{}", render_prompt(&prompt));
            io::stdout().flush().map_err(io_error)?;
            trace_duration("prompt_render", started);
            Ok(())
        }
        Some("serve") => serve(&args[2..]),
        Some("socket-ping") => socket_ping(&args[2..]),
        Some("benchmark-client") => benchmark_client(&args[2..]),
        Some("--version" | "-V") => {
            println!("mbx {VERSION}");
            Ok(())
        }
        Some("--help" | "-h") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

fn print_help() {
    println!(
        "mbx {VERSION}\n\n\
         Bash-compatible terminal UX foundation prototype\n\n\
         USAGE:\n  mbx handshake\n  mbx prompt [OPTIONS]\n  mbx serve --stdio\n  \
         mbx serve --socket PATH\n  mbx socket-ping --socket PATH\n  \
         mbx benchmark-client --socket PATH [--iterations N]\n\n\
         PROMPT OPTIONS:\n  --cwd PATH  --status N  --duration-ms N\n  \
         --no-color  --ascii  --nerd-font  --ssh  --production  --disable-git"
    );
}

fn prompt_from_cli(args: &[String]) -> Result<PromptRequest, String> {
    let cwd = env::current_dir()
        .map_err(io_error)?
        .to_string_lossy()
        .into_owned();
    let mut prompt = PromptRequest {
        cwd,
        status: 0,
        duration_ms: None,
        flags: environment_flags(),
    };

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--cwd" => prompt.cwd = next_value(args, &mut index, "--cwd")?.to_owned(),
            "--status" => {
                prompt.status = next_value(args, &mut index, "--status")?
                    .parse::<u8>()
                    .map_err(|_| "--status must be between 0 and 255".to_owned())?;
            }
            "--duration-ms" => {
                prompt.duration_ms = Some(
                    next_value(args, &mut index, "--duration-ms")?
                        .parse::<u64>()
                        .map_err(|_| "--duration-ms must be an unsigned integer".to_owned())?,
                );
            }
            "--no-color" => prompt.flags |= FLAG_NO_COLOR,
            "--ascii" => {
                prompt.flags |= FLAG_ASCII_ICONS;
                prompt.flags &= !FLAG_NERD_ICONS;
            }
            "--nerd-font" => {
                prompt.flags |= FLAG_NERD_ICONS;
                prompt.flags &= !FLAG_ASCII_ICONS;
            }
            "--ssh" => prompt.flags |= FLAG_SSH,
            "--production" => prompt.flags |= FLAG_PRODUCTION,
            "--disable-git" => prompt.flags |= FLAG_DISABLE_GIT,
            unknown => return Err(format!("unknown prompt option: {unknown}")),
        }
        index += 1;
    }
    Ok(prompt)
}

fn next_value<'a>(args: &'a [String], index: &mut usize, option: &str) -> Result<&'a str, String> {
    *index += 1;
    args.get(*index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn environment_flags() -> u32 {
    let mut flags = 0;
    if env::var_os("NO_COLOR").is_some()
        || env::var("MBX_COLOR").is_ok_and(|value| value == "never")
        || env::var("TERM").is_ok_and(|value| value == "dumb")
    {
        flags |= FLAG_NO_COLOR;
    }
    match env::var("MBX_ICONS").as_deref() {
        Ok("nerd") => flags |= FLAG_NERD_ICONS,
        Ok("never" | "ascii") => flags |= FLAG_ASCII_ICONS,
        _ => {}
    }
    if env::var_os("SSH_CONNECTION").is_some() || env::var_os("SSH_TTY").is_some() {
        flags |= FLAG_SSH;
    }
    if env::var("MBX_PRODUCTION_CONTEXT").is_ok_and(|value| value == "1") {
        flags |= FLAG_PRODUCTION;
    }
    if env::var("MBX_DISABLE_GIT").is_ok_and(|value| value == "1") {
        flags |= FLAG_DISABLE_GIT;
    }
    flags
}

fn serve(args: &[String]) -> Result<(), String> {
    match args {
        [mode] if mode == "--stdio" => serve_stdio(),
        [mode, socket] if mode == "--socket" => serve_socket(Path::new(socket)),
        _ => Err("serve requires exactly --stdio or --socket PATH".to_owned()),
    }
}

fn serve_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = BufWriter::new(stdout.lock());
    let mut line = String::new();
    loop {
        if read_bounded_line(&mut reader, &mut line)? == 0 {
            return Ok(());
        }
        write_response(&mut writer, handle_wire_request(&line))?;
    }
}

fn serve_socket(path: &Path) -> Result<(), String> {
    ensure_socket_path_available(path)?;
    let listener = UnixListener::bind(path).map_err(io_error)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_error)?;
    let _cleanup = SocketCleanup(path.to_path_buf());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_socket_stream(stream) {
                    trace_message(&format!("socket_client_error detail={error}"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(())
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

fn handle_socket_stream(stream: UnixStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(io_error)?;
    let reader_stream = stream.try_clone().map_err(io_error)?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = BufWriter::new(stream);
    let mut line = String::new();
    loop {
        let bytes = read_bounded_line(&mut reader, &mut line)?;
        if bytes == 0 {
            return Ok(());
        }
        write_response(&mut writer, handle_wire_request(&line))?;
    }
}

fn read_bounded_line(reader: &mut impl BufRead, line: &mut String) -> Result<usize, String> {
    line.clear();
    let mut limited = reader.take((MAX_MESSAGE_BYTES + 2) as u64);
    let bytes = limited.read_line(line).map_err(io_error)?;
    if bytes > MAX_MESSAGE_BYTES + 1 {
        return Err("protocol message exceeds 64 KiB".to_owned());
    }
    if bytes > 0 && !line.ends_with('\n') && bytes > MAX_MESSAGE_BYTES {
        return Err("protocol message exceeds 64 KiB".to_owned());
    }
    trim_line_ending(line);
    Ok(bytes)
}

fn handle_wire_request(line: &str) -> Response {
    let parsed = Request::decode(line);
    match parsed {
        Ok(Request {
            id,
            kind: RequestKind::Ping,
        }) => Response {
            id,
            kind: ResponseKind::Pong,
        },
        Ok(Request {
            id,
            kind: RequestKind::Prompt(prompt),
        }) => Response {
            id,
            kind: ResponseKind::Prompt(render_prompt(&prompt)),
        },
        Err(error) => Response {
            id: 0,
            kind: ResponseKind::Error(error.to_string()),
        },
    }
}

fn write_response(writer: &mut impl Write, response: Response) -> Result<(), String> {
    let encoded = response.encode();
    if encoded.len() > MAX_MESSAGE_BYTES {
        return Err("encoded response exceeds protocol limit".to_owned());
    }
    writeln!(writer, "{encoded}").map_err(io_error)?;
    writer.flush().map_err(io_error)
}

fn socket_ping(args: &[String]) -> Result<(), String> {
    let socket = socket_arg(args)?;
    let mut stream = UnixStream::connect(socket).map_err(io_error)?;
    let request = Request {
        id: 1,
        kind: RequestKind::Ping,
    };
    writeln!(stream, "{}", request.encode()).map_err(io_error)?;
    let mut response = String::new();
    read_bounded_line(&mut BufReader::new(stream), &mut response)?;
    let response = Response::decode(&response).map_err(|error| error.to_string())?;
    if response.kind != ResponseKind::Pong {
        return Err("socket server returned an unexpected response".to_owned());
    }
    println!("mbx/{VERSION} socket ready");
    Ok(())
}

fn benchmark_client(args: &[String]) -> Result<(), String> {
    let mut socket: Option<&str> = None;
    let mut iterations = 1_000_u64;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--socket" => socket = Some(next_value(args, &mut index, "--socket")?),
            "--iterations" => {
                iterations = next_value(args, &mut index, "--iterations")?
                    .parse::<u64>()
                    .map_err(|_| "--iterations must be an unsigned integer".to_owned())?;
            }
            unknown => return Err(format!("unknown benchmark option: {unknown}")),
        }
        index += 1;
    }
    if iterations == 0 {
        return Err("--iterations must be greater than zero".to_owned());
    }
    let socket = socket.ok_or_else(|| "--socket is required".to_owned())?;
    let stream = UnixStream::connect(socket).map_err(io_error)?;
    let reader_stream = stream.try_clone().map_err(io_error)?;
    let mut writer = BufWriter::new(stream);
    let mut reader = BufReader::new(reader_stream);
    let started = Instant::now();
    let mut line = String::new();
    for id in 1..=iterations {
        let request = Request {
            id,
            kind: RequestKind::Ping,
        };
        writeln!(writer, "{}", request.encode()).map_err(io_error)?;
        writer.flush().map_err(io_error)?;
        read_bounded_line(&mut reader, &mut line)?;
        let response = Response::decode(&line).map_err(|error| error.to_string())?;
        if response.id != id || response.kind != ResponseKind::Pong {
            return Err(format!("unexpected response for request {id}"));
        }
    }
    let elapsed = started.elapsed();
    println!(
        "transport=unix-socket iterations={iterations} total_ns={} mean_ns={}",
        elapsed.as_nanos(),
        elapsed.as_nanos() / u128::from(iterations)
    );
    Ok(())
}

fn socket_arg(args: &[String]) -> Result<&str, String> {
    match args {
        [option, socket] if option == "--socket" => Ok(socket),
        _ => Err("--socket PATH is required".to_owned()),
    }
}

fn render_prompt(request: &PromptRequest) -> String {
    let color = request.flags & FLAG_NO_COLOR == 0;
    let nerd = request.flags & FLAG_NERD_ICONS != 0 && request.flags & FLAG_ASCII_ICONS == 0;
    let theme = Theme::default();
    let mut segments = Vec::new();

    if request.flags & FLAG_PRODUCTION != 0 {
        let host = env::var("HOSTNAME").unwrap_or_else(|_| "host".to_owned());
        let user = env::var("USER").unwrap_or_else(|_| "user".to_owned());
        segments.push(styled(
            &format!(
                "{} PROD · {} · {}",
                if nerd { "󰀪" } else { "!" },
                sanitize_for_ps1(&host),
                sanitize_for_ps1(&user)
            ),
            theme.danger,
            color,
        ));
    } else if request.flags & FLAG_SSH != 0 {
        let host = env::var("HOSTNAME").unwrap_or_else(|_| "remote".to_owned());
        segments.push(styled(
            &format!(
                "{} {}",
                if nerd { "󰒍" } else { "ssh:" },
                sanitize_for_ps1(&host)
            ),
            theme.warning,
            color,
        ));
    }

    segments.push(styled(
        &format!(
            "{}{}",
            if nerd { " " } else { "" },
            display_path(&request.cwd)
        ),
        theme.path,
        color,
    ));

    if request.flags & FLAG_DISABLE_GIT == 0 {
        if let Some(git) = read_git_status(Path::new(&request.cwd)) {
            let mut text = format!(
                "{}{}",
                if nerd { "󰊢 " } else { "git:" },
                sanitize_for_ps1(&git.branch)
            );
            if git.staged > 0 {
                text.push_str(&format!(" +{}", git.staged));
            }
            if git.modified > 0 {
                text.push_str(&format!(" ~{}", git.modified));
            }
            if git.untracked > 0 {
                text.push_str(&format!(" ?{}", git.untracked));
            }
            segments.push(styled(
                &text,
                if git.staged + git.modified + git.untracked == 0 {
                    theme.git_clean
                } else {
                    theme.git_dirty
                },
                color,
            ));
        }
    }

    if request.status != 0 {
        segments.push(styled(
            &format!("{} {}", if nerd { "" } else { "exit" }, request.status),
            theme.error,
            color,
        ));
    }

    if let Some(duration) = request.duration_ms.filter(|duration| *duration >= 2_000) {
        segments.push(styled(&format_duration(duration), theme.muted, color));
    }

    let arrow = styled(if nerd { "❯" } else { ">" }, theme.primary, color);
    format!("{}\\n{} ", segments.join("  "), arrow)
}

#[derive(Clone, Copy)]
struct Theme {
    primary: &'static str,
    path: &'static str,
    git_clean: &'static str,
    git_dirty: &'static str,
    warning: &'static str,
    danger: &'static str,
    error: &'static str,
    muted: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: "1;38;5;81",
            path: "1;38;5;117",
            git_clean: "38;5;114",
            git_dirty: "38;5;215",
            warning: "1;38;5;215",
            danger: "1;38;5;196",
            error: "1;38;5;203",
            muted: "38;5;245",
        }
    }
}

fn styled(text: &str, ansi_role: &str, enabled: bool) -> String {
    if enabled {
        format!("\\[\\e[{ansi_role}m\\]{text}\\[\\e[0m\\]")
    } else {
        text.to_owned()
    }
}

fn display_path(cwd: &str) -> String {
    let compact = match env::var("HOME") {
        Ok(home) if cwd == home => "~".to_owned(),
        Ok(home) if cwd.starts_with(&home) && cwd.as_bytes().get(home.len()) == Some(&b'/') => {
            format!("~{}", &cwd[home.len()..])
        }
        _ => cwd.to_owned(),
    };
    let compact = if compact.chars().count() > 52 {
        let parts: Vec<&str> = compact.rsplitn(3, '/').collect();
        if parts.len() == 3 {
            format!("…/{}/{}", parts[1], parts[0])
        } else {
            compact
        }
    } else {
        compact
    };
    sanitize_for_ps1(&compact)
}

fn sanitize_for_ps1(value: &str) -> String {
    let mut safe = String::with_capacity(value.len());
    for character in value.chars().take(256) {
        if character.is_control() || matches!(character, '$' | '`' | '\\') {
            safe.push('?');
        } else {
            safe.push(character);
        }
    }
    safe
}

#[derive(Debug, Default)]
struct GitStatus {
    branch: String,
    staged: usize,
    modified: usize,
    untracked: usize,
}

fn read_git_status(cwd: &Path) -> Option<GitStatus> {
    if !cwd.is_dir() {
        return None;
    }
    let output = Command::new("git")
        .args([
            "-c",
            "color.ui=false",
            "-C",
            cwd.to_str()?,
            "status",
            "--porcelain=v1",
            "--branch",
            "--untracked-files=normal",
        ])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.len() > 1024 * 1024 {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_git_status(&stdout)
}

fn parse_git_status(stdout: &str) -> Option<GitStatus> {
    let mut lines = stdout.lines();
    let header = lines.next()?.strip_prefix("## ")?;
    let header = header
        .strip_prefix("No commits yet on ")
        .or_else(|| header.strip_prefix("Initial commit on "))
        .unwrap_or(header);
    let branch = header
        .split_once("...")
        .map_or(header, |(branch, _)| branch)
        .trim()
        .to_owned();
    let mut status = GitStatus {
        branch: if branch.is_empty() {
            "detached".to_owned()
        } else {
            branch
        },
        ..GitStatus::default()
    };
    for line in lines {
        let bytes = line.as_bytes();
        if bytes.len() < 2 {
            continue;
        }
        if bytes[0] == b'?' && bytes[1] == b'?' {
            status.untracked += 1;
        } else {
            if bytes[0] != b' ' && bytes[0] != b'?' {
                status.staged += 1;
            }
            if bytes[1] != b' ' && bytes[1] != b'?' {
                status.modified += 1;
            }
        }
    }
    Some(status)
}

fn format_duration(duration_ms: u64) -> String {
    if duration_ms < 10_000 {
        format!("{:.1}s", duration_ms as f64 / 1_000.0)
    } else if duration_ms < 60_000 {
        format!("{}s", duration_ms / 1_000)
    } else {
        format!("{}m {}s", duration_ms / 60_000, (duration_ms / 1_000) % 60)
    }
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

fn trace_duration(event: &str, started: Instant) {
    if env::var("MBX_LOG").is_ok_and(|value| value == "trace") {
        eprintln!(
            "mbx trace event={event} elapsed_us={}",
            started.elapsed().as_micros()
        );
    }
}

fn trace_message(message: &str) {
    if env::var("MBX_LOG").is_ok_and(|value| value == "trace") {
        eprintln!("mbx trace {message}");
    }
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

    #[test]
    fn prompt_is_plain_when_color_and_icons_are_disabled() {
        let prompt = render_prompt(&PromptRequest {
            cwd: "/tmp/project".to_owned(),
            status: 2,
            duration_ms: Some(2_500),
            flags: FLAG_NO_COLOR | FLAG_ASCII_ICONS | FLAG_DISABLE_GIT,
        });
        assert_eq!(prompt, "/tmp/project  exit 2  2.5s\\n> ");
    }

    #[test]
    fn untrusted_prompt_content_cannot_add_prompt_expansions() {
        let safe = sanitize_for_ps1("bad$(touch /tmp/nope)`id`\\[\\e]0;title");
        assert!(!safe.contains('$'));
        assert!(!safe.contains('`'));
        assert!(!safe.contains('\\'));
        assert!(!safe.chars().any(char::is_control));
    }

    #[test]
    fn long_paths_are_compacted() {
        let path = "/a/very/long/path/that/is/definitely/longer/than/fifty/two/characters/project";
        assert_eq!(display_path(path), "…/characters/project");
    }

    #[test]
    fn unborn_git_branch_is_parsed_without_status_prose() {
        let status = parse_git_status("## No commits yet on main\n?? README.md\n").unwrap();
        assert_eq!(status.branch, "main");
        assert_eq!(status.untracked, 1);
    }
}
