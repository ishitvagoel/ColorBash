use crate::history::{HistoryEntry, HistoryPolicy};

const DISABLE_VAR: &str = "MBX_HISTORY";
const EXCLUDE_VAR: &str = "MBX_HISTORY_EXCLUDE";

pub struct EnvironmentHistoryPolicy {
    disabled: bool,
    exclusions: Vec<String>,
}

impl EnvironmentHistoryPolicy {
    pub fn from_environment() -> Self {
        let history_setting = std::env::var(DISABLE_VAR).ok();
        let disabled = history_disabled(history_setting.as_deref());
        let exclusions = std::env::var(EXCLUDE_VAR)
            .ok()
            .map(|value| {
                value
                    .split(':')
                    .map(|pattern| pattern.to_owned())
                    .filter(|pattern| !pattern.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            disabled,
            exclusions,
        }
    }

    pub fn disabled_var() -> &'static str {
        DISABLE_VAR
    }

    pub fn exclude_var() -> &'static str {
        EXCLUDE_VAR
    }
}

fn history_disabled(value: Option<&str>) -> bool {
    value != Some("1")
}

impl HistoryPolicy for EnvironmentHistoryPolicy {
    fn disabled(&self) -> bool {
        self.disabled
    }

    fn allows(&self, entry: &HistoryEntry) -> bool {
        !self
            .exclusions
            .iter()
            .any(|pattern| glob_match(pattern, &entry.command_text))
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    match_here(&pattern, &text)
}

fn match_here(pattern: &[char], text: &[char]) -> bool {
    match pattern.first() {
        None => text.is_empty(),
        Some('*') => {
            for split in 0..=text.len() {
                if match_here(&pattern[1..], &text[split..]) {
                    return true;
                }
            }
            false
        }
        Some('?') => !text.is_empty() && match_here(&pattern[1..], &text[1..]),
        Some('[') => match_bracket(pattern, text),
        Some(character) => {
            !text.is_empty() && *character == text[0] && match_here(&pattern[1..], &text[1..])
        }
    }
}

fn match_bracket(pattern: &[char], text: &[char]) -> bool {
    if text.is_empty() {
        return false;
    }
    let (end, negated) = match pattern.get(1) {
        Some('!') => (2, true),
        Some('^') => (2, true),
        _ => (1, false),
    };
    let mut index = end;
    let mut matched = false;
    while index < pattern.len() {
        let lower = pattern[index];
        if lower == ']' {
            break;
        }
        if index + 2 < pattern.len() && pattern[index + 1] == '-' && pattern[index + 2] != ']' {
            let upper = pattern[index + 2];
            matched = matched || (lower..=upper).contains(&text[0]);
            index += 3;
        } else {
            matched = matched || lower == text[0];
            index += 1;
        }
    }
    while index < pattern.len() && pattern[index] != ']' {
        index += 1;
    }
    if index >= pattern.len() {
        return false;
    }
    if matched == negated {
        return false;
    }
    match_here(&pattern[index + 1..], &text[1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(command: &str) -> HistoryEntry {
        HistoryEntry {
            session_id: "s".to_owned(),
            event_sequence: 1,
            history_number: Some(1),
            command_text: command.to_owned(),
            start_cwd: "/w".to_owned(),
            completed_at: "2026-08-15T10:00:00Z".to_owned(),
            status: 0,
            duration_ms: None,
            host: "h".to_owned(),
            user: "u".to_owned(),
        }
    }

    #[test]
    fn glob_matches_wildcards_and_brackets() {
        assert!(glob_match("git *", "git status"));
        assert!(!glob_match("git *", "svn status"));
        assert!(glob_match("git st*", "git status"));
        assert!(glob_match("git s?atus", "git status"));
        assert!(!glob_match("git s?tus", "git status"));
        assert!(glob_match("echo [abc]x", "echo ax"));
        assert!(!glob_match("echo [abc]x", "echo dx"));
        assert!(glob_match("echo [!abc]x", "echo dx"));
        assert!(glob_match("echo [a-c]x", "echo bx"));
        assert!(glob_match("*secret*", "docker run --secret xyz"));
    }

    #[test]
    fn policy_applies_disable_and_exclusions() {
        let policy = EnvironmentHistoryPolicy {
            disabled: true,
            exclusions: vec!["git commit *".to_owned()],
        };
        assert!(policy.disabled());
        assert!(!policy.allows(&entry("git commit -m hi")));
        assert!(policy.allows(&entry("git status")));
    }

    #[test]
    fn policy_requires_explicit_opt_in() {
        assert!(history_disabled(None));
        assert!(history_disabled(Some("0")));
        assert!(history_disabled(Some("yes")));
        assert!(!history_disabled(Some("1")));
    }

    #[test]
    fn environment_policy_reads_vars() {
        unsafe {
            std::env::set_var("MBX_HISTORY", "0");
            std::env::set_var("MBX_HISTORY_EXCLUDE", "rm *:ssh *");
        }
        let policy = EnvironmentHistoryPolicy::from_environment();
        assert!(policy.disabled());
        assert!(!policy.allows(&entry("rm -rf /tmp")));
        assert!(!policy.allows(&entry("ssh host")));
        assert!(policy.allows(&entry("ls")));
        unsafe {
            std::env::remove_var("MBX_HISTORY");
            std::env::remove_var("MBX_HISTORY_EXCLUDE");
        }
    }
}
