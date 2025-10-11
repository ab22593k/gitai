use regex::Regex;
use std::collections::HashSet;

use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;

// Regex for extracting CMake project version
static CMAKE_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"project\([^)]+\s+VERSION\s+([^\s)]+)").expect("Should compile: CMAKE_VERSION_RE")
});
// Regex for extracting CMake dependencies (find_package)
static CMAKE_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"find_package\(([^)]+)\)").expect("Should compile: CMAKE_DEPENDENCY_RE")
});
// Regex for extracting modified C++ functions
static CPP_FUNCTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(?:static\s+)?(?:inline\s+)?(?:const\s+)?(?:volatile\s+)?(?:unsigned\s+)?(?:signed\s+)?(?:short\s+)?(?:long\s+)?(?:void|int|char|float|double|struct\s+\w+|class\s+\w+)\s+(\w+)\s*\(")
        .expect("Should compile: CPP_FUNCTION_RE")
});
// Regex for extracting modified C++ classes
static CPP_CLASS_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*class\s+(\w+)").expect("Should compile: CPP_CLASS_RE")
});
// Regex for checking C++ include changes
static CPP_INCLUDE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*#include").expect("Should compile: CPP_INCLUDE_RE")
});

pub struct CppAnalyzer;

impl FileAnalyzer for CppAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if has_include_changes(&staged_file.diff) {
            analysis.push("Include statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "C++ source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some("C++".to_string()),
            ..Default::default()
        };

        if file == "CMakeLists.txt" {
            Self::extract_cmake_metadata(content, &mut metadata);
        } else {
            Self::extract_cpp_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl CppAnalyzer {
    fn extract_cmake_metadata(content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("CMake".to_string());

        if let Some(cap) = CMAKE_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        for cap in CMAKE_DEPENDENCY_RE.captures_iter(content) {
            let package = cap[1].split(' ').next().unwrap_or(&cap[1]);
            metadata.dependencies.push(package.to_string());
        }
    }

    fn extract_cpp_file_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("#include <iostream>") {
            metadata.framework = Some("Standard I/O".to_string());
        }

        if content.contains("#include <vector>") {
            metadata.framework = Some("Standard Library".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let functions: HashSet<String> = CPP_FUNCTION_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions.into_iter().collect())
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let classes: HashSet<String> = CPP_CLASS_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes.into_iter().collect())
    }
}

fn has_include_changes(diff: &str) -> bool {
    CPP_INCLUDE_RE.is_match(diff)
}
