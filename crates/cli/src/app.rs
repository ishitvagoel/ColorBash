use crate::VERSION;
use crate::cli::{self, CliCommand, HighlightCommand, HistoryCommand, ServeTarget};
use crate::environment;
use crate::history::{HistoryControl, HistoryError, HistoryPolicy, HistorySearch};
use crate::prompt::PromptRendering;
use crate::provider::{
    GitRepositoryStatusProvider, NullRepositoryContextProvider, RepositoryContextProvider,
};
use crate::service::ProtocolService;
use crate::storage::{QueuedHistoryStore, default_store_path};
use crate::telemetry::trace_duration;
use crate::transport::{self, SocketClient};
use mbx_protocol::{Request, RequestKind, ResponseKind};
use std::io::{self, Write};
use std::num::NonZeroU64;
use std::time::Instant;

pub fn execute(command: CliCommand, renderer: &dyn PromptRendering) -> Result<(), String> {
    match command {
        CliCommand::Handshake => {
            println!("mbx/{VERSION} ready");
            Ok(())
        }
        CliCommand::Prompt(prompt) => {
            let started = Instant::now();
            print!("{}", renderer.render_prompt(&prompt));
            io::stdout().flush().map_err(|error| error.to_string())?;
            trace_duration("prompt_render", started);
            Ok(())
        }
        CliCommand::Serve(target) => {
            let service = ProtocolService::new(renderer);
            let policy = crate::policy::EnvironmentHistoryPolicy::from_environment();
            let history_handler: Option<Box<dyn crate::history_service::HistoryHandler>> = if policy
                .disabled()
            {
                None
            } else {
                let store =
                    std::sync::Arc::new(open_capture_store().map_err(|error| error.to_string())?);
                Some(Box::new(crate::history_service::HistoryService::new(
                    Box::new(std::sync::Arc::clone(&store)),
                    Box::new(std::sync::Arc::clone(&store)),
                    Box::new(policy),
                )))
            };
            // Highlighting has no privacy or storage contract and does not
            // require MBX_HISTORY=1 (ADR 0014), so the handler is always
            // present; Bash decides whether to ever send a HIGHLIGHT frame.
            let highlight_handler: Option<Box<dyn crate::highlight_service::HighlightHandler>> =
                Some(Box::new(crate::highlight_service::HighlightService));
            match target {
                ServeTarget::Stdio => {
                    transport::serve_stdio(&service, history_handler, highlight_handler)
                }
                ServeTarget::Socket(path) => {
                    transport::serve_socket(&path, &service, history_handler, highlight_handler)
                }
            }
        }
        CliCommand::SocketPing(path) => socket_ping(&path),
        CliCommand::BenchmarkClient { socket, iterations } => benchmark_client(&socket, iterations),
        CliCommand::History(history) => execute_history(history),
        CliCommand::Highlight(highlight) => execute_highlight(highlight),
        CliCommand::Version => {
            println!("mbx {VERSION}");
            Ok(())
        }
        CliCommand::Help => {
            println!("{}", cli::help_text(VERSION));
            Ok(())
        }
    }
}

fn execute_history(command: HistoryCommand) -> Result<(), String> {
    let policy = crate::policy::EnvironmentHistoryPolicy::from_environment();
    if policy.disabled() {
        return Err(crate::history::HistoryError::new(
            crate::history::HistoryErrorKind::Disabled,
            format!(
                "history capture is disabled; set {}=1 to enable it ({} holds the exclusions)",
                crate::policy::EnvironmentHistoryPolicy::disabled_var(),
                crate::policy::EnvironmentHistoryPolicy::exclude_var(),
            ),
        )
        .to_string());
    }
    match command {
        HistoryCommand::Path => {
            println!("{}", default_store_path().display());
            Ok(())
        }
        HistoryCommand::Count => {
            let store = open_history_store()?;
            println!("{}", store.count().map_err(|error| error.to_string())?);
            Ok(())
        }
        HistoryCommand::Clear => {
            let store = open_history_store()?;
            store.clear().map_err(|error| error.to_string())
        }
        HistoryCommand::Delete => {
            let store = open_history_store()?;
            store.delete().map_err(|error| error.to_string())
        }
        HistoryCommand::SearchRecent { limit } => {
            let store = open_history_store()?;
            print_entries(store.recent(limit).map_err(|error| error.to_string())?)
        }
        HistoryCommand::SearchPrefix { prefix, cwd, limit } => {
            let store = open_history_store()?;
            let entries = match cwd {
                Some(cwd) => store
                    .exact_prefix_in_cwd(&prefix, &cwd, limit)
                    .map_err(|error| error.to_string())?,
                None => store
                    .exact_prefix(&prefix, limit)
                    .map_err(|error| error.to_string())?,
            };
            print_entries(entries)
        }
        HistoryCommand::SearchCwd { cwd, limit } => {
            let store = open_history_store()?;
            print_entries(
                store
                    .by_cwd(&cwd, limit)
                    .map_err(|error| error.to_string())?,
            )
        }
        HistoryCommand::SearchRepo { repo_root, limit } => {
            let store = open_history_store()?;
            print_entries(
                store
                    .by_repo(&repo_root, limit)
                    .map_err(|error| error.to_string())?,
            )
        }
        HistoryCommand::SearchBranch { repo_branch, limit } => {
            let store = open_history_store()?;
            print_entries(
                store
                    .by_branch(&repo_branch, limit)
                    .map_err(|error| error.to_string())?,
            )
        }
        HistoryCommand::SearchFuzzy { needle, cwd, limit } => {
            let store = open_history_store()?;
            let entries = match cwd {
                Some(cwd) => store
                    .fuzzy_in_cwd(&needle, &cwd, limit)
                    .map_err(|error| error.to_string())?,
                None => store
                    .fuzzy(&needle, limit)
                    .map_err(|error| error.to_string())?,
            };
            print_entries(entries)
        }
        HistoryCommand::SearchFailed { limit } => {
            let store = open_history_store()?;
            print_entries(store.failed(limit).map_err(|error| error.to_string())?)
        }
    }
}

fn execute_highlight(command: HighlightCommand) -> Result<(), String> {
    match command {
        HighlightCommand::Line {
            text,
            point,
            no_color,
            color,
        } => {
            let color = match color {
                Some(explicit) => explicit,
                None => !no_color && !environment::color_disabled_for_stdout(),
            };
            let rendered = crate::highlight::highlight_line(&text, point, color)
                .ok_or_else(|| "highlight input was rejected".to_owned())?;
            println!("{}", rendered.0);
            println!("{}", rendered.1);
            Ok(())
        }
    }
}

fn open_capture_store() -> Result<QueuedHistoryStore, HistoryError> {
    let context: Box<dyn RepositoryContextProvider> = if environment::git_discovery_disabled() {
        Box::new(NullRepositoryContextProvider)
    } else {
        Box::new(GitRepositoryStatusProvider::default())
    };
    QueuedHistoryStore::open_with_context(
        &default_store_path(),
        crate::history::DEFAULT_QUEUE_CAPACITY,
        context,
    )
}

fn open_history_store() -> Result<QueuedHistoryStore, String> {
    QueuedHistoryStore::open_default(crate::history::DEFAULT_QUEUE_CAPACITY)
        .map_err(|error| error.to_string())
}

fn print_entries(entries: Vec<crate::history::HistoryEntry>) -> Result<(), String> {
    for entry in entries {
        println!("{}", entry.command_text);
    }
    Ok(())
}

fn socket_ping(path: &std::path::Path) -> Result<(), String> {
    let mut client = SocketClient::connect(path)?;
    let request = Request {
        id: 1,
        kind: RequestKind::Ping,
    };
    let response = client.exchange(&request)?;
    if response.kind != ResponseKind::Pong {
        return Err("socket server returned an unexpected response".to_owned());
    }
    println!("mbx/{VERSION} socket ready");
    Ok(())
}

fn benchmark_client(path: &std::path::Path, iterations: NonZeroU64) -> Result<(), String> {
    let mut client = SocketClient::connect(path)?;
    let started = Instant::now();
    for id in 1..=iterations.get() {
        let request = Request {
            id,
            kind: RequestKind::Ping,
        };
        let response = client.exchange(&request)?;
        if response.kind != ResponseKind::Pong {
            return Err(format!("unexpected response for request {id}"));
        }
    }
    let elapsed = started.elapsed();
    println!(
        "transport=unix-socket iterations={iterations} total_ns={} mean_ns={}",
        elapsed.as_nanos(),
        elapsed.as_nanos() / u128::from(iterations.get())
    );
    Ok(())
}
