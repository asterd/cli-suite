use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

use crate::cli::FrameworkArg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: Utf8PathBuf,
    pub framework: FrameworkArg,
}

pub fn detect_projects(root: &Utf8Path, forced: Option<FrameworkArg>) -> Vec<Project> {
    if let Some(framework) = forced {
        return vec![Project {
            root: root.to_path_buf(),
            framework,
        }];
    }

    if let Some(framework) = configured_framework(root) {
        return vec![project(root, framework)];
    }

    let mut projects = Vec::new();
    detect_in_dir(root, &mut projects);

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if matches!(
                name,
                ".git" | "target" | "node_modules" | ".venv" | "venv" | "dist"
            ) {
                continue;
            }
            if path.is_dir() {
                if let Ok(child) = Utf8PathBuf::from_path_buf(path) {
                    detect_in_dir(&child, &mut projects);
                }
            }
        }
    }

    projects.sort_by(|left, right| {
        left.root
            .cmp(&right.root)
            .then_with(|| left.framework.as_str().cmp(right.framework.as_str()))
    });
    projects.dedup();
    projects
}

pub fn framework_rows() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "jest",
            "package.json",
            "scripts.test or devDependencies mention jest",
        ),
        (
            "vitest",
            "package.json",
            "scripts.test or devDependencies mention vitest",
        ),
        (
            "pytest",
            "pyproject.toml",
            "[tool.pytest] or pytest dependency marker",
        ),
        ("cargo", "Cargo.toml", "Rust package or workspace manifest"),
        ("go", "go.mod", "Go module manifest"),
        (
            "bun",
            "package.json",
            "scripts.test or devDependencies mention bun",
        ),
        ("deno", "deno.json", "Deno project manifest"),
    ]
}

fn detect_in_dir(dir: &Utf8Path, projects: &mut Vec<Project>) {
    if let Some(framework) = configured_framework(dir) {
        projects.push(project(dir, framework));
        return;
    }
    if has_file(dir, "deno.json") {
        projects.push(project(dir, FrameworkArg::Deno));
    }
    if has_file(dir, "go.mod") {
        projects.push(project(dir, FrameworkArg::Go));
    }
    if has_file(dir, "Cargo.toml") {
        projects.push(project(dir, FrameworkArg::Cargo));
    }
    if pyproject_mentions_pytest(dir) {
        projects.push(project(dir, FrameworkArg::Pytest));
    }
    for framework in package_frameworks(dir) {
        projects.push(project(dir, framework));
    }
}

fn project(root: &Utf8Path, framework: FrameworkArg) -> Project {
    Project {
        root: root.to_path_buf(),
        framework,
    }
}

fn has_file(dir: &Utf8Path, name: &str) -> bool {
    dir.join(name).is_file()
}

fn configured_framework(dir: &Utf8Path) -> Option<FrameworkArg> {
    framework_from_axt_toml(&dir.join("axt-test.toml"))
        .or_else(|| framework_from_pyproject(&dir.join("pyproject.toml")))
        .or_else(|| framework_from_package_json(&dir.join("package.json")))
}

fn framework_from_axt_toml(path: &Utf8Path) -> Option<FrameworkArg> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| framework_from_toml_text(&text, None))
}

fn framework_from_pyproject(path: &Utf8Path) -> Option<FrameworkArg> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| framework_from_toml_text(&text, Some("[tool.axt-test]")))
}

fn framework_from_toml_text(text: &str, section: Option<&str>) -> Option<FrameworkArg> {
    let mut in_section = section.is_none();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(section_name) = section {
            if trimmed.starts_with('[') {
                in_section = trimmed == section_name;
            }
        }
        if in_section && trimmed.starts_with("framework") {
            let (_, value) = trimmed.split_once('=')?;
            return parse_framework(value.trim().trim_matches('"').trim_matches('\''));
        }
    }
    None
}

fn framework_from_package_json(path: &Utf8Path) -> Option<FrameworkArg> {
    let text = fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<Value>(&text).ok()?;
    value
        .get("axt-test")
        .and_then(|config| config.get("framework"))
        .and_then(Value::as_str)
        .and_then(parse_framework)
}

fn parse_framework(value: &str) -> Option<FrameworkArg> {
    match value {
        "jest" => Some(FrameworkArg::Jest),
        "vitest" => Some(FrameworkArg::Vitest),
        "pytest" => Some(FrameworkArg::Pytest),
        "cargo" | "cargo-test" | "cargo test" => Some(FrameworkArg::Cargo),
        "go" | "go-test" | "go test" => Some(FrameworkArg::Go),
        "bun" => Some(FrameworkArg::Bun),
        "deno" => Some(FrameworkArg::Deno),
        _ => None,
    }
}

fn pyproject_mentions_pytest(dir: &Utf8Path) -> bool {
    let path = dir.join("pyproject.toml");
    fs::read_to_string(path)
        .is_ok_and(|text| text.contains("pytest") || text.contains("[tool.axt-test]"))
}

fn package_frameworks(dir: &Utf8Path) -> Vec<FrameworkArg> {
    let path = dir.join("package.json");
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let blob = value.to_string();
    let mut frameworks = Vec::new();
    if blob.contains("vitest") {
        frameworks.push(FrameworkArg::Vitest);
    }
    if blob.contains("jest") {
        frameworks.push(FrameworkArg::Jest);
    }
    if blob.contains("bun") {
        frameworks.push(FrameworkArg::Bun);
    }
    frameworks
}
