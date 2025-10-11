use crate::core::context::StagedFile;

use super::{FileAnalyzer, ProjectMetadata};
use regex::Regex;
use std::collections::HashSet;

// Regex for extracting makefile version
static MAKEFILE_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"VERSION\s*=\s*([^\s]+)").expect("Should compile: MAKEFILE_VERSION_RE")
});
// Regex for extracting makefile dependencies
static MAKEFILE_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"LIBS\s*\+=\s*([^\s]+)").expect("Should compile: MAKEFILE_DEPENDENCY_RE")
});
// Regex for extracting modified C functions
static C_FUNCTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(?:static\s+)?(?:inline\s+)?(?:const\s+)?(?:volatile\s+)?(?:unsigned\s+)?(?:signed\s+)?(?:short\s+)?(?:long\s+)?(?:void|int|char|float|double|struct\s+\w+)\s+(\w+)\s*\(")
        .expect("Should compile: C_FUNCTION_RE")
});
// Regex for extracting modified C structs
static C_STRUCT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*struct\s+(\w+)").expect("Should compile: C_STRUCT_RE")
});
// Regex for checking C include changes
static C_INCLUDE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*#include").expect("Should compile: C_INCLUDE_RE")
});

pub struct CAnalyzer;

impl FileAnalyzer for CAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(structs) = extract_modified_structs(&staged_file.diff) {
            analysis.push(format!("Modified structs: {}", structs.join(", ")));
        }

        if has_include_changes(&staged_file.diff) {
            analysis.push("Include statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "C source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata: ProjectMetadata = ProjectMetadata {
            language: Some("C".to_string()),
            ..Default::default()
        };

        if file == "Makefile" {
            Self::extract_makefile_metadata(content, &mut metadata);
        } else {
            Self::extract_c_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl CAnalyzer {
    fn extract_makefile_metadata(content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("Makefile".to_string());

        if let Some(cap) = MAKEFILE_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        for cap in MAKEFILE_DEPENDENCY_RE.captures_iter(content) {
            metadata.dependencies.push(cap[1].to_string());
        }
    }

    fn extract_c_file_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("#include <stdio.h>") {
            metadata.framework = Some("Standard I/O".to_string());
        }

        if content.contains("#include <stdlib.h>") {
            metadata.framework = Some("Standard Library".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let functions: HashSet<String> = C_FUNCTION_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions.into_iter().collect())
    }
}

fn extract_modified_structs(diff: &str) -> Option<Vec<String>> {
    let structs: HashSet<String> = C_STRUCT_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if structs.is_empty() {
        None
    } else {
        Some(structs.into_iter().collect())
    }
}

fn has_include_changes(diff: &str) -> bool {
    C_INCLUDE_RE.is_match(diff)
}
