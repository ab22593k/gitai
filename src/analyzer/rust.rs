use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;

// Regex for extracting version from Cargo.toml
static CARGO_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"version\s*=\s*"([^"]+)""#).expect("Should compile: CARGO_VERSION_RE")
});
// Regex for finding the [dependencies] section in Cargo.toml
static CARGO_DEPS_SECTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^\[dependencies\](?:\s*\n(?:.*\s*=\s*.*)*)")
        .expect("Should compile: CARGO_DEPS_SECTION_RE")
});
// Regex for extracting modified Rust functions
static RUST_FN_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*(?:pub\s+)?fn\s+(\w+)").expect("Should compile: RUST_FN_RE")
});
// Regex for extracting modified Rust structs
static RUST_STRUCT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*(?:pub\s+)?struct\s+(\w+)").expect("Should compile: RUST_STRUCT_RE")
});
// Regex for extracting modified Rust traits
static RUST_TRAIT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*(?:pub\s+)?trait\s+(\w+)").expect("Should compile: RUST_TRAIT_RE")
});
// Regex for checking Rust import changes (use/extern crate)
static RUST_IMPORT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*(use|extern crate)").expect("Should compile: RUST_IMPORT_RE")
});

/// Analyzer for Rust source files
pub struct RustAnalyzer;

impl FileAnalyzer for RustAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        // Check for new or modified functions
        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        // Check for new or modified structs
        if let Some(structs) = extract_modified_structs(&staged_file.diff) {
            analysis.push(format!("Modified structs: {}", structs.join(", ")));
        }

        // Check for new or modified traits
        if let Some(traits) = extract_modified_traits(&staged_file.diff) {
            analysis.push(format!("Modified traits: {}", traits.join(", ")));
        }

        // Check for new or modified imports
        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Rust source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some("Rust".to_string()),
            ..Default::default()
        };

        if file == "Cargo.toml" {
            Self::extract_cargo_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl RustAnalyzer {
    fn extract_cargo_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if let Some(cap) = CARGO_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        if let Some(deps_section) = CARGO_DEPS_SECTION_RE.find(content) {
            let deps_lines = deps_section.as_str().lines().skip(1);
            for line in deps_lines {
                if let Some(dep_name) = line.split('=').next() {
                    metadata.dependencies.push(dep_name.trim().to_string());
                }
            }
        }

        if content.contains("rocket") {
            metadata.framework = Some("Rocket".to_string());
        } else if content.contains("actix") {
            metadata.framework = Some("Actix".to_string());
        }

        metadata.build_system = Some("Cargo".to_string());

        if content.contains("[dev-dependencies]")
            && (content.contains("\"test\"") || content.contains("'test'"))
        {
            metadata.test_framework = Some("built-in".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let functions: Vec<String> = RUST_FN_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn extract_modified_structs(diff: &str) -> Option<Vec<String>> {
    let structs: Vec<String> = RUST_STRUCT_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if structs.is_empty() {
        None
    } else {
        Some(structs)
    }
}

fn extract_modified_traits(diff: &str) -> Option<Vec<String>> {
    let traits: Vec<String> = RUST_TRAIT_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if traits.is_empty() {
        None
    } else {
        Some(traits)
    }
}

fn has_import_changes(diff: &str) -> bool {
    RUST_IMPORT_RE.is_match(diff)
}
