//! Application wiring for the MBX helper.
//!
//! The binary is intentionally only a composition root. Protocol handling,
//! rendering policy, providers, and I/O adapters live behind focused module
//! boundaries so they can be tested and changed independently.

mod app;
mod cli;
mod environment;
mod history;
mod history_service;
mod policy;
mod prompt;
mod provider;
mod service;
mod storage;
mod telemetry;
mod transport;

#[cfg(test)]
mod corpus;

use environment::RuntimeEnvironment;
use prompt::PromptRenderer;
use provider::{
    CachedRepositoryStatusProvider, DEFAULT_REPOSITORY_CACHE_TTL, GitRepositoryStatusProvider,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let runtime = RuntimeEnvironment::capture();
    let command = cli::parse(&args, || runtime.prompt_defaults())?;
    let repository_status = CachedRepositoryStatusProvider::new(
        Box::new(GitRepositoryStatusProvider::default()),
        DEFAULT_REPOSITORY_CACHE_TTL,
    );
    let renderer =
        PromptRenderer::standard(runtime.render_environment, Box::new(repository_status));
    app::execute(command, &renderer)
}

#[cfg(test)]
mod seam_contract_tests {
    use super::prompt::Theme;
    use super::provider::{ProviderError, RepositoryStatus, RepositoryStatusProvider};
    use std::path::Path;

    struct SiblingProvider;

    impl RepositoryStatusProvider for SiblingProvider {
        fn status(&self, _cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
            Err(ProviderError::message("substitute failure"))
        }
    }

    #[test]
    fn sibling_module_can_construct_theme_and_provider_error() {
        let theme = Theme {
            primary: "primary",
            path: "path",
            repository_clean: "clean",
            repository_dirty: "dirty",
            warning: "warning",
            danger: "danger",
            error: "error",
            muted: "muted",
        };
        let error = SiblingProvider.status(Path::new(".")).unwrap_err();

        assert_eq!(theme.primary, "primary");
        assert_eq!(theme.muted, "muted");
        assert_eq!(error.to_string(), "substitute failure");
    }
}
