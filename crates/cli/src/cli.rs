use crate::prompt::PromptContext;
use mbx_protocol::{
    PromptFlags, FLAG_ASCII_ICONS, FLAG_DISABLE_GIT, FLAG_NERD_ICONS, FLAG_NO_COLOR,
    FLAG_PRODUCTION, FLAG_SSH,
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
pub enum HistoryCommand {
    Path,
    Count,
    Clear,
    Delete,
    SearchRecent {
        limit: usize,
    },
    SearchPrefix {
        prefix: String,
        cwd: Option<String>,
        limit: usize,
    },
    SearchCwd {
        cwd: String,
        limit: usize,
    },
    SearchRepo {
        repo_root: String,
        limit: usize,
    },
    SearchBranch {
        repo_branch: String,
        limit: usize,
    },
    SearchFuzzy {
        needle: String,
        cwd: Option<String>,
        limit: usize,
    },
    SearchFailed {
        limit: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HighlightCommand {
    Line {
        text: String,
        point: usize,
        no_color: bool,
        /// An explicit color decision from a caller that already knows the
        /// terminal capability (Bash, via `_mbx_highlight_color_flag`). When
        /// present it wins over `no_color` and over this process's own
        /// stdout, which is meaningless here: direct CLI use aside, both the
        /// coprocess and the process-substitution spawn path never have the
        /// interactive shell's terminal on their own stdout (M-062).
        color: Option<bool>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepoCommand {
    Root { cwd: Option<PathBuf> },
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
    History(HistoryCommand),
    Highlight(HighlightCommand),
    Repo(RepoCommand),
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
        Some("history") => parse_history(&args[1..]).map(CliCommand::History),
        Some("highlight") => parse_highlight(&args[1..]).map(CliCommand::Highlight),
        Some("repo") => parse_repo(&args[1..]).map(CliCommand::Repo),
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
         mbx benchmark-client --socket PATH [--iterations N]\n  \
         mbx history (path|count|clear|delete)\n  \
         mbx history search recent [--limit N]\n  \
         mbx history search prefix TEXT [--cwd PATH] [--limit N]\n  \
         mbx history search cwd PATH [--limit N]\n  \
         mbx history search repo ROOT [--limit N]\n  \
         mbx history search branch NAME [--limit N]\n  \
         mbx history search fuzzy TEXT [--cwd PATH] [--limit N]\n  \
         mbx history search failed [--limit N]\n  \
         mbx highlight TEXT [--point N] [--no-color] [--color 0|1]\n  \
         mbx repo root [--cwd PATH]\n\n\
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

fn parse_highlight(args: &[String]) -> Result<HighlightCommand, String> {
    let text = args.first().cloned().ok_or("highlight requires TEXT")?;
    let mut point = text.chars().count();
    let mut no_color = false;
    let mut color = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--point" => {
                index += 1;
                point = args
                    .get(index)
                    .ok_or("--point requires a value")?
                    .parse::<usize>()
                    .map_err(|_| "--point must be an unsigned integer")?;
            }
            "--no-color" => no_color = true,
            "--color" => {
                index += 1;
                color = Some(match args.get(index).map(String::as_str) {
                    Some("0") => false,
                    Some("1") => true,
                    _ => return Err("--color requires 0 or 1".to_owned()),
                });
            }
            unknown => return Err(format!("unknown highlight option: {unknown}")),
        }
        index += 1;
    }
    if no_color && color == Some(true) {
        return Err("--no-color conflicts with --color 1".to_owned());
    }
    Ok(HighlightCommand::Line {
        text,
        point,
        no_color,
        color,
    })
}

fn parse_repo(args: &[String]) -> Result<RepoCommand, String> {
    match args.first().map(String::as_str) {
        Some("root") => {
            let mut cwd = None;
            let mut index = 1;
            while index < args.len() {
                match args[index].as_str() {
                    "--cwd" => {
                        index += 1;
                        let value = args.get(index).ok_or("--cwd requires a value")?;
                        cwd = Some(PathBuf::from(value));
                    }
                    unknown => return Err(format!("unknown repo root option: {unknown}")),
                }
                index += 1;
            }
            Ok(RepoCommand::Root { cwd })
        }
        Some(unknown) => Err(format!("unknown repo subcommand: {unknown}")),
        None => Err("repo requires a subcommand (root)".to_owned()),
    }
}

fn parse_history(args: &[String]) -> Result<HistoryCommand, String> {
    match args.first().map(String::as_str) {
        Some("path") => expect_no_history_args(&args[1..], HistoryCommand::Path),
        Some("count") => expect_no_history_args(&args[1..], HistoryCommand::Count),
        Some("clear") => expect_no_history_args(&args[1..], HistoryCommand::Clear),
        Some("delete") => expect_no_history_args(&args[1..], HistoryCommand::Delete),
        Some("search") => parse_history_search(&args[1..]),
        Some(command) => Err(format!("unknown history command: {command}")),
        None => Err("history requires a subcommand (path|count|clear|delete|search)".to_owned()),
    }
}

fn expect_no_history_args(
    args: &[String],
    command: HistoryCommand,
) -> Result<HistoryCommand, String> {
    if args.is_empty() {
        Ok(command)
    } else {
        Err(format!("unexpected arguments: {}", args.join(" ")))
    }
}

fn parse_history_search(args: &[String]) -> Result<HistoryCommand, String> {
    let mut limit = crate::history::DEFAULT_QUERY_LIMIT;
    let mut cwd = None;
    let kind;
    let mut index;
    match args.first().map(String::as_str) {
        Some("recent") => {
            kind = HistorySearchKind::Recent;
            index = 1;
        }
        Some("prefix") => {
            index = 2;
            kind = HistorySearchKind::Prefix(
                args.get(1).cloned().ok_or("search prefix requires TEXT")?,
            );
        }
        Some("cwd") => {
            index = 2;
            kind = HistorySearchKind::Cwd(args.get(1).cloned().ok_or("search cwd requires PATH")?);
        }
        Some("repo") => {
            index = 2;
            kind =
                HistorySearchKind::Repo(args.get(1).cloned().ok_or("search repo requires ROOT")?);
        }
        Some("branch") => {
            index = 2;
            kind = HistorySearchKind::Branch(
                args.get(1).cloned().ok_or("search branch requires NAME")?,
            );
        }
        Some("fuzzy") => {
            index = 2;
            kind =
                HistorySearchKind::Fuzzy(args.get(1).cloned().ok_or("search fuzzy requires TEXT")?);
        }
        Some("failed") => {
            kind = HistorySearchKind::Failed;
            index = 1;
        }
        Some(command) => return Err(format!("unknown search kind: {command}")),
        None => {
            return Err(
                "history search requires (recent|prefix|cwd|repo|branch|fuzzy|failed)".to_owned(),
            );
        }
    }
    while index < args.len() {
        match args[index].as_str() {
            "--limit" => {
                index += 1;
                limit = args
                    .get(index)
                    .ok_or("--limit requires a value")?
                    .parse::<usize>()
                    .map_err(|_| "--limit must be an unsigned integer")?;
                if limit > crate::history::MAX_QUERY_LIMIT {
                    return Err(format!(
                        "--limit must be at most {}",
                        crate::history::MAX_QUERY_LIMIT
                    ));
                }
            }
            "--cwd" => {
                if !matches!(
                    kind,
                    HistorySearchKind::Prefix(_) | HistorySearchKind::Fuzzy(_)
                ) {
                    return Err("--cwd is only valid for prefix and fuzzy search".to_owned());
                }
                index += 1;
                let value = args.get(index).ok_or("--cwd requires a value")?;
                if value.is_empty() {
                    return Err("--cwd requires a value".to_owned());
                }
                cwd = Some(value.clone());
            }
            unknown => return Err(format!("unknown search option: {unknown}")),
        }
        index += 1;
    }
    Ok(match kind {
        HistorySearchKind::Recent => HistoryCommand::SearchRecent { limit },
        HistorySearchKind::Prefix(prefix) => HistoryCommand::SearchPrefix { prefix, cwd, limit },
        HistorySearchKind::Cwd(path) => HistoryCommand::SearchCwd { cwd: path, limit },
        HistorySearchKind::Repo(repo_root) => HistoryCommand::SearchRepo { repo_root, limit },
        HistorySearchKind::Branch(repo_branch) => {
            HistoryCommand::SearchBranch { repo_branch, limit }
        }
        HistorySearchKind::Fuzzy(needle) => HistoryCommand::SearchFuzzy { needle, cwd, limit },
        HistorySearchKind::Failed => HistoryCommand::SearchFailed { limit },
    })
}

enum HistorySearchKind {
    Recent,
    Prefix(String),
    Cwd(String),
    Repo(String),
    Branch(String),
    Fuzzy(String),
    Failed,
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
    fn handshake_does_not_resolve_prompt_defaults() {
        let command = parse(&args(&["handshake"]), || {
            panic!("prompt defaults must not be resolved for handshake")
        })
        .unwrap();
        assert_eq!(command, CliCommand::Handshake);
    }

    #[test]
    fn raw_flags_replace_piped_default_no_color() {
        let command = parse(&args(&["prompt", "--flags", "34"]), || {
            Ok(PromptDefaults {
                cwd: "/tmp".to_owned(),
                flags: PromptFlags::from_bits(FLAG_NO_COLOR),
            })
        })
        .unwrap();
        let CliCommand::Prompt(prompt) = command else {
            panic!("expected prompt command");
        };
        assert!(!prompt.flags.no_color());
        assert!(prompt.flags.ascii_icons());
        assert!(prompt.flags.git_disabled());
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
        let fuzzy = parse(
            &args(&["history", "search", "fuzzy", "git", "--limit", "3"]),
            || panic!("history must not resolve prompt defaults"),
        )
        .unwrap();
        assert_eq!(
            fuzzy,
            CliCommand::History(HistoryCommand::SearchFuzzy {
                needle: "git".to_owned(),
                cwd: None,
                limit: 3,
            })
        );
        let prefix_cwd = parse(
            &args(&[
                "history", "search", "prefix", "echo", "--cwd", "/work", "--limit", "4",
            ]),
            || panic!("history must not resolve prompt defaults"),
        )
        .unwrap();
        assert_eq!(
            prefix_cwd,
            CliCommand::History(HistoryCommand::SearchPrefix {
                prefix: "echo".to_owned(),
                cwd: Some("/work".to_owned()),
                limit: 4,
            })
        );
        assert_eq!(
            parse(
                &args(&["history", "search", "recent", "--cwd", "/work"]),
                || panic!("history must not resolve prompt defaults"),
            )
            .unwrap_err(),
            "--cwd is only valid for prefix and fuzzy search"
        );
        let repo = parse(
            &args(&["history", "search", "repo", "/workspace", "--limit", "2"]),
            || panic!("history must not resolve prompt defaults"),
        )
        .unwrap();
        assert_eq!(
            repo,
            CliCommand::History(HistoryCommand::SearchRepo {
                repo_root: "/workspace".to_owned(),
                limit: 2,
            })
        );
        let branch = parse(
            &args(&["history", "search", "branch", "hist-branch", "--limit", "2"]),
            || panic!("history must not resolve prompt defaults"),
        )
        .unwrap();
        assert_eq!(
            branch,
            CliCommand::History(HistoryCommand::SearchBranch {
                repo_branch: "hist-branch".to_owned(),
                limit: 2,
            })
        );
        assert_eq!(
            parse(&args(&["history", "search", "branch"]), || panic!(
                "history must not resolve prompt defaults"
            ),)
            .unwrap_err(),
            "search branch requires NAME"
        );
        let failed = parse(
            &args(&["history", "search", "failed", "--limit", "3"]),
            || panic!("history must not resolve prompt defaults"),
        )
        .unwrap();
        assert_eq!(
            failed,
            CliCommand::History(HistoryCommand::SearchFailed { limit: 3 })
        );
        assert_eq!(
            parse(&args(&["history", "search", "failed", "git"]), || panic!(
                "history must not resolve prompt defaults"
            ),)
            .unwrap_err(),
            "unknown search option: git"
        );
    }

    #[test]
    fn repo_root_parses_with_and_without_cwd() {
        let command = parse(&args(&["repo", "root"]), || {
            panic!("repo must not resolve prompt defaults")
        })
        .unwrap();
        assert_eq!(command, CliCommand::Repo(RepoCommand::Root { cwd: None }));

        let command = parse(&args(&["repo", "root", "--cwd", "/work"]), || {
            panic!("repo must not resolve prompt defaults")
        })
        .unwrap();
        assert_eq!(
            command,
            CliCommand::Repo(RepoCommand::Root {
                cwd: Some(PathBuf::from("/work"))
            })
        );
    }

    #[test]
    fn repo_root_rejects_unknown_option_and_missing_cwd_value() {
        assert_eq!(
            parse(&args(&["repo", "root", "--bogus"]), || panic!(
                "repo must not resolve prompt defaults"
            ))
            .unwrap_err(),
            "unknown repo root option: --bogus"
        );
        assert_eq!(
            parse(&args(&["repo", "root", "--cwd"]), || panic!(
                "repo must not resolve prompt defaults"
            ))
            .unwrap_err(),
            "--cwd requires a value"
        );
        assert_eq!(
            parse(&args(&["repo"]), || panic!(
                "repo must not resolve prompt defaults"
            ))
            .unwrap_err(),
            "repo requires a subcommand (root)"
        );
        assert_eq!(
            parse(&args(&["repo", "branches"]), || panic!(
                "repo must not resolve prompt defaults"
            ))
            .unwrap_err(),
            "unknown repo subcommand: branches"
        );
    }
}
