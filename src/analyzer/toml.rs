use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

// Regex for extracting dependency names in TOML
static TOML_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(?:dependencies|dev-dependencies|\[dependencies\]|\[dev-dependencies\]|\[\w+\.dependencies\])")
        .expect("Should compile: TOML_DEPENDENCY_RE")
});

// Regex for extracting package versions
static TOML_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"(?m)^[+-]\s*version\s*=\s*["'](.+?)["']"#)
        .expect("Should compile: TOML_VERSION_RE")
});

// Regex for extracting section headers
static TOML_SECTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*\[(.+?)\]").expect("Should compile: TOML_SECTION_RE")
});

/// Analyzer for TOML configuration files
pub struct TomlAnalyzer;

impl FileAnalyzer for TomlAnalyzer {
    fn analyze(&self, file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        // Check for dependency changes
        if TOML_DEPENDENCY_RE.is_match(&staged_file.diff) {
            analysis.push("Dependencies have been updated".to_string());
        }

        // Check for version changes
        if let Some(caps) = TOML_VERSION_RE.captures(&staged_file.diff) {
            analysis.push(format!("Version updated to {}", &caps[1]));
        }

        // Extract modified sections
        let sections: HashSet<String> = TOML_SECTION_RE
            .captures_iter(&staged_file.diff)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();

        if !sections.is_empty() {
            let sections_list: Vec<String> = sections.into_iter().collect();
            analysis.push(format!("Modified sections: {}", sections_list.join(", ")));
        }

        // Special handling for specific TOML files
        if file.ends_with("Cargo.toml") {
            analysis.push("Rust project configuration changed".to_string());
        } else if file.ends_with("pyproject.toml") {
            analysis.push("Python project configuration changed".to_string());
        } else if file.contains(".github") && file.contains("workflow") {
            analysis.push("GitHub workflow configuration changed".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "TOML configuration file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        // Cargo.toml indicates a Rust project
        if file.ends_with("Cargo.toml") {
            metadata.language = Some("Rust".to_string());

            // Extract version
            if let Some(caps) = Regex::new(r#"(?m)^version\s*=\s*["'](.+?)["']"#)
                .ok()
                .and_then(|re| re.captures(content))
            {
                metadata.version = Some(caps[1].to_string());
            }

            // Extract dependencies
            if let Some(deps_section) = Regex::new(r"(?ms)\[dependencies\](.*?)(\[|\z)")
                .ok()
                .and_then(|re| re.captures(content))
            {
                let deps_content = &deps_section[1];
                if let Ok(dep_re) = Regex::new(r"(?m)^(\w+)\s*=") {
                    for cap in dep_re.captures_iter(deps_content) {
                        metadata.dependencies.push(cap[1].to_string());
                    }
                }
            }
        }
        // pyproject.toml indicates a Python project
        else if file.ends_with("pyproject.toml") {
            metadata.language = Some("Python".to_string());

            // Extract Python build system
            if content.contains("[build-system]") {
                if content.contains("poetry") {
                    metadata.build_system = Some("Poetry".to_string());
                } else if content.contains("setuptools") {
                    metadata.build_system = Some("Setuptools".to_string());
                } else if content.contains("flit") {
                    metadata.build_system = Some("Flit".to_string());
                }
            }
        }
        // Any other TOML file
        else {
            metadata.language = Some("TOML Configuration".to_string());
        }

        metadata
    }
}
