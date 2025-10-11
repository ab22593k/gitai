use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

// Regex for extracting modified JS/TS functions (function keyword or const arrow func)
static JS_FUNCTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(function\s+(\w+)|const\s+(\w+)\s*=\s*(\([^)]*\)\s*=>|\function))")
        .expect("Should compile: JS_FUNCTION_RE")
});
// Regex for extracting modified JS/TS classes
static JS_CLASS_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*class\s+(\w+)").expect("Should compile: JS_CLASS_RE")
});
// Regex for checking JS/TS import/export changes
static JS_IMPORT_EXPORT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(import|export)").expect("Should compile: JS_IMPORT_EXPORT_RE")
});
// Regex for extracting modified React class components
static REACT_CLASS_COMPONENT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*class\s+(\w+)\s+extends\s+React\.Component")
        .expect("Should compile: REACT_CLASS_COMPONENT_RE")
});
// Regex for extracting modified React functional components
static REACT_FUNC_COMPONENT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(?:function\s+(\w+)|const\s+(\w+)\s*=)(?:\s*\([^)]*\))?\s*(?:=>)?\s*(?:\{[^}]*return|=>)\s*(?:<|\()").expect("Should compile: REACT_FUNC_COMPONENT_RE")
});

pub struct JavaScriptAnalyzer;

impl FileAnalyzer for JavaScriptAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        if let Some(components) = extract_modified_react_components(&staged_file.diff) {
            analysis.push(format!(
                "Modified React components: {}",
                components.join(", ")
            ));
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "JavaScript/TypeScript source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some(
                if file.to_lowercase().ends_with(".ts") {
                    "TypeScript"
                } else {
                    "JavaScript"
                }
                .to_string(),
            ),
            ..Default::default()
        };

        if file == "package.json" {
            Self::extract_package_json_metadata(content, &mut metadata);
        } else {
            Self::extract_js_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl JavaScriptAnalyzer {
    fn extract_package_json_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(version) = json["version"].as_str() {
                metadata.version = Some(version.to_string());
            }

            if let Some(dependencies) = json["dependencies"].as_object() {
                for dep in dependencies.keys() {
                    metadata.dependencies.push(dep.to_string());
                }
            }

            if let Some(dev_dependencies) = json["devDependencies"].as_object() {
                for dep in dev_dependencies.keys() {
                    if dep.contains("test") || dep.contains("jest") || dep.contains("mocha") {
                        metadata.test_framework = Some(dep.to_string());
                        break;
                    }
                }
            }

            metadata.build_system = Some("npm".to_string());
        }
    }

    fn extract_js_file_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("import React") || content.contains("from 'react'") {
            metadata.framework = Some("React".to_string());
        } else if content.contains("import Vue") || content.contains("from 'vue'") {
            metadata.framework = Some("Vue".to_string());
        } else if content.contains("import { Component") || content.contains("from '@angular/core'")
        {
            metadata.framework = Some("Angular".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let functions: Vec<String> = JS_FUNCTION_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).or(cap.get(3)).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let classes: Vec<String> = JS_CLASS_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes)
    }
}

fn has_import_changes(diff: &str) -> bool {
    JS_IMPORT_EXPORT_RE.is_match(diff)
}

fn extract_modified_react_components(diff: &str) -> Option<Vec<String>> {
    let mut components = HashSet::new();

    for cap in REACT_CLASS_COMPONENT_RE.captures_iter(diff) {
        if let Some(m) = cap.get(1) {
            components.insert(m.as_str().to_string());
        }
    }

    for cap in REACT_FUNC_COMPONENT_RE.captures_iter(diff) {
        if let Some(m) = cap.get(1).or(cap.get(2)) {
            components.insert(m.as_str().to_string());
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components.into_iter().collect())
    }
}
