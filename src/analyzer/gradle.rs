use crate::core::context::StagedFile;

use super::{FileAnalyzer, ProjectMetadata};
use regex::Regex;
use std::collections::HashSet;

// Regex for checking dependency changes in Gradle diff
static GRADLE_DEP_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(implementation|api|testImplementation|compile)")
        .expect("Should compile: GRADLE_DEP_CHANGE_RE")
});
// Regex for checking plugin changes in Gradle diff
static GRADLE_PLUGIN_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(plugins|apply plugin)")
        .expect("Should compile: GRADLE_PLUGIN_CHANGE_RE")
});
// Regex for checking task changes in Gradle diff
static GRADLE_TASK_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*task\s+").expect("Should compile: GRADLE_TASK_CHANGE_RE")
});
// Regex for extracting Gradle project version
static GRADLE_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"version\s*=\s*['"](.*?)['"]"#).expect("Should compile: GRADLE_VERSION_RE")
});
// Regex for extracting Gradle dependencies
static GRADLE_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"implementation\s+['"](.+?):(.+?):(.+?)['"]"#)
        .expect("Should compile: GRADLE_DEPENDENCY_RE")
});
// Regex for extracting Gradle plugins
static GRADLE_PLUGIN_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"id\s+['"](.+?)['"]"#).expect("Should compile: GRADLE_PLUGIN_RE")
});

pub struct GradleAnalyzer;

impl FileAnalyzer for GradleAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if has_dependency_changes(&staged_file.diff) {
            analysis.push("Dependencies have been modified".to_string());
        }

        if has_plugin_changes(&staged_file.diff) {
            analysis.push("Plugins have been modified".to_string());
        }

        if has_task_changes(&staged_file.diff) {
            analysis.push("Tasks have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Gradle build file"
    }

    fn extract_metadata(&self, _file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some("Groovy/Kotlin".to_string()),
            ..Default::default()
        };
        metadata.build_system = Some("Gradle".to_string());

        if let Some(version) = extract_gradle_version(content) {
            metadata.version = Some(version);
        }

        if let Some(dependencies) = extract_gradle_dependencies(content) {
            metadata.dependencies = dependencies;
        }

        if let Some(plugins) = extract_gradle_plugins(content) {
            metadata.plugins = plugins;
        }

        metadata
    }
}

fn has_dependency_changes(diff: &str) -> bool {
    GRADLE_DEP_CHANGE_RE.is_match(diff)
}

fn has_plugin_changes(diff: &str) -> bool {
    GRADLE_PLUGIN_CHANGE_RE.is_match(diff)
}

fn has_task_changes(diff: &str) -> bool {
    GRADLE_TASK_CHANGE_RE.is_match(diff)
}

fn extract_gradle_version(content: &str) -> Option<String> {
    GRADLE_VERSION_RE
        .captures(content)
        .map(|cap| cap[1].to_string())
}

fn extract_gradle_dependencies(content: &str) -> Option<Vec<String>> {
    let dependencies: HashSet<String> = GRADLE_DEPENDENCY_RE
        .captures_iter(content)
        .map(|cap| format!("{}:{}:{}", &cap[1], &cap[2], &cap[3]))
        .collect();

    if dependencies.is_empty() {
        None
    } else {
        Some(dependencies.into_iter().collect())
    }
}

fn extract_gradle_plugins(content: &str) -> Option<Vec<String>> {
    let plugins: HashSet<String> = GRADLE_PLUGIN_RE
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect();

    if plugins.is_empty() {
        None
    } else {
        Some(plugins.into_iter().collect())
    }
}
