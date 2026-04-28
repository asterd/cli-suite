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

use std::{env, fmt, io::IsTerminal, path::PathBuf, str::FromStr};

pub use anstream::ColorChoice;
use camino::Utf8PathBuf;
use clap::{ArgGroup, Args, ValueEnum};
use serde::Serialize;
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
}

/// Output modes shared by all binaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable, TTY-aware output.
    Human,
    /// Standard JSON envelope.
    Json,
    /// The JSON envelope's `data` payload only.
    JsonData,
    /// Newline-delimited JSON for streaming and pipelines.
    Jsonl,
    /// Agent Compact Format.
    Agent,
    /// Human-readable output without color or decorations.
    Plain,
}

/// Output schema formats shared by commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SchemaFormat {
    /// Human output schema or description.
    Human,
    /// JSON envelope schema.
    Json,
    /// JSONL record schema or description.
    Jsonl,
    /// Agent Compact Format schema or description.
    Agent,
}

impl fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Human => "human",
            Self::Json => "json",
            Self::Jsonl => "jsonl",
            Self::Agent => "agent",
        })
    }
}

impl OutputMode {
    /// Resolve an output mode from raw boolean flags.
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn from_flags(
        json: bool,
        json_data: bool,
        jsonl: bool,
        agent: bool,
        plain: bool,
    ) -> Result<Self, CoreError> {
        let selected = usize::from(json)
            + usize::from(json_data)
            + usize::from(jsonl)
            + usize::from(agent)
            + usize::from(plain);

        if selected > 1 {
            return Err(CoreError::ConflictingOutputModes);
        }

        if json {
            Ok(Self::Json)
        } else if json_data {
            Ok(Self::JsonData)
        } else if jsonl {
            Ok(Self::Jsonl)
        } else if agent {
            Ok(Self::Agent)
        } else if plain {
            Ok(Self::Plain)
        } else {
            Ok(Self::Human)
        }
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Human => "human",
            Self::Json => "json",
            Self::JsonData => "json-data",
            Self::Jsonl => "jsonl",
            Self::Agent => "agent",
            Self::Plain => "plain",
        })
    }
}

impl FromStr for OutputMode {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            "json-data" => Ok(Self::JsonData),
            "jsonl" => Ok(Self::Jsonl),
            "agent" => Ok(Self::Agent),
            "plain" => Ok(Self::Plain),
            other => Err(CoreError::UnknownOutputMode(other.to_owned())),
        }
    }
}

/// Clap flags for shared output mode selection.
#[derive(Debug, Clone, Default, Args)]
#[command(group(ArgGroup::new("output_mode").args(["json", "json_data", "jsonl", "agent", "plain"]).multiple(false)))]
pub struct OutputModeFlags {
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,

    /// Emit only the JSON data payload.
    #[arg(long)]
    pub json_data: bool,

    /// Emit newline-delimited JSON.
    #[arg(long)]
    pub jsonl: bool,

    /// Emit Agent Compact Format.
    #[arg(long)]
    pub agent: bool,

    /// Emit plain human-readable output.
    #[arg(long)]
    pub plain: bool,
}

impl OutputModeFlags {
    /// Resolve the selected output mode.
    pub fn mode(&self) -> Result<OutputMode, CoreError> {
        OutputMode::from_flags(
            self.json,
            self.json_data,
            self.jsonl,
            self.agent,
            self.plain,
        )
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

    /// Print this command's output schema and exit.
    #[arg(long, value_name = "FORMAT", num_args = 0..=1, default_missing_value = "json")]
    pub print_schema: Option<SchemaFormat>,

    /// List the standard error catalog as JSONL and exit.
    #[arg(long)]
    pub list_errors: bool,
}

impl CommonArgs {
    /// Resolve the selected output mode.
    pub fn mode(&self) -> Result<OutputMode, CoreError> {
        self.output.mode()
    }

    /// Resolve the selected output limits.
    #[must_use]
    pub fn limits(&self) -> OutputLimits {
        OutputLimits::from(&self.limits)
    }
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
        clock: Box<dyn Clock>,
    ) -> Self {
        Self {
            cwd,
            mode,
            limits,
            color,
            config,
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
        assert_eq!(envelope_args.common.mode()?, OutputMode::Json);

        let data_args = TestCli::try_parse_from(["test", "--json-data"])?;
        assert_eq!(data_args.common.mode()?, OutputMode::JsonData);

        let stream_args = TestCli::try_parse_from(["test", "--jsonl"])?;
        assert_eq!(stream_args.common.mode()?, OutputMode::Jsonl);

        let acf_args = TestCli::try_parse_from(["test", "--agent"])?;
        assert_eq!(acf_args.common.mode()?, OutputMode::Agent);

        let plain_args = TestCli::try_parse_from(["test", "--plain"])?;
        assert_eq!(plain_args.common.mode()?, OutputMode::Plain);

        let default_args = TestCli::try_parse_from(["test"])?;
        assert_eq!(default_args.common.mode()?, OutputMode::Human);

        Ok(())
    }

    #[test]
    fn clap_rejects_conflicting_output_modes() {
        let error = TestCli::try_parse_from(["test", "--json", "--agent"]);
        assert!(error.is_err());
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
    fn print_schema_accepts_optional_format() -> Result<(), Box<dyn std::error::Error>> {
        let default_schema = TestCli::try_parse_from(["test", "--print-schema"])?;
        assert_eq!(default_schema.common.print_schema, Some(SchemaFormat::Json));

        let agent_schema = TestCli::try_parse_from(["test", "--print-schema", "agent"])?;
        assert_eq!(agent_schema.common.print_schema, Some(SchemaFormat::Agent));

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
