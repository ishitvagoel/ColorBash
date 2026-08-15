use crate::prompt::PromptContext;
use mbx_protocol::{
    FLAG_ASCII_ICONS, FLAG_DISABLE_GIT, FLAG_NERD_ICONS, FLAG_NO_COLOR, FLAG_PRODUCTION, FLAG_SSH,
    PromptFlags,
};
use std::num::NonZeroU64;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptDefaults {
    pub cwd: String,
    pub flags: PromptFlags,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServeTarget {
    Stdio,
    Socket(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliCommand {
    Handshake,
    Prompt(PromptContext),
    Serve(ServeTarget),
    SocketPing(PathBuf),
    BenchmarkClient {
        socket: PathBuf,
        iterations: NonZeroU64,
    },
    Version,
    Help,
}

/// Parses process arguments and resolves prompt-only defaults through an
/// injected, lazily invoked boundary.
pub fn parse(
    args: &[String],
    prompt_defaults: impl FnOnce() -> Result<PromptDefaults, String>,
) -> Result<CliCommand, String> {
    match args.first().map(String::as_str) {
        Some("handshake") => Ok(CliCommand::Handshake),
        Some("prompt") => parse_prompt(&args[1..], prompt_defaults()?).map(CliCommand::Prompt),
        Some("serve") => parse_serve(&args[1..]).map(CliCommand::Serve),
        Some("socket-ping") => parse_socket_path(&args[1..]).map(CliCommand::SocketPing),
        Some("benchmark-client") => parse_benchmark(&args[1..]),
        Some("--version" | "-V") => Ok(CliCommand::Version),
        Some("--help" | "-h") | None => Ok(CliCommand::Help),
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

pub fn help_text(version: &str) -> String {
    format!(
        "mbx {version}\n\n\
         Bash-compatible terminal UX foundation prototype\n\n\
         USAGE:\n  mbx handshake\n  mbx prompt [OPTIONS]\n  mbx serve --stdio\n  \
         mbx serve --socket PATH\n  mbx socket-ping --socket PATH\n  \
         mbx benchmark-client --socket PATH [--iterations N]\n\n\
         PROMPT OPTIONS:\n  --cwd PATH  --status N  --duration-ms N  --flags BITS\n  \
         --no-color  --ascii  --nerd-font  --ssh  --production  --disable-git"
    )
}

fn parse_prompt(args: &[String], defaults: PromptDefaults) -> Result<PromptContext, String> {
    let mut prompt = PromptContext {
        cwd: defaults.cwd,
        status: 0,
        duration_ms: None,
        flags: defaults.flags,
    };
    let mut flags = defaults.flags;
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
            "--flags" => {
                flags = PromptFlags::from_bits(
                    next_value(args, &mut index, "--flags")?
                        .parse::<u32>()
                        .map_err(|_| "--flags must be an unsigned 32-bit integer".to_owned())?,
                );
            }
            "--no-color" => flags.insert(FLAG_NO_COLOR),
            "--ascii" => {
                flags.insert(FLAG_ASCII_ICONS);
                flags.remove(FLAG_NERD_ICONS);
            }
            "--nerd-font" => {
                flags.insert(FLAG_NERD_ICONS);
                flags.remove(FLAG_ASCII_ICONS);
            }
            "--ssh" => flags.insert(FLAG_SSH),
            "--production" => flags.insert(FLAG_PRODUCTION),
            "--disable-git" => flags.insert(FLAG_DISABLE_GIT),
            unknown => return Err(format!("unknown prompt option: {unknown}")),
        }
        index += 1;
    }
    prompt.flags = flags;
    Ok(prompt)
}

fn parse_serve(args: &[String]) -> Result<ServeTarget, String> {
    match args {
        [mode] if mode == "--stdio" => Ok(ServeTarget::Stdio),
        [mode, socket] if mode == "--socket" => Ok(ServeTarget::Socket(PathBuf::from(socket))),
        _ => Err("serve requires exactly --stdio or --socket PATH".to_owned()),
    }
}

fn parse_socket_path(args: &[String]) -> Result<PathBuf, String> {
    match args {
        [option, socket] if option == "--socket" => Ok(PathBuf::from(socket)),
        _ => Err("--socket PATH is required".to_owned()),
    }
}

fn parse_benchmark(args: &[String]) -> Result<CliCommand, String> {
    let mut socket = None;
    let mut iterations = NonZeroU64::new(1_000).expect("the default iteration count is non-zero");
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--socket" => {
                socket = Some(PathBuf::from(next_value(args, &mut index, "--socket")?));
            }
            "--iterations" => {
                let value = next_value(args, &mut index, "--iterations")?
                    .parse::<u64>()
                    .map_err(|_| "--iterations must be an unsigned integer".to_owned())?;
                iterations = NonZeroU64::new(value)
                    .ok_or_else(|| "--iterations must be greater than zero".to_owned())?;
            }
            unknown => return Err(format!("unknown benchmark option: {unknown}")),
        }
        index += 1;
    }
    let socket = socket.ok_or_else(|| "--socket is required".to_owned())?;
    Ok(CliCommand::BenchmarkClient { socket, iterations })
}

fn next_value<'a>(args: &'a [String], index: &mut usize, option: &str) -> Result<&'a str, String> {
    *index += 1;
    args.get(*index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> PromptDefaults {
        PromptDefaults {
            cwd: "/default".to_owned(),
            flags: PromptFlags::empty(),
        }
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn no_command_selects_help() {
        let command = parse(&[], || panic!("help must not resolve prompt defaults")).unwrap();
        assert_eq!(command, CliCommand::Help);
    }

    #[test]
    fn prompt_options_override_defaults() {
        let command = parse(
            &args(&[
                "prompt",
                "--cwd",
                "/work",
                "--status",
                "127",
                "--duration-ms",
                "2500",
                "--ssh",
                "--disable-git",
            ]),
            || Ok(defaults()),
        )
        .unwrap();
        let CliCommand::Prompt(prompt) = command else {
            panic!("expected prompt command");
        };
        assert_eq!(prompt.cwd, "/work");
        assert_eq!(prompt.status, 127);
        assert_eq!(prompt.duration_ms, Some(2_500));
        let flags = prompt.flags;
        assert!(flags.ssh());
        assert!(flags.git_disabled());
    }

    #[test]
    fn last_icon_option_wins() {
        let command = parse(&args(&["prompt", "--nerd-font", "--ascii"]), || {
            Ok(defaults())
        })
        .unwrap();
        let CliCommand::Prompt(prompt) = command else {
            panic!("expected prompt command");
        };
        let flags = prompt.flags;
        assert!(flags.ascii_icons());
        assert!(!flags.nerd_icons());
    }

    #[test]
    fn raw_flags_replace_defaults_and_named_options_mutate_them() {
        const UNKNOWN_FLAG: u32 = 1 << 31;
        let command = parse(&args(&["prompt", "--flags", "2147483648", "--ssh"]), || {
            Ok(PromptDefaults {
                cwd: "/default".to_owned(),
                flags: PromptFlags::from_bits(FLAG_NO_COLOR),
            })
        })
        .unwrap();
        let CliCommand::Prompt(prompt) = command else {
            panic!("expected prompt command");
        };
        assert_eq!(prompt.flags.bits(), UNKNOWN_FLAG | FLAG_SSH);
    }

    #[test]
    fn missing_and_invalid_values_are_rejected() {
        assert_eq!(
            parse(&args(&["prompt", "--cwd"]), || Ok(defaults())).unwrap_err(),
            "--cwd requires a value"
        );
        assert_eq!(
            parse(&args(&["prompt", "--status", "256"]), || Ok(defaults())).unwrap_err(),
            "--status must be between 0 and 255"
        );
        assert_eq!(
            parse(&args(&["prompt", "--flags", "4294967296"]), || {
                Ok(defaults())
            })
            .unwrap_err(),
            "--flags must be an unsigned 32-bit integer"
        );
        assert_eq!(
            parse(
                &args(&[
                    "benchmark-client",
                    "--socket",
                    "/tmp/x",
                    "--iterations",
                    "0"
                ]),
                || Ok(defaults()),
            )
            .unwrap_err(),
            "--iterations must be greater than zero"
        );
    }
}
