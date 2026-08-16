use crate::provider::{ProviderError, RepositoryStatus, RepositoryStatusProvider};
use crate::telemetry::trace_message;
use mbx_protocol::PromptFlags;
use std::path::Path;
use unicode_width::UnicodeWidthChar;

/// Transport-independent context required to construct one prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptContext {
    pub cwd: String,
    pub status: u8,
    pub duration_ms: Option<u64>,
    pub flags: PromptFlags,
}

/// Narrow application port implemented by terminal-specific prompt renderers.
pub trait PromptRendering {
    fn render_prompt(&self, context: &PromptContext) -> String;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RenderEnvironment {
    pub home: Option<String>,
    pub host: Option<String>,
    pub user: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticRole {
    Primary,
    Path,
    RepositoryClean,
    RepositoryDirty,
    Warning,
    Danger,
    Error,
    Muted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptSegment {
    text: String,
    role: SemanticRole,
}

impl PromptSegment {
    pub fn new(text: impl Into<String>, role: SemanticRole) -> Self {
        Self {
            text: text.into(),
            role,
        }
    }
}

pub struct SegmentContext<'a> {
    pub prompt: &'a PromptContext,
    pub environment: &'a RenderEnvironment,
    pub nerd_icons: bool,
}

/// Extension point for independently testable prompt capabilities.
pub trait PromptSegmentProvider {
    fn segment(&self, context: &SegmentContext<'_>) -> Option<PromptSegment>;
}

pub struct PromptRenderer {
    environment: RenderEnvironment,
    theme: Theme,
    providers: Vec<Box<dyn PromptSegmentProvider>>,
}

impl PromptRenderer {
    pub fn standard(
        environment: RenderEnvironment,
        repository_status: Box<dyn RepositoryStatusProvider>,
    ) -> Self {
        Self::new(
            environment,
            Theme::default(),
            vec![
                Box::new(SessionSegment),
                Box::new(PathSegment),
                Box::new(RepositorySegment { repository_status }),
                Box::new(ExitStatusSegment),
                Box::new(DurationSegment),
            ],
        )
    }

    pub fn new(
        environment: RenderEnvironment,
        theme: Theme,
        providers: Vec<Box<dyn PromptSegmentProvider>>,
    ) -> Self {
        Self {
            environment,
            theme,
            providers,
        }
    }
}

impl PromptRendering for PromptRenderer {
    fn render_prompt(&self, prompt: &PromptContext) -> String {
        let flags = prompt.flags;
        let depth = color_depth(flags);
        let nerd_icons = flags.nerd_icons() && !flags.ascii_icons();
        let context = SegmentContext {
            prompt,
            environment: &self.environment,
            nerd_icons,
        };
        let segments = self
            .providers
            .iter()
            .filter_map(|provider| provider.segment(&context))
            .map(|segment| {
                let safe_text = sanitize_for_ps1(&segment.text);
                styled(&safe_text, segment.role, depth, self.theme)
            })
            .collect::<Vec<_>>();
        let arrow = styled(
            if nerd_icons { "❯" } else { ">" },
            SemanticRole::Primary,
            depth,
            self.theme,
        );
        format!("{}\\n{} ", segments.join("  "), arrow)
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub primary: &'static str,
    pub path: &'static str,
    pub repository_clean: &'static str,
    pub repository_dirty: &'static str,
    pub warning: &'static str,
    pub danger: &'static str,
    pub error: &'static str,
    pub muted: &'static str,
}

impl Theme {
    fn ansi(self, role: SemanticRole) -> &'static str {
        match role {
            SemanticRole::Primary => self.primary,
            SemanticRole::Path => self.path,
            SemanticRole::RepositoryClean => self.repository_clean,
            SemanticRole::RepositoryDirty => self.repository_dirty,
            SemanticRole::Warning => self.warning,
            SemanticRole::Danger => self.danger,
            SemanticRole::Error => self.error,
            SemanticRole::Muted => self.muted,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: "1;38;5;81",
            path: "1;38;5;117",
            repository_clean: "38;5;114",
            repository_dirty: "38;5;215",
            warning: "1;38;5;215",
            danger: "1;38;5;196",
            error: "1;38;5;203",
            muted: "38;5;245",
        }
    }
}

struct SessionSegment;

impl PromptSegmentProvider for SessionSegment {
    fn segment(&self, context: &SegmentContext<'_>) -> Option<PromptSegment> {
        if context.prompt.flags.production() {
            Some(PromptSegment::new(
                format!(
                    "{} PROD · {} · {}",
                    if context.nerd_icons { "󰀪" } else { "!" },
                    sanitize_untrusted(context.environment.host.as_deref().unwrap_or("host")),
                    sanitize_untrusted(context.environment.user.as_deref().unwrap_or("user"))
                ),
                SemanticRole::Danger,
            ))
        } else if context.prompt.flags.ssh() {
            Some(PromptSegment::new(
                format!(
                    "{} {}",
                    if context.nerd_icons { "󰒍" } else { "ssh:" },
                    sanitize_untrusted(context.environment.host.as_deref().unwrap_or("remote"))
                ),
                SemanticRole::Warning,
            ))
        } else {
            None
        }
    }
}

struct PathSegment;

impl PromptSegmentProvider for PathSegment {
    fn segment(&self, context: &SegmentContext<'_>) -> Option<PromptSegment> {
        let path = sanitize_untrusted(&display_path(
            &context.prompt.cwd,
            context.environment.home.as_deref(),
        ));
        Some(PromptSegment::new(
            format!("{}{}", if context.nerd_icons { " " } else { "" }, path),
            SemanticRole::Path,
        ))
    }
}

struct RepositorySegment {
    repository_status: Box<dyn RepositoryStatusProvider>,
}

impl PromptSegmentProvider for RepositorySegment {
    fn segment(&self, context: &SegmentContext<'_>) -> Option<PromptSegment> {
        if context.prompt.flags.git_disabled() {
            return None;
        }
        let status = match self
            .repository_status
            .status(Path::new(&context.prompt.cwd))
        {
            Ok(Some(status)) => status,
            Ok(None) => return None,
            Err(error) => {
                trace_message(&provider_failure_diagnostic(&error));
                return None;
            }
        };
        Some(repository_segment(status, context.nerd_icons))
    }
}

fn provider_failure_diagnostic(error: &ProviderError) -> String {
    format!(
        "event=repository_provider_error kind={}",
        error.kind().as_str()
    )
}

fn repository_segment(status: RepositoryStatus, nerd_icons: bool) -> PromptSegment {
    let mut text = format!(
        "{}{}",
        if nerd_icons { "󰊢 " } else { "git:" },
        sanitize_untrusted(&status.branch)
    );
    if status.staged > 0 {
        text.push_str(&format!(" +{}", status.staged));
    }
    if status.modified > 0 {
        text.push_str(&format!(" ~{}", status.modified));
    }
    if status.untracked > 0 {
        text.push_str(&format!(" ?{}", status.untracked));
    }
    let role = if status.staged + status.modified + status.untracked == 0 {
        SemanticRole::RepositoryClean
    } else {
        SemanticRole::RepositoryDirty
    };
    PromptSegment::new(text, role)
}

struct ExitStatusSegment;

impl PromptSegmentProvider for ExitStatusSegment {
    fn segment(&self, context: &SegmentContext<'_>) -> Option<PromptSegment> {
        (context.prompt.status != 0).then(|| {
            PromptSegment::new(
                format!(
                    "{} {}",
                    if context.nerd_icons { "" } else { "exit" },
                    context.prompt.status
                ),
                SemanticRole::Error,
            )
        })
    }
}

struct DurationSegment;

impl PromptSegmentProvider for DurationSegment {
    fn segment(&self, context: &SegmentContext<'_>) -> Option<PromptSegment> {
        context
            .prompt
            .duration_ms
            .filter(|duration| *duration >= 2_000)
            .map(|duration| PromptSegment::new(format_duration(duration), SemanticRole::Muted))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ColorDepth {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

fn color_depth(flags: PromptFlags) -> ColorDepth {
    if flags.no_color() {
        ColorDepth::None
    } else if flags.truecolor() {
        ColorDepth::TrueColor
    } else if flags.color_16() {
        ColorDepth::Ansi16
    } else {
        ColorDepth::Ansi256
    }
}

fn role_sgr(role: SemanticRole, depth: ColorDepth, theme: Theme) -> String {
    match depth {
        ColorDepth::None => String::new(),
        ColorDepth::Ansi256 => theme.ansi(role).to_owned(),
        ColorDepth::Ansi16 => match role {
            SemanticRole::Primary | SemanticRole::Path => "1;36".to_owned(),
            SemanticRole::RepositoryClean => "1;32".to_owned(),
            SemanticRole::RepositoryDirty | SemanticRole::Warning => "1;33".to_owned(),
            SemanticRole::Danger | SemanticRole::Error => "1;31".to_owned(),
            SemanticRole::Muted => "1;30".to_owned(),
        },
        ColorDepth::TrueColor => match role {
            SemanticRole::Primary => "1;38;2;135;175;215".to_owned(),
            SemanticRole::Path => "1;38;2;135;215;255".to_owned(),
            SemanticRole::RepositoryClean => "38;2;135;215;135".to_owned(),
            SemanticRole::RepositoryDirty | SemanticRole::Warning => {
                "1;38;2;255;215;135".to_owned()
            }
            SemanticRole::Danger => "1;38;2;255;0;0".to_owned(),
            SemanticRole::Error => "1;38;2;255;135;135".to_owned(),
            SemanticRole::Muted => "38;2;138;138;138".to_owned(),
        },
    }
}

fn styled(text: &str, role: SemanticRole, depth: ColorDepth, theme: Theme) -> String {
    if depth == ColorDepth::None {
        return text.to_owned();
    }
    let sgr = role_sgr(role, depth, theme);
    format!("\\[\\e[{sgr}m\\]{text}\\[\\e[0m\\]")
}

fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|character| UnicodeWidthChar::width(character).unwrap_or(0))
        .sum()
}

fn display_path(cwd: &str, home: Option<&str>) -> String {
    let compact = match home.filter(|home| !home.is_empty()) {
        Some(home) if cwd == home => "~".to_owned(),
        Some(home) if cwd.starts_with(home) && cwd.as_bytes().get(home.len()) == Some(&b'/') => {
            format!("~{}", &cwd[home.len()..])
        }
        _ => cwd.to_owned(),
    };
    if display_width(&compact) > 52 {
        let parts: Vec<&str> = compact.rsplitn(3, '/').collect();
        if parts.len() == 3 {
            format!("…/{}/{}", parts[1], parts[0])
        } else {
            compact
        }
    } else {
        compact
    }
}

fn sanitize_for_ps1(value: &str) -> String {
    sanitize_with_limit(value, 1_024)
}

fn sanitize_untrusted(value: &str) -> String {
    sanitize_with_limit(value, 256)
}

fn sanitize_with_limit(value: &str, max_characters: usize) -> String {
    let mut safe = String::with_capacity(value.len());
    for character in value.chars().take(max_characters) {
        if character.is_control() || matches!(character, '$' | '`' | '\\') {
            safe.push('?');
        } else {
            safe.push(character);
        }
    }
    safe
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

#[cfg(test)]
mod tests {
    use super::*;
    use mbx_protocol::{
        FLAG_ASCII_ICONS, FLAG_COLOR_16, FLAG_DISABLE_GIT, FLAG_NO_COLOR, FLAG_PRODUCTION,
        FLAG_SSH, FLAG_TRUECOLOR,
    };
    use std::cell::Cell;
    use std::rc::Rc;

    struct StaticRepository(Option<RepositoryStatus>);

    impl RepositoryStatusProvider for StaticRepository {
        fn status(&self, _cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
            Ok(self.0.as_ref().map(|status| RepositoryStatus {
                branch: status.branch.clone(),
                staged: status.staged,
                modified: status.modified,
                untracked: status.untracked,
            }))
        }
    }

    struct FailingRepository;

    impl RepositoryStatusProvider for FailingRepository {
        fn status(&self, _cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
            Err(ProviderError::message("git status --secret-command-text"))
        }
    }

    struct CountingRepository {
        calls: Rc<Cell<usize>>,
    }

    impl RepositoryStatusProvider for CountingRepository {
        fn status(&self, _cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
            self.calls.set(self.calls.get() + 1);
            Ok(Some(RepositoryStatus {
                branch: "must-not-render".to_owned(),
                ..RepositoryStatus::default()
            }))
        }
    }

    fn request(flags: u32) -> PromptContext {
        PromptContext {
            cwd: "/tmp/project".to_owned(),
            status: 0,
            duration_ms: None,
            flags: PromptFlags::from_bits(flags),
        }
    }

    fn plain_renderer(repository: Option<RepositoryStatus>) -> PromptRenderer {
        PromptRenderer::standard(
            RenderEnvironment::default(),
            Box::new(StaticRepository(repository)),
        )
    }

    #[test]
    fn plain_prompt_preserves_segment_order() {
        let renderer = plain_renderer(None);
        let mut prompt = request(FLAG_NO_COLOR | FLAG_ASCII_ICONS | FLAG_DISABLE_GIT);
        prompt.status = 2;
        prompt.duration_ms = Some(2_500);
        assert_eq!(
            renderer.render_prompt(&prompt),
            "/tmp/project  exit 2  2.5s\\n> "
        );
    }

    #[test]
    fn production_context_precedes_and_replaces_ssh_context() {
        let renderer = PromptRenderer::standard(
            RenderEnvironment {
                home: None,
                host: Some("prod$host".to_owned()),
                user: Some("root\\user".to_owned()),
            },
            Box::new(StaticRepository(None)),
        );
        let prompt = request(
            FLAG_NO_COLOR | FLAG_ASCII_ICONS | FLAG_DISABLE_GIT | FLAG_PRODUCTION | FLAG_SSH,
        );
        assert_eq!(
            renderer.render_prompt(&prompt),
            "! PROD · prod?host · root?user  /tmp/project\\n> "
        );
    }

    #[test]
    fn repository_provider_is_substitutable_and_sanitized_centrally() {
        let renderer = plain_renderer(Some(RepositoryStatus {
            branch: "feature/$unsafe`branch".to_owned(),
            staged: 1,
            modified: 2,
            untracked: 3,
        }));
        let prompt = request(FLAG_NO_COLOR | FLAG_ASCII_ICONS);
        assert_eq!(
            renderer.render_prompt(&prompt),
            "/tmp/project  git:feature/?unsafe?branch +1 ~2 ?3\\n> "
        );
    }

    #[test]
    fn provider_error_omits_only_the_repository_segment() {
        let renderer =
            PromptRenderer::standard(RenderEnvironment::default(), Box::new(FailingRepository));
        let mut prompt = request(FLAG_NO_COLOR | FLAG_ASCII_ICONS);
        prompt.status = 7;
        prompt.duration_ms = Some(2_500);

        assert_eq!(
            renderer.render_prompt(&prompt),
            "/tmp/project  exit 7  2.5s\\n> "
        );
    }

    #[test]
    fn disabled_repository_segment_does_not_invoke_provider() {
        let calls = Rc::new(Cell::new(0));
        let renderer = PromptRenderer::standard(
            RenderEnvironment::default(),
            Box::new(CountingRepository {
                calls: Rc::clone(&calls),
            }),
        );

        assert_eq!(
            renderer.render_prompt(&request(
                FLAG_NO_COLOR | FLAG_ASCII_ICONS | FLAG_DISABLE_GIT
            )),
            "/tmp/project\\n> "
        );
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn provider_failure_diagnostic_exposes_only_the_typed_kind() {
        let error = ProviderError::message("git status --secret-command-text");
        let diagnostic = provider_failure_diagnostic(&error);

        assert_eq!(diagnostic, "event=repository_provider_error kind=other");
        assert!(!diagnostic.contains("secret-command-text"));
        assert!(!diagnostic.contains("git status"));
    }

    #[test]
    fn injected_home_controls_path_compaction() {
        let renderer = PromptRenderer::standard(
            RenderEnvironment {
                home: Some("/users/me".to_owned()),
                host: None,
                user: None,
            },
            Box::new(StaticRepository(None)),
        );
        let mut prompt = request(FLAG_NO_COLOR | FLAG_DISABLE_GIT);
        prompt.cwd = "/users/me/projects/mbx".to_owned();
        assert_eq!(renderer.render_prompt(&prompt), "~/projects/mbx\\n> ");
    }

    #[test]
    fn duration_boundaries_have_stable_formatting() {
        assert_eq!(format_duration(2_000), "2.0s");
        assert_eq!(format_duration(9_999), "10.0s");
        assert_eq!(format_duration(10_000), "10s");
        assert_eq!(format_duration(59_999), "59s");
        assert_eq!(format_duration(60_000), "1m 0s");
        assert_eq!(format_duration(125_000), "2m 5s");
    }

    #[test]
    fn default_theme_locks_256_sgr_table() {
        let theme = Theme::default();
        assert_eq!(theme.primary, "1;38;5;81");
        assert_eq!(theme.path, "1;38;5;117");
        assert_eq!(theme.repository_clean, "38;5;114");
        assert_eq!(theme.repository_dirty, "38;5;215");
        assert_eq!(theme.warning, "1;38;5;215");
        assert_eq!(theme.danger, "1;38;5;196");
        assert_eq!(theme.error, "1;38;5;203");
        assert_eq!(theme.muted, "38;5;245");
    }

    #[test]
    fn role_sgr_16_and_truecolor_match_locked_tables() {
        let theme = Theme::default();
        assert_eq!(
            role_sgr(SemanticRole::Path, ColorDepth::Ansi16, theme),
            "1;36"
        );
        assert_eq!(
            role_sgr(SemanticRole::RepositoryClean, ColorDepth::Ansi16, theme),
            "1;32"
        );
        assert_eq!(
            role_sgr(SemanticRole::Warning, ColorDepth::Ansi16, theme),
            "1;33"
        );
        assert_eq!(
            role_sgr(SemanticRole::Error, ColorDepth::Ansi16, theme),
            "1;31"
        );
        assert_eq!(
            role_sgr(SemanticRole::Muted, ColorDepth::Ansi16, theme),
            "1;30"
        );
        assert_eq!(
            role_sgr(SemanticRole::Primary, ColorDepth::TrueColor, theme),
            "1;38;2;135;175;215"
        );
        assert_eq!(
            role_sgr(SemanticRole::Path, ColorDepth::TrueColor, theme),
            "1;38;2;135;215;255"
        );
        assert_eq!(
            role_sgr(SemanticRole::RepositoryClean, ColorDepth::TrueColor, theme),
            "38;2;135;215;135"
        );
        assert_eq!(
            role_sgr(SemanticRole::Warning, ColorDepth::TrueColor, theme),
            "1;38;2;255;215;135"
        );
        assert_eq!(
            role_sgr(SemanticRole::Danger, ColorDepth::TrueColor, theme),
            "1;38;2;255;0;0"
        );
        assert_eq!(
            role_sgr(SemanticRole::Error, ColorDepth::TrueColor, theme),
            "1;38;2;255;135;135"
        );
        assert_eq!(
            role_sgr(SemanticRole::Muted, ColorDepth::TrueColor, theme),
            "38;2;138;138;138"
        );
    }

    #[test]
    fn native_prompt_uses_256_color_when_no_depth_flags_are_set() {
        let renderer = plain_renderer(None);
        let prompt = request(FLAG_ASCII_ICONS | FLAG_DISABLE_GIT);
        let rendered = renderer.render_prompt(&prompt);
        assert!(rendered.contains("38;5;"));
        assert!(!rendered.contains("38;2;"));
        assert!(!rendered.contains("1;36"));
    }

    #[test]
    fn native_prompt_uses_truecolor_when_requested() {
        let renderer = plain_renderer(None);
        let prompt = request(FLAG_ASCII_ICONS | FLAG_DISABLE_GIT | FLAG_TRUECOLOR);
        let rendered = renderer.render_prompt(&prompt);
        assert!(rendered.contains("38;2;"));
        assert!(!rendered.contains("38;5;"));
    }

    #[test]
    fn native_prompt_uses_16_color_when_requested() {
        let renderer = plain_renderer(None);
        let prompt = request(FLAG_ASCII_ICONS | FLAG_DISABLE_GIT | FLAG_COLOR_16);
        let rendered = renderer.render_prompt(&prompt);
        assert!(rendered.contains("1;36"));
        assert!(!rendered.contains("38;5;"));
        assert!(!rendered.contains("38;2;"));
    }

    #[test]
    fn display_width_counts_ascii_and_empty_strings() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn display_width_counts_east_asian_wide_glyphs() {
        assert_eq!(display_width("测"), 2);
        assert_eq!(display_width("测 试目录"), 9);
    }

    #[test]
    fn display_width_counts_combining_marks_with_their_base() {
        assert_eq!(display_width("e\u{301}"), 1);
        assert_eq!(display_width("e\u{301}tude"), 5);
    }

    #[test]
    fn wide_glyph_paths_compact_on_display_columns_not_scalar_count() {
        let wide_run = "测".repeat(23);
        let path = format!("/测测/测测/{wide_run}");
        assert!(
            path.chars().count() <= 52,
            "fixture must stay under the old scalar threshold"
        );
        assert!(display_width(&path) > 52);
        assert_eq!(display_path(&path, None), format!("…/测测/{wide_run}"));
    }

    #[test]
    fn ascii_path_at_display_width_threshold_is_not_compacted() {
        let path = format!("/{}", "a".repeat(51));
        assert_eq!(display_width(&path), 52);
        assert_eq!(display_path(&path, None), path);
    }

    #[test]
    fn long_paths_are_compacted() {
        let path = "/a/very/long/path/that/is/definitely/longer/than/fifty/two/characters/project";
        assert_eq!(display_path(path, None), "…/characters/project");
    }

    #[test]
    fn all_segment_text_crosses_the_ps1_sanitizer() {
        let safe = sanitize_for_ps1("bad$(touch /tmp/nope)`id`\\[\\e]0;title");
        assert!(!safe.contains('$'));
        assert!(!safe.contains('`'));
        assert!(!safe.contains('\\'));
        assert!(!safe.chars().any(char::is_control));
    }
}
