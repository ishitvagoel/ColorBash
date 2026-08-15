use crate::VERSION;
use crate::cli::{self, CliCommand, ServeTarget};
use crate::prompt::PromptRendering;
use crate::service::ProtocolService;
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
            match target {
                ServeTarget::Stdio => transport::serve_stdio(&service),
                ServeTarget::Socket(path) => transport::serve_socket(&path, &service),
            }
        }
        CliCommand::SocketPing(path) => socket_ping(&path),
        CliCommand::BenchmarkClient { socket, iterations } => benchmark_client(&socket, iterations),
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
