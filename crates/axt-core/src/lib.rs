//! Internal use only, no stability guarantees.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools
)]

use std::{
    env, fmt, fs,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

pub use anstream::ColorChoice;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{ArgGroup, Args, ValueEnum};
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use tempfile::NamedTempFile;
use thiserror::Error;
use time::OffsetDateTime;

/// Error type for shared core helpers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    /// The process current directory could not be read.
    #[error("failed to read current directory: {0}")]
    CurrentDirIo(String),

    /// The process current directory was not valid UTF-8.
    #[error("current directory is not valid UTF-8: {0:?}")]
    CurrentDirNotUtf8(PathBuf),

    /// More than one output mode was requested.
    #[error("output modes are mutually exclusive")]
    ConflictingOutputModes,

    /// A string did not match the standard error catalog.
    #[error("unknown error code: {0}")]
    UnknownErrorCode(String),

    /// A string did not match a supported output mode.
    #[error("unknown output mode: {0}")]
    UnknownOutputMode(String),

    /// A user-provided regex exceeded the configured pattern length.
    #[error("regex pattern exceeds maximum length of {max_len} bytes")]
    RegexTooLong { max_len: usize },
}

/// Output modes shared by all binaries.
///
/// The suite exposes four output modes:
/// `Human` for terminals, `Compact` for non-TTY pipelines by default,
/// `Agent` for explicit JSONL agent output, and `Json` for the canonical
/// envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable, TTY-aware output.
    Human,
    /// Compact text optimized for non-TTY pipelines and agent terminal capture.
    Compact,
    /// Standard JSON envelope.
    Json,
    /// Agent JSONL: summary record first, detail records after, schema-versioned.
    Agent,
}

/// Output schema formats shared by commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SchemaFormat {
    /// Human output description.
    Human,
    /// Compact text output description.
    Compact,
    /// JSON envelope schema.
    Json,
    /// Agent JSONL schema or description.
    Agent,
}

impl fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Human => "human",
            Self::Compact => "compact",
            Self::Json => "json",
            Self::Agent => "agent",
        })
    }
}

impl OutputMode {
    /// Resolve an output mode from explicit CLI flags.
    pub fn from_flags(json: bool, agent: bool) -> Result<Self, CoreError> {
        let selected = usize::from(json) + usize::from(agent);
        if selected > 1 {
            return Err(CoreError::ConflictingOutputModes);
        }
        if json {
            Ok(Self::Json)
        } else if agent {
            Ok(Self::Agent)
        } else {
            Ok(Self::Human)
        }
    }

    /// Resolve the effective mode after explicit flags, environment, and TTY.
    ///
    /// Precedence:
    /// 1. An explicit `--json` or `--agent` flag.
    /// 2. The `AXT_OUTPUT` environment variable (`human|compact|agent|json`).
    /// 3. Auto: `Compact` when stdout is not a terminal, otherwise `Human`.
    #[must_use]
    pub fn resolve(explicit: Self, env_value: Option<&str>, stdout_tty: bool) -> Self {
        if explicit != Self::Human {
            return explicit;
        }
        if let Some(raw) = env_value {
            if let Ok(parsed) = raw.parse::<Self>() {
                return parsed;
            }
        }
        if stdout_tty {
            Self::Human
        } else {
            Self::Compact
        }
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Human => "human",
            Self::Compact => "compact",
            Self::Json => "json",
            Self::Agent => "agent",
        })
    }
}

impl FromStr for OutputMode {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "human" => Ok(Self::Human),
            "compact" => Ok(Self::Compact),
            "json" => Ok(Self::Json),
            "agent" => Ok(Self::Agent),
            other => Err(CoreError::UnknownOutputMode(other.to_owned())),
        }
    }
}

/// Clap flags for shared output mode selection.
#[derive(Debug, Clone, Default, Args)]
#[command(group(ArgGroup::new("output_mode").args(["json", "agent"]).multiple(false)))]
pub struct OutputModeFlags {
    /// Emit the canonical JSON envelope.
    #[arg(long)]
    pub json: bool,

    /// Emit JSONL agent output (summary record first, detail records after).
    #[arg(long)]
    pub agent: bool,
}

/// Clap value parser for human-friendly durations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurationArg(pub Duration);

impl FromStr for DurationArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_duration(value).map(Self)
    }
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let split = value
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(value.len());
    if split == 0 {
        return Err(format!("missing numeric value: {value}"));
    }
    let amount = value[..split]
        .parse::<u64>()
        .map_err(|_| format!("invalid duration amount: {}", &value[..split]))?;
    let unit = &value[split..];
    let millis = match unit {
        "ms" => amount,
        "s" | "" => amount.saturating_mul(1_000),
        "m" => amount.saturating_mul(60_000),
        "h" => amount.saturating_mul(3_600_000),
        other => return Err(format!("unsupported duration unit: {other}")),
    };
    Ok(Duration::from_millis(millis))
}

impl OutputModeFlags {
    /// Resolve the explicit mode requested via flags.
    pub fn explicit_mode(&self) -> Result<OutputMode, CoreError> {
        OutputMode::from_flags(self.json, self.agent)
    }
}

/// Output limits shared by streaming renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputLimits {
    /// Maximum records to emit before truncating line-oriented output.
    pub max_records: usize,
    /// Maximum payload bytes to emit before truncating line-oriented output.
    pub max_bytes: usize,
    /// Treat truncation as a non-zero error.
    pub strict: bool,
}

/// Bounded byte tail optimized for chunked stream capture.
#[derive(Debug, Clone)]
pub struct BoundedTailBuffer {
    max: usize,
    bytes: Vec<u8>,
    start: usize,
    len: usize,
}

impl BoundedTailBuffer {
    /// Create a buffer retaining at most `max` trailing bytes.
    #[must_use]
    pub fn new(max: usize) -> Self {
        Self {
            max,
            bytes: vec![0; max],
            start: 0,
            len: 0,
        }
    }

    /// Push a chunk, retaining only the newest `max` bytes.
    pub fn push(&mut self, chunk: &[u8]) {
        if self.max == 0 || chunk.is_empty() {
            return;
        }
        if chunk.len() >= self.max {
            let keep = &chunk[chunk.len() - self.max..];
            self.bytes.copy_from_slice(keep);
            self.start = 0;
            self.len = self.max;
            return;
        }
        for byte in chunk {
            let write_at = (self.start + self.len) % self.max;
            self.bytes[write_at] = *byte;
            if self.len < self.max {
                self.len += 1;
            } else {
                self.start = (self.start + 1) % self.max;
            }
        }
    }

    /// Return retained bytes in chronological order.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        if self.len == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(self.len);
        let first = self.len.min(self.max - self.start);
        out.extend_from_slice(&self.bytes[self.start..self.start + first]);
        if first < self.len {
            out.extend_from_slice(&self.bytes[..self.len - first]);
        }
        out
    }

    /// Return retained bytes decoded lossily and split into lines.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        String::from_utf8_lossy(&self.bytes())
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }

    /// Return retained bytes decoded lossily as one string.
    #[must_use]
    pub fn text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes()).into_owned()
    }
}

/// Limits used when compiling user-provided regex patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserRegexLimits {
    /// Maximum pattern length in bytes before compilation is refused.
    pub max_pattern_len: usize,
    /// Regex heap size limit.
    pub size_limit: usize,
    /// Lazy DFA cache size limit.
    pub dfa_size_limit: usize,
}

impl UserRegexLimits {
    /// Conservative default for CLI search patterns.
    pub const DEFAULT_MAX_PATTERN_LEN: usize = 16 * 1024;
    /// Default compiled regex heap cap.
    pub const DEFAULT_SIZE_LIMIT: usize = 10 * 1024 * 1024;
    /// Default lazy DFA cache cap.
    pub const DEFAULT_DFA_SIZE_LIMIT: usize = 10 * 1024 * 1024;
}

impl Default for UserRegexLimits {
    fn default() -> Self {
        Self {
            max_pattern_len: Self::DEFAULT_MAX_PATTERN_LEN,
            size_limit: Self::DEFAULT_SIZE_LIMIT,
            dfa_size_limit: Self::DEFAULT_DFA_SIZE_LIMIT,
        }
    }
}

/// Compile a user-provided regex with explicit resource limits.
pub fn compile_user_regex(pattern: &str, limits: UserRegexLimits) -> Result<Regex, UserRegexError> {
    if pattern.len() > limits.max_pattern_len {
        return Err(CoreError::RegexTooLong {
            max_len: limits.max_pattern_len,
        }
        .into());
    }
    RegexBuilder::new(pattern)
        .size_limit(limits.size_limit)
        .dfa_size_limit(limits.dfa_size_limit)
        .build()
        .map_err(UserRegexError::Regex)
}

/// Error returned by [`compile_user_regex`].
#[derive(Debug, Error)]
pub enum UserRegexError {
    /// Shared core error.
    #[error("{0}")]
    Core(#[from] CoreError),
    /// Regex parser or compiler error.
    #[error("{0}")]
    Regex(#[from] regex::Error),
}

impl OutputLimits {
    /// Default maximum record count.
    pub const DEFAULT_MAX_RECORDS: usize = 200;
    /// Default maximum byte count.
    pub const DEFAULT_MAX_BYTES: usize = 65_536;
}

impl Default for OutputLimits {
    fn default() -> Self {
        Self {
            max_records: Self::DEFAULT_MAX_RECORDS,
            max_bytes: Self::DEFAULT_MAX_BYTES,
            strict: false,
        }
    }
}

/// Clap flags for shared output limits.
#[derive(Debug, Clone, Args)]
pub struct OutputLimitFlags {
    /// Maximum records to emit before truncating line-oriented output.
    #[arg(long = "limit", default_value_t = OutputLimits::DEFAULT_MAX_RECORDS, value_name = "N")]
    pub max_records: usize,

    /// Maximum payload bytes to emit before truncating line-oriented output.
    #[arg(long, default_value_t = OutputLimits::DEFAULT_MAX_BYTES, value_name = "BYTES")]
    pub max_bytes: usize,

    /// Exit non-zero when output truncation is required.
    #[arg(long)]
    pub strict: bool,
}

impl Default for OutputLimitFlags {
    fn default() -> Self {
        Self {
            max_records: OutputLimits::DEFAULT_MAX_RECORDS,
            max_bytes: OutputLimits::DEFAULT_MAX_BYTES,
            strict: false,
        }
    }
}

impl From<OutputLimitFlags> for OutputLimits {
    fn from(value: OutputLimitFlags) -> Self {
        Self {
            max_records: value.max_records,
            max_bytes: value.max_bytes,
            strict: value.strict,
        }
    }
}

impl From<&OutputLimitFlags> for OutputLimits {
    fn from(value: &OutputLimitFlags) -> Self {
        Self {
            max_records: value.max_records,
            max_bytes: value.max_bytes,
            strict: value.strict,
        }
    }
}

/// Shared flags every command supports.
#[derive(Debug, Clone, Default, Args)]
pub struct CommonArgs {
    /// Output mode flags.
    #[command(flatten)]
    pub output: OutputModeFlags,

    /// Output limit flags.
    #[command(flatten)]
    pub limits: OutputLimitFlags,

    /// Maximum wall-clock duration for commands that support bounded execution.
    #[arg(long, value_name = "DURATION")]
    pub max_duration: Option<DurationArg>,

    /// Print this command's output schema and exit.
    #[arg(long, value_name = "FORMAT", num_args = 0..=1, default_missing_value = "json")]
    pub print_schema: Option<SchemaFormat>,

    /// List the standard error catalog as JSONL and exit.
    #[arg(long)]
    pub list_errors: bool,
}

impl CommonArgs {
    /// Resolve the explicit output mode requested by CLI flags only.
    pub fn explicit_mode(&self) -> Result<OutputMode, CoreError> {
        self.output.explicit_mode()
    }

    /// Resolve the effective output mode, factoring in `AXT_OUTPUT` and TTY.
    pub fn mode(&self) -> Result<OutputMode, CoreError> {
        let explicit = self.output.explicit_mode()?;
        let env_value = env::var("AXT_OUTPUT").ok();
        Ok(OutputMode::resolve(
            explicit,
            env_value.as_deref(),
            stdout_is_terminal(),
        ))
    }

    /// Resolve the selected output limits.
    #[must_use]
    pub fn limits(&self) -> OutputLimits {
        OutputLimits::from(&self.limits)
    }

    /// Resolve the selected wall-clock duration bound, if any.
    #[must_use]
    pub fn max_duration(&self) -> Option<Duration> {
        self.max_duration.map(|duration| duration.0)
    }
}

/// Atomically write bytes to a file and fsync durable data before publishing it.
pub fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let parent = path.parent().unwrap_or_else(|| Utf8Path::new("."));
    let mut file = NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|err| err.error)?;
    #[cfg(unix)]
    {
        sync_parent_dir(parent)
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent_dir(parent: &Utf8Path) -> io::Result<()> {
    fs::File::open(parent)?.sync_all()
}

/// Standard error codes shared by all commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Success.
    Ok,
    /// Generic runtime failure.
    RuntimeError,
    /// CLI argument or flag invalid.
    UsageError,
    /// A required path does not exist.
    PathNotFound,
    /// Insufficient permissions.
    PermissionDenied,
    /// Operation exceeded `--timeout`.
    Timeout,
    /// `--strict` and output had to be truncated.
    OutputTruncatedStrict,
    /// SIGINT / Ctrl-C received.
    Interrupted,
    /// Filesystem or stream IO failure.
    IoError,
    /// Feature unavailable on this platform.
    FeatureUnsupported,
    /// Internal schema validation bug.
    SchemaViolation,
    /// Wrapped command exited non-zero.
    CommandFailed,
    /// Git repo expected but not found or readable.
    GitUnavailable,
    /// User config file malformed.
    ConfigError,
    /// Offline command attempted network.
    NetworkDisabled,
}

impl ErrorCode {
    /// Stable string form from the standard catalog.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::RuntimeError => "runtime_error",
            Self::UsageError => "usage_error",
            Self::PathNotFound => "path_not_found",
            Self::PermissionDenied => "permission_denied",
            Self::Timeout => "timeout",
            Self::OutputTruncatedStrict => "output_truncated_strict",
            Self::Interrupted => "interrupted",
            Self::IoError => "io_error",
            Self::FeatureUnsupported => "feature_unsupported",
            Self::SchemaViolation => "schema_violation",
            Self::CommandFailed => "command_failed",
            Self::GitUnavailable => "git_unavailable",
            Self::ConfigError => "config_error",
            Self::NetworkDisabled => "network_disabled",
        }
    }

    /// Process exit code from the standard catalog.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::RuntimeError => 1,
            Self::UsageError => 2,
            Self::PathNotFound => 3,
            Self::PermissionDenied => 4,
            Self::Timeout => 5,
            Self::OutputTruncatedStrict => 6,
            Self::Interrupted => 7,
            Self::IoError => 8,
            Self::FeatureUnsupported => 9,
            Self::SchemaViolation => 10,
            Self::CommandFailed => 11,
            Self::GitUnavailable => 12,
            Self::ConfigError => 13,
            Self::NetworkDisabled => 14,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl TryFrom<&str> for ErrorCode {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ok" => Ok(Self::Ok),
            "runtime_error" => Ok(Self::RuntimeError),
            "usage_error" => Ok(Self::UsageError),
            "path_not_found" => Ok(Self::PathNotFound),
            "permission_denied" => Ok(Self::PermissionDenied),
            "timeout" => Ok(Self::Timeout),
            "output_truncated_strict" => Ok(Self::OutputTruncatedStrict),
            "interrupted" => Ok(Self::Interrupted),
            "io_error" => Ok(Self::IoError),
            "feature_unsupported" => Ok(Self::FeatureUnsupported),
            "schema_violation" => Ok(Self::SchemaViolation),
            "command_failed" => Ok(Self::CommandFailed),
            "git_unavailable" => Ok(Self::GitUnavailable),
            "config_error" => Ok(Self::ConfigError),
            "network_disabled" => Ok(Self::NetworkDisabled),
            other => Err(CoreError::UnknownErrorCode(other.to_owned())),
        }
    }
}

/// Retryability marker from the standard error catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retryable {
    /// Not applicable.
    NotApplicable,
    /// Retry may help.
    Maybe,
    /// Retry is expected to help.
    Yes,
    /// Retry will not help.
    No,
    /// Retryability depends on the wrapped command.
    Depends,
}

impl Retryable {
    /// Stable string form used by `--list-errors`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "n/a",
            Self::Maybe => "maybe",
            Self::Yes => "yes",
            Self::No => "no",
            Self::Depends => "depends",
        }
    }
}

impl Serialize for Retryable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// One entry in the standard error catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ErrorCatalogEntry {
    /// Stable error code.
    pub code: ErrorCode,
    /// Process exit code.
    pub exit: u8,
    /// Human-readable meaning.
    pub meaning: &'static str,
    /// Retryability marker.
    pub retryable: Retryable,
}

/// Standard error catalog from `docs/spec.md` section 5.
pub const STANDARD_ERROR_CATALOG: &[ErrorCatalogEntry] = &[
    ErrorCatalogEntry {
        code: ErrorCode::Ok,
        exit: 0,
        meaning: "Success",
        retryable: Retryable::NotApplicable,
    },
    ErrorCatalogEntry {
        code: ErrorCode::RuntimeError,
        exit: 1,
        meaning: "Generic runtime failure",
        retryable: Retryable::Maybe,
    },
    ErrorCatalogEntry {
        code: ErrorCode::UsageError,
        exit: 2,
        meaning: "CLI argument or flag invalid",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::PathNotFound,
        exit: 3,
        meaning: "A required path does not exist",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::PermissionDenied,
        exit: 4,
        meaning: "Insufficient permissions",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::Timeout,
        exit: 5,
        meaning: "Operation exceeded --timeout",
        retryable: Retryable::Yes,
    },
    ErrorCatalogEntry {
        code: ErrorCode::OutputTruncatedStrict,
        exit: 6,
        meaning: "--strict and output had to be truncated",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::Interrupted,
        exit: 7,
        meaning: "SIGINT / Ctrl-C received",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::IoError,
        exit: 8,
        meaning: "Filesystem or stream IO failure",
        retryable: Retryable::Maybe,
    },
    ErrorCatalogEntry {
        code: ErrorCode::FeatureUnsupported,
        exit: 9,
        meaning: "Feature unavailable on this platform",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::SchemaViolation,
        exit: 10,
        meaning: "Internal: produced data violated its own schema",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::CommandFailed,
        exit: 11,
        meaning: "Wrapped command exited non-zero",
        retryable: Retryable::Depends,
    },
    ErrorCatalogEntry {
        code: ErrorCode::GitUnavailable,
        exit: 12,
        meaning: "Git repo expected but not found / not readable",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::ConfigError,
        exit: 13,
        meaning: "User config file malformed",
        retryable: Retryable::No,
    },
    ErrorCatalogEntry {
        code: ErrorCode::NetworkDisabled,
        exit: 14,
        meaning: "An offline command attempted network",
        retryable: Retryable::No,
    },
];

/// Clock abstraction used for deterministic timestamps.
pub trait Clock: fmt::Debug + Send + Sync {
    /// Return the current UTC timestamp.
    fn now_utc(&self) -> OffsetDateTime;
}

/// System-backed UTC clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Fixed UTC clock intended for tests.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    now: OffsetDateTime,
}

#[cfg(any(test, feature = "test-support"))]
impl FixedClock {
    /// Create a fixed clock at the provided timestamp.
    #[must_use]
    pub const fn new(now: OffsetDateTime) -> Self {
        Self { now }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Clock for FixedClock {
    fn now_utc(&self) -> OffsetDateTime {
        self.now
    }
}

/// Resolved user configuration for shared command contexts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedConfig {
    /// Path to the loaded config file, if any.
    pub path: Option<Utf8PathBuf>,
}

/// Shared command context every binary can build from common flags.
#[derive(Debug)]
pub struct CommandContext {
    /// Current working directory.
    pub cwd: Utf8PathBuf,
    /// Selected output mode.
    pub mode: OutputMode,
    /// Output limits.
    pub limits: OutputLimits,
    /// Resolved color choice.
    pub color: ColorChoice,
    /// Resolved configuration.
    pub config: ResolvedConfig,
    /// Shared wall-clock duration bound.
    pub max_duration: Option<Duration>,
    /// Clock for timestamp-producing operations.
    pub clock: Box<dyn Clock>,
}

impl CommandContext {
    /// Build a context from explicit parts.
    #[must_use]
    pub fn new(
        cwd: Utf8PathBuf,
        mode: OutputMode,
        limits: OutputLimits,
        color: ColorChoice,
        config: ResolvedConfig,
        max_duration: Option<Duration>,
        clock: Box<dyn Clock>,
    ) -> Self {
        Self {
            cwd,
            mode,
            limits,
            color,
            config,
            max_duration,
            clock,
        }
    }

    /// Build a context for the current process.
    pub fn from_common_args(common: &CommonArgs, clock: Box<dyn Clock>) -> Result<Self, CoreError> {
        let cwd = current_dir_utf8()?;
        Ok(Self::new(
            cwd,
            common.mode()?,
            common.limits(),
            resolve_color_choice(stdout_is_terminal()),
            ResolvedConfig::default(),
            common.max_duration(),
            clock,
        ))
    }
}

/// Read the current directory as a UTF-8 path.
pub fn current_dir_utf8() -> Result<Utf8PathBuf, CoreError> {
    let cwd = env::current_dir().map_err(|err| CoreError::CurrentDirIo(err.to_string()))?;
    Utf8PathBuf::from_path_buf(cwd).map_err(CoreError::CurrentDirNotUtf8)
}

/// Return whether stdout is connected to a terminal.
#[must_use]
pub fn stdout_is_terminal() -> bool {
    std::io::stdout().is_terminal()
}

/// Resolve color choice using the process environment.
#[must_use]
pub fn resolve_color_choice(stdout_tty: bool) -> ColorChoice {
    let no_color = env::var("NO_COLOR").ok();
    let clicolor_force = env::var("CLICOLOR_FORCE").ok();
    let force_color = env::var("FORCE_COLOR").ok();
    resolve_color_choice_from_env(
        stdout_tty,
        no_color.as_deref(),
        clicolor_force.as_deref(),
        force_color.as_deref(),
    )
}

/// Resolve color choice from explicit environment values.
#[must_use]
pub fn resolve_color_choice_from_env(
    stdout_tty: bool,
    no_color: Option<&str>,
    clicolor_force: Option<&str>,
    force_color: Option<&str>,
) -> ColorChoice {
    if is_set(no_color) {
        return ColorChoice::Never;
    }

    if is_truthy_force(clicolor_force) || is_truthy_force(force_color) {
        return ColorChoice::AlwaysAnsi;
    }

    if stdout_tty {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}

fn is_set(value: Option<&str>) -> bool {
    value.is_some_and(|inner| !inner.is_empty())
}

fn is_truthy_force(value: Option<&str>) -> bool {
    value.is_some_and(|inner| !inner.is_empty() && inner != "0")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serde_json::json;

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(flatten)]
        common: CommonArgs,
    }

    #[test]
    fn error_catalog_matches_spec_order_and_exits() {
        let expected = [
            ("ok", 0),
            ("runtime_error", 1),
            ("usage_error", 2),
            ("path_not_found", 3),
            ("permission_denied", 4),
            ("timeout", 5),
            ("output_truncated_strict", 6),
            ("interrupted", 7),
            ("io_error", 8),
            ("feature_unsupported", 9),
            ("schema_violation", 10),
            ("command_failed", 11),
            ("git_unavailable", 12),
            ("config_error", 13),
            ("network_disabled", 14),
        ];

        assert_eq!(STANDARD_ERROR_CATALOG.len(), expected.len());

        for (entry, (code, exit)) in STANDARD_ERROR_CATALOG.iter().zip(expected) {
            assert_eq!(entry.code.as_str(), code);
            assert_eq!(entry.exit, exit);
            assert_eq!(entry.code.exit_code(), exit);
        }
    }

    #[test]
    fn error_code_serializes_as_catalog_string() {
        let value = json!({ "code": ErrorCode::PathNotFound });
        assert_eq!(value, json!({ "code": "path_not_found" }));
    }

    #[test]
    fn output_modes_parse_from_clap_flags() -> Result<(), Box<dyn std::error::Error>> {
        let envelope_args = TestCli::try_parse_from(["test", "--json"])?;
        assert_eq!(envelope_args.common.explicit_mode()?, OutputMode::Json);

        let agent_args = TestCli::try_parse_from(["test", "--agent"])?;
        assert_eq!(agent_args.common.explicit_mode()?, OutputMode::Agent);

        let default_args = TestCli::try_parse_from(["test"])?;
        assert_eq!(default_args.common.explicit_mode()?, OutputMode::Human);

        Ok(())
    }

    #[test]
    fn clap_rejects_conflicting_output_modes() {
        let error = TestCli::try_parse_from(["test", "--json", "--agent"]);
        assert!(error.is_err());
    }

    #[test]
    fn resolve_uses_explicit_flag_when_present() {
        assert_eq!(
            OutputMode::resolve(OutputMode::Json, Some("agent"), false),
            OutputMode::Json
        );
        assert_eq!(
            OutputMode::resolve(OutputMode::Agent, None, true),
            OutputMode::Agent
        );
    }

    #[test]
    fn resolve_falls_back_to_env_then_tty() {
        assert_eq!(
            OutputMode::resolve(OutputMode::Human, Some("json"), true),
            OutputMode::Json
        );
        assert_eq!(
            OutputMode::resolve(OutputMode::Human, Some("compact"), true),
            OutputMode::Compact
        );
        assert_eq!(
            OutputMode::resolve(OutputMode::Human, Some("garbage"), false),
            OutputMode::Compact
        );
        assert_eq!(
            OutputMode::resolve(OutputMode::Human, None, false),
            OutputMode::Compact
        );
        assert_eq!(
            OutputMode::resolve(OutputMode::Human, None, true),
            OutputMode::Human
        );
    }

    #[test]
    fn output_limits_parse_defaults_and_overrides() -> Result<(), Box<dyn std::error::Error>> {
        let default_cli = TestCli::try_parse_from(["test"])?;
        assert_eq!(default_cli.common.limits(), OutputLimits::default());

        let custom_cli =
            TestCli::try_parse_from(["test", "--limit", "3", "--max-bytes", "99", "--strict"])?;
        assert_eq!(
            custom_cli.common.limits(),
            OutputLimits {
                max_records: 3,
                max_bytes: 99,
                strict: true,
            }
        );

        Ok(())
    }

    #[test]
    fn bounded_tail_buffer_keeps_latest_bytes_across_chunks() {
        let mut buffer = BoundedTailBuffer::new(5);
        buffer.push(b"abc");
        buffer.push(b"def");
        assert_eq!(buffer.bytes(), b"bcdef");

        buffer.push(b"ghijkl");
        assert_eq!(buffer.bytes(), b"hijkl");
    }

    #[test]
    fn bounded_tail_buffer_returns_lossy_lines() {
        let mut buffer = BoundedTailBuffer::new(9);
        buffer.push(b"one\ntwo\nthree");
        assert_eq!(buffer.lines(), vec!["two".to_owned(), "three".to_owned()]);
    }

    #[test]
    fn compile_user_regex_rejects_overlong_patterns() {
        let result = compile_user_regex(
            "abcdef",
            UserRegexLimits {
                max_pattern_len: 3,
                ..UserRegexLimits::default()
            },
        );

        assert!(matches!(
            result,
            Err(UserRegexError::Core(CoreError::RegexTooLong { max_len: 3 }))
        ));
    }

    #[test]
    fn print_schema_accepts_optional_format() -> Result<(), Box<dyn std::error::Error>> {
        let default_schema = TestCli::try_parse_from(["test", "--print-schema"])?;
        assert_eq!(default_schema.common.print_schema, Some(SchemaFormat::Json));

        let agent_schema = TestCli::try_parse_from(["test", "--print-schema", "agent"])?;
        assert_eq!(agent_schema.common.print_schema, Some(SchemaFormat::Agent));

        let compact_schema = TestCli::try_parse_from(["test", "--print-schema", "compact"])?;
        assert_eq!(
            compact_schema.common.print_schema,
            Some(SchemaFormat::Compact)
        );

        Ok(())
    }

    #[test]
    fn fixed_clock_returns_injected_time() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let clock = FixedClock::new(now);
        assert_eq!(clock.now_utc(), now);
    }

    #[test]
    fn color_resolution_honors_documented_precedence() {
        assert_eq!(
            resolve_color_choice_from_env(true, Some("1"), Some("1"), Some("1")),
            ColorChoice::Never
        );
        assert_eq!(
            resolve_color_choice_from_env(false, None, Some("1"), None),
            ColorChoice::AlwaysAnsi
        );
        assert_eq!(
            resolve_color_choice_from_env(false, None, Some("0"), Some("1")),
            ColorChoice::AlwaysAnsi
        );
        assert_eq!(
            resolve_color_choice_from_env(true, None, None, None),
            ColorChoice::Auto
        );
        assert_eq!(
            resolve_color_choice_from_env(false, None, None, None),
            ColorChoice::Never
        );
    }
}
