use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestData {
    pub frameworks: Vec<String>,
    pub suites: Vec<TestSuite>,
    pub cases: Vec<TestCase>,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub todo: usize,
    pub duration_ms: u64,
    pub started: String,
    pub truncated: bool,
}

impl TestData {
    #[must_use]
    pub const fn ok(&self) -> bool {
        self.failed == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSuite {
    pub framework: String,
    pub name: String,
    pub file: Option<Utf8PathBuf>,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub todo: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCase {
    pub framework: String,
    pub status: TestStatus,
    pub name: String,
    pub suite: Option<String>,
    pub file: Option<Utf8PathBuf>,
    pub line: Option<u64>,
    pub duration_ms: u64,
    pub failure: Option<TestFailure>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parser_defaulted_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Todo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFailure {
    pub message: String,
    pub stack: Option<String>,
    pub actual: Option<String>,
    pub expected: Option<String>,
    pub diff: Option<String>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NormalizedEvent {
    Suite(TestSuite),
    Case(TestCase),
    Summary(TestSummary),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSummary {
    pub frameworks: Vec<String>,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub todo: usize,
    pub duration_ms: u64,
    pub started: String,
    pub truncated: bool,
}

impl From<&TestData> for TestSummary {
    fn from(data: &TestData) -> Self {
        Self {
            frameworks: data.frameworks.clone(),
            total: data.total,
            passed: data.passed,
            failed: data.failed,
            skipped: data.skipped,
            todo: data.todo,
            duration_ms: data.duration_ms,
            started: data.started.clone(),
            truncated: data.truncated,
        }
    }
}
