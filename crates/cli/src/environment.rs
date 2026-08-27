use crate::cli::PromptDefaults;
use crate::prompt::RenderEnvironment;
use mbx_protocol::{
    FLAG_ASCII_ICONS, FLAG_COLOR_16, FLAG_DISABLE_GIT, FLAG_NERD_ICONS, FLAG_NO_COLOR,
    FLAG_PRODUCTION, FLAG_SSH, FLAG_TRUECOLOR, PromptFlags,
};
use std::env;
use std::io::{self, IsTerminal};

/// Immutable process state captured once at the composition boundary.
pub struct RuntimeEnvironment {
    prompt_flags: PromptFlags,
    pub render_environment: RenderEnvironment,
}

impl RuntimeEnvironment {
    pub fn capture() -> Self {
        Self {
            prompt_flags: prompt_flags(io::stdout().is_terminal()),
            render_environment: RenderEnvironment {
                home: env::var("HOME").ok(),
                host: env::var("HOSTNAME").ok(),
                user: env::var("USER").ok(),
            },
        }
    }

    pub fn prompt_defaults(&self) -> Result<PromptDefaults, String> {
        let cwd = env::current_dir()
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .into_owned();
        Ok(PromptDefaults {
            cwd,
            flags: self.prompt_flags,
        })
    }
}

fn color_disabled(stdout_is_terminal: bool) -> bool {
    env::var_os("NO_COLOR").is_some()
        || env::var("MBX_COLOR").is_ok_and(|value| value == "never")
        || env::var("TERM").is_ok_and(|value| value == "dumb")
        || !stdout_is_terminal
}

fn colorterm_is_truecolor(colorterm: &str) -> bool {
    matches!(
        colorterm.to_ascii_lowercase().as_str(),
        "truecolor" | "24bit"
    )
}

fn term_supports_256(term: &str) -> bool {
    term.contains("256color") || term == "xterm-direct"
}

fn apply_color_capability(flags: &mut PromptFlags, term: &str, colorterm: Option<&str>) {
    flags.remove(FLAG_COLOR_16 | FLAG_TRUECOLOR);
    if flags.no_color() {
        return;
    }
    if colorterm.is_some_and(colorterm_is_truecolor) {
        flags.insert(FLAG_TRUECOLOR);
    } else if !term_supports_256(term) {
        flags.insert(FLAG_COLOR_16);
    }
}

pub fn git_discovery_disabled() -> bool {
    env::var("MBX_DISABLE_GIT").is_ok_and(|value| value == "1")
}

pub fn color_disabled_for_stdout() -> bool {
    color_disabled(io::stdout().is_terminal())
}

fn prompt_flags(stdout_is_terminal: bool) -> PromptFlags {
    let mut flags = PromptFlags::empty();
    if color_disabled(stdout_is_terminal) {
        flags.insert(FLAG_NO_COLOR);
    }
    match env::var("MBX_ICONS").as_deref() {
        Ok("nerd") => flags.insert(FLAG_NERD_ICONS),
        Ok("never" | "ascii") => flags.insert(FLAG_ASCII_ICONS),
        _ => {}
    }
    if env::var_os("SSH_CONNECTION").is_some() || env::var_os("SSH_TTY").is_some() {
        flags.insert(FLAG_SSH);
    }
    if env::var("MBX_PRODUCTION_CONTEXT").is_ok_and(|value| value == "1") {
        flags.insert(FLAG_PRODUCTION);
    }
    if git_discovery_disabled() {
        flags.insert(FLAG_DISABLE_GIT);
    }
    let term = env::var("TERM").unwrap_or_default();
    let colorterm = env::var("COLORTERM").ok();
    apply_color_capability(&mut flags, &term, colorterm.as_deref());
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piped_stdout_disables_color_by_default() {
        assert!(color_disabled(false));
        assert!(prompt_flags(false).no_color());
    }

    #[test]
    fn tty_without_env_disable_allows_color() {
        if env::var_os("NO_COLOR").is_some()
            || env::var("MBX_COLOR").is_ok_and(|value| value == "never")
            || env::var("TERM").is_ok_and(|value| value == "dumb")
        {
            return;
        }
        assert!(!color_disabled(true));
        assert!(!prompt_flags(true).no_color());
    }

    #[test]
    fn color_disabled_matrix_matches_contract() {
        assert!(!color_should_be_disabled(true, false, false, false));
        assert!(color_should_be_disabled(false, false, false, false));
        assert!(color_should_be_disabled(true, true, false, false));
        assert!(color_should_be_disabled(true, false, true, false));
        assert!(color_should_be_disabled(true, false, false, true));
    }

    #[test]
    fn color_capability_matrix_matches_contract() {
        let mut flags = PromptFlags::empty();
        apply_color_capability(&mut flags, "xterm-256color", None);
        assert!(!flags.color_16());
        assert!(!flags.truecolor());

        flags = PromptFlags::empty();
        apply_color_capability(&mut flags, "xterm", Some("truecolor"));
        assert!(flags.truecolor());
        assert!(!flags.color_16());

        flags = PromptFlags::empty();
        apply_color_capability(&mut flags, "xterm", Some("24bit"));
        assert!(flags.truecolor());

        flags = PromptFlags::empty();
        apply_color_capability(&mut flags, "xterm", None);
        assert!(flags.color_16());
        assert!(!flags.truecolor());

        flags = PromptFlags::from_bits(FLAG_NO_COLOR);
        apply_color_capability(&mut flags, "xterm", Some("truecolor"));
        assert!(!flags.truecolor());
        assert!(!flags.color_16());
    }

    fn color_should_be_disabled(
        stdout_is_terminal: bool,
        no_color_env: bool,
        mbx_color_never: bool,
        term_dumb: bool,
    ) -> bool {
        no_color_env || mbx_color_never || term_dumb || !stdout_is_terminal
    }
}
