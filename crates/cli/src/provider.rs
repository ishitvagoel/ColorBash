use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_GIT_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_GIT_DEADLINE: Duration = Duration::from_millis(50);
pub const DEFAULT_REPOSITORY_CACHE_TTL: Duration = Duration::from_secs(1);
const DEFAULT_REPOSITORY_CACHE_CAPACITY: usize = 128;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(1);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepositoryStatus {
    pub branch: String,
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryContext {
    pub root: String,
    pub branch: Option<String>,
}

pub trait RepositoryContextProvider: Send + 'static {
    fn context(&self, cwd: &Path) -> Result<Option<RepositoryContext>, ProviderError>;
}

pub struct NullRepositoryContextProvider;

impl RepositoryContextProvider for NullRepositoryContextProvider {
    fn context(&self, _cwd: &Path) -> Result<Option<RepositoryContext>, ProviderError> {
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderErrorKind {
    Resolve,
    Spawn,
    Acquisition,
    Read,
    Wait,
    Cleanup,
    Timeout,
    Oversize,
    InvalidUtf8,
    MalformedOutput,
    CommandFailure,
    Other,
}

impl ProviderErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resolve => "resolve",
            Self::Spawn => "spawn",
            Self::Acquisition => "acquisition",
            Self::Read => "read",
            Self::Wait => "wait",
            Self::Cleanup => "cleanup",
            Self::Timeout => "timeout",
            Self::Oversize => "oversize",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::MalformedOutput => "malformed_output",
            Self::CommandFailure => "command_failure",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderError {
    kind: ProviderErrorKind,
    message: String,
}

impl ProviderError {
    /// Creates an error for provider substitutes that do not use a built-in kind.
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            kind: ProviderErrorKind::Other,
            message: message.into(),
        }
    }

    pub const fn kind(&self) -> ProviderErrorKind {
        self.kind
    }

    fn typed(kind: ProviderErrorKind, message: &'static str) -> Self {
        Self {
            kind,
            ..Self::message(message)
        }
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProviderError {}

pub trait RepositoryStatusProvider {
    fn status(&self, cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GitCommandSpec {
    program: PathBuf,
    args: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
}

impl GitCommandSpec {
    fn preflight(program: &Path, cwd: &Path) -> Self {
        Self::fixed(
            program,
            cwd,
            [
                OsString::from("rev-parse"),
                OsString::from("--is-inside-work-tree"),
            ],
        )
    }

    fn status(program: &Path, cwd: &Path) -> Self {
        Self::fixed(
            program,
            cwd,
            [
                OsString::from("status"),
                OsString::from("--porcelain=v1"),
                OsString::from("--branch"),
                OsString::from("--untracked-files=normal"),
            ],
        )
    }

    fn show_toplevel(program: &Path, cwd: &Path) -> Self {
        Self::fixed(
            program,
            cwd,
            [
                OsString::from("rev-parse"),
                OsString::from("--show-toplevel"),
            ],
        )
    }

    fn symbolic_ref(program: &Path, cwd: &Path) -> Self {
        Self::fixed(
            program,
            cwd,
            [
                OsString::from("symbolic-ref"),
                OsString::from("--short"),
                OsString::from("HEAD"),
            ],
        )
    }

    fn abbrev_ref(program: &Path, cwd: &Path) -> Self {
        Self::fixed(
            program,
            cwd,
            [
                OsString::from("rev-parse"),
                OsString::from("--abbrev-ref"),
                OsString::from("HEAD"),
            ],
        )
    }

    fn fixed(program: &Path, cwd: &Path, operation: impl IntoIterator<Item = OsString>) -> Self {
        debug_assert!(program.is_absolute());
        let mut args = vec![
            OsString::from("-c"),
            OsString::from("color.ui=false"),
            OsString::from("-c"),
            OsString::from("core.fsmonitor=false"),
            OsString::from("-C"),
            cwd.as_os_str().to_owned(),
        ];
        args.extend(operation);
        Self {
            program: program.to_path_buf(),
            args,
            environment: vec![
                (OsString::from("GIT_OPTIONAL_LOCKS"), OsString::from("0")),
                (OsString::from("GIT_TERMINAL_PROMPT"), OsString::from("0")),
                (OsString::from("LC_ALL"), OsString::from("C")),
            ],
        }
    }

    #[cfg(test)]
    fn test(program: impl Into<PathBuf>, args: &[&str]) -> Self {
        Self {
            program: program.into(),
            args: args
                .iter()
                .map(|argument| OsString::from(*argument))
                .collect(),
            environment: Vec::new(),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .envs(self.environment.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        command
    }
}

fn resolve_git_executable(search_path: Option<&OsStr>) -> Result<PathBuf, ProviderError> {
    let mut executable_name = OsString::from("git");
    executable_name.push(env::consts::EXE_SUFFIX);
    resolve_executable(search_path, &executable_name, is_executable_file).ok_or_else(|| {
        ProviderError::typed(
            ProviderErrorKind::Resolve,
            "Git executable was not found in an absolute PATH entry",
        )
    })
}

fn resolve_executable(
    search_path: Option<&OsStr>,
    executable_name: &OsStr,
    mut is_executable: impl FnMut(&Path) -> bool,
) -> Option<PathBuf> {
    env::split_paths(search_path?)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(executable_name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[derive(Clone, Copy, Debug)]
struct AcquisitionPolicy {
    deadline: Duration,
    max_stdout_bytes: usize,
}

impl AcquisitionPolicy {
    fn git(deadline: Duration) -> Self {
        Self {
            deadline: deadline.min(MAX_GIT_DEADLINE),
            max_stdout_bytes: MAX_GIT_OUTPUT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GitCommandOutput {
    success: bool,
    stdout: Vec<u8>,
}

trait GitCommandRunner: Send {
    fn run(
        &self,
        spec: &GitCommandSpec,
        policy: AcquisitionPolicy,
    ) -> Result<GitCommandOutput, ProviderError>;
}

#[derive(Debug, Default)]
struct BoundedGitCommandRunner;

impl GitCommandRunner for BoundedGitCommandRunner {
    fn run(
        &self,
        spec: &GitCommandSpec,
        policy: AcquisitionPolicy,
    ) -> Result<GitCommandOutput, ProviderError> {
        let started = Instant::now();
        let mut child = spec.command().spawn().map_err(|_| {
            ProviderError::typed(ProviderErrorKind::Spawn, "failed to start Git inspection")
        })?;
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_and_reap(&mut child)?;
                return Err(ProviderError::typed(
                    ProviderErrorKind::Acquisition,
                    "Git inspection stdout was unavailable",
                ));
            }
        };

        let (sender, receiver) = mpsc::sync_channel(1);
        let max_stdout_bytes = policy.max_stdout_bytes;
        let reader = match thread::Builder::new()
            .name("mbx-git-stdout".to_owned())
            .spawn(move || {
                let result = read_capped(stdout, max_stdout_bytes);
                let _ = sender.send(result);
            }) {
            Ok(reader) => reader,
            Err(_) => {
                terminate_and_reap(&mut child)?;
                return Err(ProviderError::typed(
                    ProviderErrorKind::Acquisition,
                    "failed to start bounded Git output acquisition",
                ));
            }
        };

        let mut stdout = None;
        let mut success = None;
        loop {
            let elapsed = started.elapsed();
            if elapsed >= policy.deadline {
                terminate_and_reap(&mut child)?;
                // Do not join here: an unexpected descendant that inherited stdout
                // must not extend the provider deadline after the direct child dies.
                drop(reader);
                return Err(ProviderError::typed(
                    ProviderErrorKind::Timeout,
                    "Git inspection exceeded its deadline",
                ));
            }

            if stdout.is_none() {
                match receiver.try_recv() {
                    Ok(Ok(bytes)) => stdout = Some(bytes),
                    Ok(Err(error)) => {
                        terminate_and_reap(&mut child)?;
                        let _ = reader.join();
                        return Err(error);
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        terminate_and_reap(&mut child)?;
                        let _ = reader.join();
                        return Err(ProviderError::typed(
                            ProviderErrorKind::Read,
                            "bounded Git output acquisition stopped unexpectedly",
                        ));
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }

            if success.is_none() {
                match child.try_wait() {
                    Ok(Some(status)) => success = Some(status.success()),
                    Ok(None) => {}
                    Err(_) => {
                        terminate_and_reap(&mut child)?;
                        return Err(ProviderError::typed(
                            ProviderErrorKind::Wait,
                            "failed while waiting for Git inspection",
                        ));
                    }
                }
            }

            let completed = success.and_then(|success| {
                stdout
                    .take()
                    .map(|stdout| GitCommandOutput { success, stdout })
            });
            if let Some(output) = completed {
                if reader.join().is_err() {
                    return Err(ProviderError::typed(
                        ProviderErrorKind::Read,
                        "bounded Git output acquisition stopped unexpectedly",
                    ));
                }
                if started.elapsed() >= policy.deadline {
                    return Err(ProviderError::typed(
                        ProviderErrorKind::Timeout,
                        "Git inspection exceeded its deadline",
                    ));
                }
                return Ok(output);
            }

            thread::sleep(
                policy
                    .deadline
                    .saturating_sub(started.elapsed())
                    .min(PROCESS_POLL_INTERVAL),
            );
        }
    }
}

fn read_capped(reader: impl Read, max_bytes: usize) -> Result<Vec<u8>, ProviderError> {
    let acquisition_limit = max_bytes.saturating_add(1);
    let mut bytes = Vec::with_capacity(acquisition_limit.min(8 * 1024));
    reader
        .take(acquisition_limit as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            ProviderError::typed(
                ProviderErrorKind::Read,
                "failed to read Git inspection output",
            )
        })?;
    if bytes.len() > max_bytes {
        return Err(ProviderError::typed(
            ProviderErrorKind::Oversize,
            "Git inspection output exceeded its acquisition limit",
        ));
    }
    Ok(bytes)
}

/// Stops and reaps the direct child. This intentionally makes no portable
/// process-tree cleanup claim; the timeout path must remain bounded even if an
/// unexpected descendant inherited the stdout pipe.
fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), ProviderError> {
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(_) => {
            return Err(ProviderError::typed(
                ProviderErrorKind::Cleanup,
                "failed to inspect Git child state during cleanup",
            ));
        }
    }

    if let Err(error) = child.kill() {
        if error.kind() != std::io::ErrorKind::InvalidInput {
            return Err(ProviderError::typed(
                ProviderErrorKind::Cleanup,
                "failed to stop Git inspection",
            ));
        }
    }
    child.wait().map(|_| ()).map_err(|_| {
        ProviderError::typed(ProviderErrorKind::Cleanup, "failed to reap Git inspection")
    })
}

pub struct GitRepositoryStatusProvider {
    executable: Result<PathBuf, ProviderError>,
    runner: Box<dyn GitCommandRunner + Send>,
    policy: AcquisitionPolicy,
}

impl GitRepositoryStatusProvider {
    pub fn new(deadline: Duration) -> Self {
        let search_path = env::var_os("PATH");
        Self::with_resolution(
            deadline,
            resolve_git_executable(search_path.as_deref()),
            Box::new(BoundedGitCommandRunner),
        )
    }

    fn with_resolution(
        deadline: Duration,
        executable: Result<PathBuf, ProviderError>,
        runner: Box<dyn GitCommandRunner + Send>,
    ) -> Self {
        Self {
            executable,
            runner,
            policy: AcquisitionPolicy::git(deadline),
        }
    }

    #[cfg(test)]
    fn with_runner(
        deadline: Duration,
        executable: impl Into<PathBuf>,
        runner: Box<dyn GitCommandRunner + Send>,
    ) -> Self {
        let executable = executable.into();
        assert!(executable.is_absolute());
        Self::with_resolution(deadline, Ok(executable), runner)
    }

    fn remaining_policy(&self, started: Instant) -> Result<AcquisitionPolicy, ProviderError> {
        let deadline = self.policy.deadline.saturating_sub(started.elapsed());
        if deadline.is_zero() {
            return Err(ProviderError::typed(
                ProviderErrorKind::Timeout,
                "Git inspection exceeded its deadline",
            ));
        }
        Ok(AcquisitionPolicy {
            deadline,
            max_stdout_bytes: self.policy.max_stdout_bytes,
        })
    }
}

impl Default for GitRepositoryStatusProvider {
    fn default() -> Self {
        Self::new(MAX_GIT_DEADLINE)
    }
}

impl RepositoryStatusProvider for GitRepositoryStatusProvider {
    fn status(&self, cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
        let executable = self.executable.as_ref().map_err(Clone::clone)?;
        let started = Instant::now();

        let preflight = self.runner.run(
            &GitCommandSpec::preflight(executable, cwd),
            self.remaining_policy(started)?,
        )?;
        // Only the fixed discovery preflight maps non-success to absence. Once
        // a work tree is established, any failing status command is an error.
        if !preflight.success {
            return Ok(None);
        }
        match String::from_utf8(preflight.stdout)
            .map_err(|_| {
                ProviderError::typed(
                    ProviderErrorKind::InvalidUtf8,
                    "Git preflight output was not valid UTF-8",
                )
            })?
            .trim()
        {
            "true" => {}
            "false" => return Ok(None),
            _ => {
                return Err(ProviderError::typed(
                    ProviderErrorKind::MalformedOutput,
                    "Git preflight output was not recognized",
                ));
            }
        }

        let output = self.runner.run(
            &GitCommandSpec::status(executable, cwd),
            self.remaining_policy(started)?,
        )?;
        if !output.success {
            return Err(ProviderError::typed(
                ProviderErrorKind::CommandFailure,
                "Git status inspection failed",
            ));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|_| {
            ProviderError::typed(
                ProviderErrorKind::InvalidUtf8,
                "Git inspection output was not valid UTF-8",
            )
        })?;
        parse_git_status(&stdout).map(Some).ok_or_else(|| {
            ProviderError::typed(
                ProviderErrorKind::MalformedOutput,
                "Git inspection output had no branch header",
            )
        })
    }
}

impl RepositoryContextProvider for GitRepositoryStatusProvider {
    fn context(&self, cwd: &Path) -> Result<Option<RepositoryContext>, ProviderError> {
        if !cwd.is_absolute() {
            return Ok(None);
        }
        let executable = self.executable.as_ref().map_err(Clone::clone)?;
        let started = Instant::now();
        let toplevel = self.runner.run(
            &GitCommandSpec::show_toplevel(executable, cwd),
            self.remaining_policy(started)?,
        )?;
        if !toplevel.success {
            return Ok(None);
        }
        let stdout = String::from_utf8(toplevel.stdout).map_err(|_| {
            ProviderError::typed(
                ProviderErrorKind::InvalidUtf8,
                "Git toplevel output was not valid UTF-8",
            )
        })?;
        let root = parse_show_toplevel(&stdout).ok_or_else(|| {
            ProviderError::typed(
                ProviderErrorKind::MalformedOutput,
                "Git toplevel output was not an absolute path",
            )
        })?;

        let branch = self
            .remaining_policy(started)
            .ok()
            .and_then(|policy| {
                self.runner
                    .run(&GitCommandSpec::symbolic_ref(executable, cwd), policy)
                    .ok()
            })
            .and_then(|output| {
                if output.success {
                    String::from_utf8(output.stdout)
                        .ok()
                        .and_then(|text| parse_ref_name(&text))
                } else {
                    self.remaining_policy(started)
                        .ok()
                        .and_then(|policy| {
                            self.runner
                                .run(&GitCommandSpec::abbrev_ref(executable, cwd), policy)
                                .ok()
                        })
                        .and_then(|abbrev| {
                            if abbrev.success {
                                String::from_utf8(abbrev.stdout)
                                    .ok()
                                    .and_then(|text| parse_ref_name(&text))
                            } else {
                                None
                            }
                        })
                }
            });

        Ok(Some(RepositoryContext { root, branch }))
    }
}

trait MonotonicClock {
    fn now(&self) -> Instant;
}

struct SystemClock;

impl MonotonicClock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Clone)]
struct CacheEntry {
    recorded_at: Instant,
    result: Result<Option<RepositoryStatus>, ProviderError>,
}

#[derive(Default)]
struct CacheState {
    entries: HashMap<PathBuf, CacheEntry>,
}

pub struct CachedRepositoryStatusProvider {
    inner: Box<dyn RepositoryStatusProvider>,
    ttl: Duration,
    capacity: usize,
    clock: Box<dyn MonotonicClock>,
    state: Mutex<CacheState>,
}

impl CachedRepositoryStatusProvider {
    /// Caches status, absence, and provider errors for the same short
    /// TTL so repeated prompts cannot immediately restart a failing Git lookup.
    pub fn new(inner: Box<dyn RepositoryStatusProvider>, ttl: Duration) -> Self {
        Self::with_clock(
            inner,
            ttl,
            DEFAULT_REPOSITORY_CACHE_CAPACITY,
            Box::new(SystemClock),
        )
    }

    fn with_clock(
        inner: Box<dyn RepositoryStatusProvider>,
        ttl: Duration,
        capacity: usize,
        clock: Box<dyn MonotonicClock>,
    ) -> Self {
        Self {
            inner,
            ttl,
            capacity,
            clock,
            state: Mutex::new(CacheState::default()),
        }
    }

    pub fn invalidate(&self, cwd: &Path) {
        self.lock_state().entries.remove(cwd);
    }

    fn lock_state(&self) -> MutexGuard<'_, CacheState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn insert(&self, cwd: &Path, result: Result<Option<RepositoryStatus>, ProviderError>) {
        if self.capacity == 0 {
            return;
        }
        let mut state = self.lock_state();
        if !state.entries.contains_key(cwd) && state.entries.len() >= self.capacity {
            let oldest = state
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.recorded_at)
                .map(|(path, _)| path.clone());
            if let Some(oldest) = oldest {
                state.entries.remove(&oldest);
            }
        }
        state.entries.insert(
            cwd.to_path_buf(),
            CacheEntry {
                recorded_at: self.clock.now(),
                result,
            },
        );
    }
}

impl RepositoryStatusProvider for CachedRepositoryStatusProvider {
    fn status(&self, cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
        let now = self.clock.now();
        let stale = {
            let state = self.lock_state();
            if let Some(entry) = state.entries.get(cwd) {
                if now
                    .checked_duration_since(entry.recorded_at)
                    .is_some_and(|age| age < self.ttl)
                {
                    return entry.result.clone();
                }
                true
            } else {
                false
            }
        };
        if stale {
            self.invalidate(cwd);
        }

        let result = self.inner.status(cwd);
        self.insert(cwd, result.clone());
        result
    }
}

fn parse_git_status(stdout: &str) -> Option<RepositoryStatus> {
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
    let mut status = RepositoryStatus {
        branch: if branch.is_empty() {
            "detached".to_owned()
        } else {
            branch
        },
        ..RepositoryStatus::default()
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

fn parse_show_toplevel(stdout: &str) -> Option<String> {
    let line = stdout.lines().next()?.trim();
    if line.starts_with('/') && !line.bytes().any(|byte| byte < 0x20 || byte == 0x7f) {
        Some(line.to_owned())
    } else {
        None
    }
}

fn parse_ref_name(stdout: &str) -> Option<String> {
    let line = stdout.lines().next()?.trim();
    if line.is_empty() || line.bytes().any(|byte| byte < 0x20 || byte == 0x7f) {
        None
    } else {
        Some(line.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct StubRunner {
        calls: Arc<AtomicUsize>,
        specs: Arc<Mutex<Vec<GitCommandSpec>>>,
        policies: Arc<Mutex<Vec<AcquisitionPolicy>>>,
        results: Arc<Mutex<VecDeque<Result<GitCommandOutput, ProviderError>>>>,
    }

    impl GitCommandRunner for StubRunner {
        fn run(
            &self,
            spec: &GitCommandSpec,
            policy: AcquisitionPolicy,
        ) -> Result<GitCommandOutput, ProviderError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.specs.lock().expect("stub specs").push(spec.clone());
            self.policies.lock().expect("stub policies").push(policy);
            self.results
                .lock()
                .expect("stub results")
                .pop_front()
                .expect("stub runner needs one result per expected call")
        }
    }

    struct PanicRunner;

    impl GitCommandRunner for PanicRunner {
        fn run(
            &self,
            _spec: &GitCommandSpec,
            _policy: AcquisitionPolicy,
        ) -> Result<GitCommandOutput, ProviderError> {
            panic!("an unresolved executable must never reach command execution")
        }
    }

    struct StubProviderFixture {
        provider: GitRepositoryStatusProvider,
        calls: Arc<AtomicUsize>,
        specs: Arc<Mutex<Vec<GitCommandSpec>>>,
        policies: Arc<Mutex<Vec<AcquisitionPolicy>>>,
    }

    fn stub_provider(
        results: impl IntoIterator<Item = Result<GitCommandOutput, ProviderError>>,
    ) -> StubProviderFixture {
        let calls = Arc::new(AtomicUsize::new(0));
        let specs = Arc::new(Mutex::new(Vec::new()));
        let policies = Arc::new(Mutex::new(Vec::new()));
        let runner = StubRunner {
            calls: Arc::clone(&calls),
            specs: Arc::clone(&specs),
            policies: Arc::clone(&policies),
            results: Arc::new(Mutex::new(results.into_iter().collect())),
        };
        StubProviderFixture {
            provider: GitRepositoryStatusProvider::with_runner(
                MAX_GIT_DEADLINE,
                fake_git_executable(),
                Box::new(runner),
            ),
            calls,
            specs,
            policies,
        }
    }

    fn fake_git_executable() -> PathBuf {
        PathBuf::from("/resolved/bin/git")
    }

    fn successful_output(stdout: impl Into<Vec<u8>>) -> Result<GitCommandOutput, ProviderError> {
        Ok(GitCommandOutput {
            success: true,
            stdout: stdout.into(),
        })
    }

    #[test]
    fn git_command_spec_is_fixed_and_disables_hostile_fsmonitor_configuration() {
        let executable = fake_git_executable();
        let spec = GitCommandSpec::status(&executable, Path::new("/tmp/repository"));
        let command = spec.command();
        assert_eq!(command.get_program(), executable.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                OsStr::new("-c"),
                OsStr::new("color.ui=false"),
                OsStr::new("-c"),
                OsStr::new("core.fsmonitor=false"),
                OsStr::new("-C"),
                OsStr::new("/tmp/repository"),
                OsStr::new("status"),
                OsStr::new("--porcelain=v1"),
                OsStr::new("--branch"),
                OsStr::new("--untracked-files=normal"),
            ]
        );
        let environment = command
            .get_envs()
            .map(|(key, value)| (key, value.expect("the command sets a value")))
            .collect::<HashMap<_, _>>();
        assert_eq!(environment.len(), 3);
        assert_eq!(
            environment.get(OsStr::new("GIT_OPTIONAL_LOCKS")),
            Some(&OsStr::new("0"))
        );
        assert_eq!(
            environment.get(OsStr::new("GIT_TERMINAL_PROMPT")),
            Some(&OsStr::new("0"))
        );
        assert_eq!(
            environment.get(OsStr::new("LC_ALL")),
            Some(&OsStr::new("C"))
        );
    }

    #[test]
    fn executable_resolution_ignores_empty_and_relative_path_entries() {
        let directory = TestDirectory::new();
        let trusted_directory = directory.path().join("trusted-bin");
        let search_path = env::join_paths([
            PathBuf::new(),
            PathBuf::from("."),
            PathBuf::from("repository-local-bin"),
            trusted_directory.clone(),
        ])
        .unwrap();
        let probed = Rc::new(RefCell::new(Vec::new()));

        let executable = resolve_executable(Some(search_path.as_os_str()), OsStr::new("git"), {
            let probed = Rc::clone(&probed);
            move |candidate| {
                probed.borrow_mut().push(candidate.to_path_buf());
                candidate == trusted_directory.join("git")
            }
        })
        .expect("the absolute trusted entry should resolve");

        assert!(executable.is_absolute());
        assert_eq!(
            probed.borrow().as_slice(),
            std::slice::from_ref(&executable)
        );
        assert_eq!(
            GitCommandSpec::status(&executable, directory.path()).program,
            executable
        );
    }

    #[test]
    fn executable_resolution_failure_is_typed_and_never_falls_back_to_bare_git() {
        let search_path = env::join_paths([PathBuf::new(), PathBuf::from(".")]).unwrap();
        let error = resolve_git_executable(Some(search_path.as_os_str())).unwrap_err();
        let provider = GitRepositoryStatusProvider::with_resolution(
            MAX_GIT_DEADLINE,
            Err(error.clone()),
            Box::new(PanicRunner),
        );

        assert_eq!(error.kind(), ProviderErrorKind::Resolve);
        assert_eq!(
            provider.status(Path::new(".")).unwrap_err().kind(),
            ProviderErrorKind::Resolve
        );
    }

    #[test]
    fn hostile_repository_fsmonitor_configuration_is_not_executed() {
        let directory = TestDirectory::new();
        let git = test_executable("git");
        let hook = directory.path().join("hostile-fsmonitor");
        let marker = directory.path().join("hostile-fsmonitor.invoked");
        fs::write(&hook, "#!/bin/sh\n: > \"$0.invoked\"\n").unwrap();
        let mut permissions = fs::metadata(&hook).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&hook, permissions).unwrap();

        assert!(
            Command::new(&git)
                .args(["init", "--quiet"])
                .current_dir(directory.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new(&git)
                .args(["config", "--local", "core.fsmonitor"])
                .arg(&hook)
                .current_dir(directory.path())
                .status()
                .unwrap()
                .success()
        );

        let _ = Command::new(&git)
            .args(["status", "--porcelain=v1", "--branch"])
            .current_dir(directory.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(
            marker.exists(),
            "control Git command did not invoke the configured monitor"
        );
        fs::remove_file(&marker).unwrap();

        let status = GitRepositoryStatusProvider::default()
            .status(directory.path())
            .unwrap()
            .expect("the temporary repository should be detected");
        assert!(!status.branch.is_empty());
        assert!(
            !marker.exists(),
            "the fixed provider command invoked hostile filesystem-monitor configuration"
        );
    }

    #[test]
    fn non_repository_preflight_is_normal_absence() {
        let directory = TestDirectory::new();

        assert_eq!(
            GitRepositoryStatusProvider::default()
                .status(directory.path())
                .unwrap(),
            None
        );
    }

    #[test]
    fn context_spec_uses_show_toplevel_and_symbolic_ref() {
        let executable = fake_git_executable();
        let cwd = Path::new("/tmp/repository");
        let fixture = stub_provider([
            successful_output(b"/tmp/repository\n".to_vec()),
            successful_output(b"hist-branch\n".to_vec()),
        ]);
        let context = fixture.provider.context(cwd).unwrap().unwrap();
        assert_eq!(context.root, "/tmp/repository");
        assert_eq!(context.branch.as_deref(), Some("hist-branch"));
        assert_eq!(
            fixture.specs.lock().expect("specs").as_slice(),
            &[
                GitCommandSpec::show_toplevel(&executable, cwd),
                GitCommandSpec::symbolic_ref(&executable, cwd),
            ]
        );
    }

    #[test]
    fn context_falls_back_to_abbrev_ref_when_symbolic_ref_fails() {
        let cwd = Path::new("/tmp/repository");
        let fixture = stub_provider([
            successful_output(b"/tmp/repository\n".to_vec()),
            Ok(GitCommandOutput {
                success: false,
                stdout: Vec::new(),
            }),
            successful_output(b"HEAD\n".to_vec()),
        ]);
        let context = fixture.provider.context(cwd).unwrap().unwrap();
        assert_eq!(context.branch.as_deref(), Some("HEAD"));
        assert_eq!(fixture.calls.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn context_skips_relative_cwd_without_running_git() {
        let fixture = stub_provider([]);
        assert_eq!(
            fixture.provider.context(Path::new("relative")).unwrap(),
            None
        );
        assert_eq!(fixture.calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn context_treats_failed_toplevel_as_absence() {
        let fixture = stub_provider([Ok(GitCommandOutput {
            success: false,
            stdout: Vec::new(),
        })]);
        assert_eq!(
            fixture
                .provider
                .context(Path::new("/tmp/not-a-repo"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn context_returns_root_and_branch_for_a_worktree() {
        let directory = TestDirectory::new();
        let git = test_executable("git");
        assert!(
            Command::new(&git)
                .args(["init", "--quiet"])
                .current_dir(directory.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new(&git)
                .args(["symbolic-ref", "HEAD", "refs/heads/hist-branch"])
                .current_dir(directory.path())
                .status()
                .unwrap()
                .success()
        );
        let context = GitRepositoryStatusProvider::default()
            .context(directory.path())
            .unwrap()
            .expect("the temporary repository should be detected");
        let expected = directory.path().canonicalize().unwrap();
        assert_eq!(Path::new(&context.root), expected.as_path());
        assert_eq!(context.branch.as_deref(), Some("hist-branch"));
    }

    #[test]
    fn bounded_reader_accepts_max_and_rejects_max_plus_one() {
        let exact = vec![b'x'; MAX_GIT_OUTPUT_BYTES];
        assert_eq!(
            read_capped(Cursor::new(exact.clone()), MAX_GIT_OUTPUT_BYTES).unwrap(),
            exact
        );
        let error = read_capped(
            Cursor::new(vec![b'x'; MAX_GIT_OUTPUT_BYTES + 1]),
            MAX_GIT_OUTPUT_BYTES,
        )
        .unwrap_err();
        assert_eq!(error.kind(), ProviderErrorKind::Oversize);
    }

    #[test]
    fn bounded_runner_distinguishes_spawn_timeout_and_oversize() {
        let runner = BoundedGitCommandRunner;
        let shell = test_executable("sh");
        let spawn_error = runner
            .run(
                &GitCommandSpec::test("/definitely/missing/mbx-git", &[]),
                AcquisitionPolicy {
                    deadline: MAX_GIT_DEADLINE,
                    max_stdout_bytes: 8,
                },
            )
            .unwrap_err();
        assert_eq!(spawn_error.kind(), ProviderErrorKind::Spawn);

        let started = Instant::now();
        let timeout_error = runner
            .run(
                &GitCommandSpec::test(shell.clone(), &["-c", "while :; do :; done"]),
                AcquisitionPolicy {
                    deadline: Duration::from_millis(10),
                    max_stdout_bytes: 8,
                },
            )
            .unwrap_err();
        assert_eq!(timeout_error.kind(), ProviderErrorKind::Timeout);
        assert!(started.elapsed() < Duration::from_millis(250));

        let oversize_error = runner
            .run(
                &GitCommandSpec::test(shell, &["-c", "printf 123456789"]),
                AcquisitionPolicy {
                    deadline: MAX_GIT_DEADLINE,
                    max_stdout_bytes: 8,
                },
            )
            .unwrap_err();
        assert_eq!(oversize_error.kind(), ProviderErrorKind::Oversize);
    }

    #[test]
    fn timeout_reaps_direct_child_without_waiting_for_descendant_inherited_stdout() {
        let directory = TestDirectory::new();
        let pid_file = directory.path().join("pids");
        let shell = test_executable("sh");
        let sleep = test_executable("sleep");
        let pid_file_text = pid_file.to_str().unwrap();
        let sleep_text = sleep.to_str().unwrap();
        let script = concat!(
            "\"$2\" 5 & descendant=$!; ",
            "printf '%s %s\\n' \"$$\" \"$descendant\" > \"$1\"; ",
            "exec \"$2\" 5"
        );
        let runner = BoundedGitCommandRunner;
        let started = Instant::now();

        let error = runner
            .run(
                &GitCommandSpec::test(
                    shell.clone(),
                    &["-c", script, "mbx-test", pid_file_text, sleep_text],
                ),
                AcquisitionPolicy {
                    deadline: Duration::from_millis(40),
                    max_stdout_bytes: 8,
                },
            )
            .unwrap_err();
        let elapsed = started.elapsed();
        let pids = fs::read_to_string(&pid_file).unwrap();
        let mut pids = pids.split_whitespace().map(str::parse::<u32>);
        let direct_pid = pids.next().unwrap().unwrap();
        let descendant_pid = pids.next().unwrap().unwrap();
        let direct_exists = process_exists(&shell, direct_pid);
        let descendant_exists = process_exists(&shell, descendant_pid);
        if descendant_exists {
            stop_process(&shell, descendant_pid);
        }

        assert_eq!(error.kind(), ProviderErrorKind::Timeout);
        assert!(elapsed < Duration::from_millis(250));
        assert!(!direct_exists);
        assert!(
            descendant_exists,
            "the background process must still hold stdout when the runner returns"
        );
    }

    #[test]
    fn configured_deadline_is_clamped_to_fifty_milliseconds() {
        let provider = GitRepositoryStatusProvider::new(Duration::from_secs(10));
        assert_eq!(provider.policy.deadline, MAX_GIT_DEADLINE);
    }

    #[test]
    fn provider_distinguishes_absence_and_typed_failures() {
        let absent = stub_provider([Ok(GitCommandOutput {
            success: false,
            stdout: Vec::new(),
        })]);
        assert_eq!(absent.provider.status(Path::new(".")).unwrap(), None);
        assert_eq!(absent.calls.load(Ordering::Relaxed), 1);

        let invalid_utf8 = stub_provider([
            successful_output(b"true\n".to_vec()),
            successful_output(vec![0xff]),
        ]);
        assert_eq!(
            invalid_utf8
                .provider
                .status(Path::new("."))
                .unwrap_err()
                .kind(),
            ProviderErrorKind::InvalidUtf8
        );

        let malformed = stub_provider([
            successful_output(b"true\n".to_vec()),
            successful_output(b"not porcelain\n".to_vec()),
        ]);
        assert_eq!(
            malformed
                .provider
                .status(Path::new("."))
                .unwrap_err()
                .kind(),
            ProviderErrorKind::MalformedOutput
        );

        let command_failure = stub_provider([
            successful_output(b"true\n".to_vec()),
            Ok(GitCommandOutput {
                success: false,
                stdout: Vec::new(),
            }),
        ]);
        assert_eq!(
            command_failure
                .provider
                .status(Path::new("."))
                .unwrap_err()
                .kind(),
            ProviderErrorKind::CommandFailure
        );
    }

    #[test]
    fn provider_uses_the_fixed_spec_and_keeps_parser_output_typed() {
        let hostile_branch = "feature/$unsafe`branch\u{1b}]0;title";
        let fixture = stub_provider([
            successful_output(b"true\n".to_vec()),
            successful_output(format!("## {hostile_branch}\n M changed\n").into_bytes()),
        ]);
        let status = fixture.provider.status(Path::new(".")).unwrap().unwrap();
        assert_eq!(status.branch, hostile_branch);
        assert_eq!(status.modified, 1);
        assert_eq!(
            fixture.specs.lock().expect("specs").as_slice(),
            &[
                GitCommandSpec::preflight(&fake_git_executable(), Path::new(".")),
                GitCommandSpec::status(&fake_git_executable(), Path::new(".")),
            ]
        );
        let policies = fixture.policies.lock().expect("policies");
        assert_eq!(policies.len(), 2);
        assert!(policies[0].deadline <= MAX_GIT_DEADLINE);
        assert!(policies[1].deadline <= policies[0].deadline);
    }

    struct TestClock(Rc<Cell<Instant>>);

    impl MonotonicClock for TestClock {
        fn now(&self) -> Instant {
            self.0.get()
        }
    }

    struct SequencedProvider {
        calls: Rc<Cell<usize>>,
    }

    struct ResultSequenceProvider {
        calls: Rc<Cell<usize>>,
        results: RefCell<VecDeque<Result<Option<RepositoryStatus>, ProviderError>>>,
    }

    impl RepositoryStatusProvider for ResultSequenceProvider {
        fn status(&self, _cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
            self.calls.set(self.calls.get() + 1);
            self.results
                .borrow_mut()
                .pop_front()
                .expect("result sequence needs one result per cache miss")
        }
    }

    impl RepositoryStatusProvider for SequencedProvider {
        fn status(&self, _cwd: &Path) -> Result<Option<RepositoryStatus>, ProviderError> {
            let call = self.calls.get() + 1;
            self.calls.set(call);
            Ok(Some(RepositoryStatus {
                branch: format!("branch-{call}"),
                ..RepositoryStatus::default()
            }))
        }
    }

    fn cached_provider(
        ttl: Duration,
    ) -> (
        CachedRepositoryStatusProvider,
        Rc<Cell<usize>>,
        Rc<Cell<Instant>>,
    ) {
        let calls = Rc::new(Cell::new(0));
        let now = Rc::new(Cell::new(Instant::now()));
        let provider = CachedRepositoryStatusProvider::with_clock(
            Box::new(SequencedProvider {
                calls: Rc::clone(&calls),
            }),
            ttl,
            8,
            Box::new(TestClock(Rc::clone(&now))),
        );
        (provider, calls, now)
    }

    #[test]
    fn repository_cache_hits_until_deterministic_expiry() {
        let ttl = Duration::from_secs(1);
        let (provider, calls, now) = cached_provider(ttl);

        let first = provider.status(Path::new("/work")).unwrap().unwrap();
        let hit = provider.status(Path::new("/work")).unwrap().unwrap();
        assert_eq!(first.branch, "branch-1");
        assert_eq!(hit, first);
        assert_eq!(calls.get(), 1);

        now.set(now.get() + ttl);
        let expired = provider.status(Path::new("/work")).unwrap().unwrap();
        assert_eq!(expired.branch, "branch-2");
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn repository_cache_negatively_caches_absence_until_expiry() {
        let ttl = Duration::from_secs(1);
        let calls = Rc::new(Cell::new(0));
        let now = Rc::new(Cell::new(Instant::now()));
        let provider = CachedRepositoryStatusProvider::with_clock(
            Box::new(ResultSequenceProvider {
                calls: Rc::clone(&calls),
                results: RefCell::new(
                    [
                        Ok(None),
                        Ok(Some(RepositoryStatus {
                            branch: "available".to_owned(),
                            ..RepositoryStatus::default()
                        })),
                    ]
                    .into(),
                ),
            }),
            ttl,
            8,
            Box::new(TestClock(Rc::clone(&now))),
        );

        assert_eq!(provider.status(Path::new("/work")).unwrap(), None);
        assert_eq!(provider.status(Path::new("/work")).unwrap(), None);
        assert_eq!(calls.get(), 1);

        now.set(now.get() + ttl);
        assert_eq!(
            provider.status(Path::new("/work")).unwrap().unwrap().branch,
            "available"
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn repository_cache_holds_transient_errors_only_until_expiry() {
        let ttl = Duration::from_secs(1);
        let calls = Rc::new(Cell::new(0));
        let now = Rc::new(Cell::new(Instant::now()));
        let transient =
            ProviderError::typed(ProviderErrorKind::Timeout, "transient provider timeout");
        let provider = CachedRepositoryStatusProvider::with_clock(
            Box::new(ResultSequenceProvider {
                calls: Rc::clone(&calls),
                results: RefCell::new(
                    [
                        Err(transient.clone()),
                        Ok(Some(RepositoryStatus {
                            branch: "recovered".to_owned(),
                            ..RepositoryStatus::default()
                        })),
                    ]
                    .into(),
                ),
            }),
            ttl,
            8,
            Box::new(TestClock(Rc::clone(&now))),
        );

        assert_eq!(provider.status(Path::new("/work")).unwrap_err(), transient);
        assert_eq!(provider.status(Path::new("/work")).unwrap_err(), transient);
        assert_eq!(calls.get(), 1);

        now.set(now.get() + ttl);
        assert_eq!(
            provider.status(Path::new("/work")).unwrap().unwrap().branch,
            "recovered"
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn repository_cache_invalidation_forces_refresh() {
        let (provider, calls, _) = cached_provider(Duration::from_secs(1));

        assert_eq!(
            provider.status(Path::new("/work")).unwrap().unwrap().branch,
            "branch-1"
        );
        provider.invalidate(Path::new("/work"));
        assert_eq!(
            provider.status(Path::new("/work")).unwrap().unwrap().branch,
            "branch-2"
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn repository_cache_evicts_when_its_fixed_capacity_is_reached() {
        let calls = Rc::new(Cell::new(0));
        let now = Rc::new(Cell::new(Instant::now()));
        let provider = CachedRepositoryStatusProvider::with_clock(
            Box::new(SequencedProvider {
                calls: Rc::clone(&calls),
            }),
            Duration::from_secs(60),
            1,
            Box::new(TestClock(Rc::clone(&now))),
        );

        provider.status(Path::new("/first")).unwrap();
        now.set(now.get() + Duration::from_millis(1));
        provider.status(Path::new("/second")).unwrap();
        provider.status(Path::new("/first")).unwrap();

        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn parses_clean_branch() {
        assert_eq!(
            parse_git_status("## main\n"),
            Some(RepositoryStatus {
                branch: "main".to_owned(),
                ..RepositoryStatus::default()
            })
        );
    }

    #[test]
    fn strips_upstream_and_tracking_details_from_branch() {
        assert_eq!(
            parse_git_status("## feature/auth...origin/feature/auth [ahead 2, behind 1]\n"),
            Some(RepositoryStatus {
                branch: "feature/auth".to_owned(),
                ..RepositoryStatus::default()
            })
        );
    }

    #[test]
    fn parses_both_unborn_branch_header_forms() {
        for (header, expected_branch) in [
            ("No commits yet on main", "main"),
            ("Initial commit on trunk", "trunk"),
        ] {
            let status = parse_git_status(&format!("## {header}\n?? README.md\n")).unwrap();
            assert_eq!(status.branch, expected_branch);
            assert_eq!(status.untracked, 1);
        }
    }

    #[test]
    fn counts_index_worktree_untracked_and_unmerged_states() {
        let status = parse_git_status(
            "## main\nM  staged.rs\n M modified.rs\nMM both.rs\n?? untracked.rs\nUU conflict.rs\n",
        )
        .unwrap();

        assert_eq!(
            status,
            RepositoryStatus {
                branch: "main".to_owned(),
                staged: 3,
                modified: 3,
                untracked: 1,
            }
        );
    }

    #[test]
    fn question_mark_only_suppresses_the_corresponding_column() {
        let status = parse_git_status("## main\n?M unusual-worktree\nM? unusual-index\n").unwrap();

        assert_eq!(status.staged, 1);
        assert_eq!(status.modified, 1);
        assert_eq!(status.untracked, 0);
    }

    #[test]
    fn ignores_lines_too_short_to_contain_a_porcelain_status() {
        let status = parse_git_status("## main\n\nX\n M valid\n").unwrap();

        assert_eq!(status.staged, 0);
        assert_eq!(status.modified, 1);
    }

    #[test]
    fn uses_detached_fallback_for_an_empty_branch_name() {
        assert_eq!(
            parse_git_status("##    \n"),
            Some(RepositoryStatus {
                branch: "detached".to_owned(),
                ..RepositoryStatus::default()
            })
        );
    }

    #[test]
    fn rejects_missing_or_malformed_branch_headers() {
        assert_eq!(parse_git_status(""), None);
        assert_eq!(parse_git_status("main\n M file\n"), None);
        assert_eq!(parse_git_status("# main\n M file\n"), None);
    }

    fn test_executable(name: &str) -> PathBuf {
        let mut executable_name = OsString::from(name);
        executable_name.push(env::consts::EXE_SUFFIX);
        let search_path = env::var_os("PATH");
        resolve_executable(search_path.as_deref(), &executable_name, is_executable_file)
            .unwrap_or_else(|| panic!("test executable was not found: {name}"))
    }

    fn process_exists(shell: &Path, pid: u32) -> bool {
        Command::new(shell)
            .args(["-c", "kill -0 \"$1\" 2>/dev/null", "mbx-test"])
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn stop_process(shell: &Path, pid: u32) {
        assert!(
            Command::new(shell)
                .args(["-c", "kill -KILL \"$1\"", "mbx-test"])
                .arg(pid.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success()),
            "failed to stop the descendant created by the timeout test"
        );
    }

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            loop {
                let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("mbx-provider-{}-{sequence}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("failed to create provider test directory: {error}"),
                }
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
