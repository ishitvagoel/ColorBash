use crate::cli::PromptDefaults;
use crate::prompt::RenderEnvironment;
use mbx_protocol::{
    FLAG_ASCII_ICONS, FLAG_DISABLE_GIT, FLAG_NERD_ICONS, FLAG_NO_COLOR, FLAG_PRODUCTION, FLAG_SSH,
    PromptFlags,
};
use std::env;

/// Immutable process state captured once at the composition boundary.
pub struct RuntimeEnvironment {
    prompt_flags: PromptFlags,
    pub render_environment: RenderEnvironment,
}

impl RuntimeEnvironment {
    pub fn capture() -> Self {
        Self {
            prompt_flags: prompt_flags(),
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

fn prompt_flags() -> PromptFlags {
    let mut flags = PromptFlags::empty();
    if env::var_os("NO_COLOR").is_some()
        || env::var("MBX_COLOR").is_ok_and(|value| value == "never")
        || env::var("TERM").is_ok_and(|value| value == "dumb")
    {
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
    if env::var("MBX_DISABLE_GIT").is_ok_and(|value| value == "1") {
        flags.insert(FLAG_DISABLE_GIT);
    }
    flags
}
