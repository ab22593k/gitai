use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use crate::debug;
use regex::Regex;
use std::collections::HashSet;

// Regex for extracting modified top-level JSON keys
static JSON_TOP_LEVEL_KEY_RE: std::sync::LazyLock<Result<Regex, regex::Error>> =
    std::sync::LazyLock::new(|| Regex::new(r#"^[+-]\s*"(\w+)"\s*:"#));
// Regex for checking JSON array changes
static JSON_ARRAY_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"(?m)^[+-]\s*(?:"[^"]+"\s*:\s*)?\[|\s*[+-]\s*"[^"]+","#)
        .expect("Should compile: JSON_ARRAY_CHANGE_RE")
});
// Regex for checking nested JSON object changes
static JSON_NESTED_OBJECT_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"(?m)^[+-]\s*"[^"]+"\s*:\s*\{|\s*[+-]\s*"[^"]+"\s*:"#)
        .expect("Should compile: JSON_NESTED_OBJECT_CHANGE_RE")
});

pub struct JsonAnalyzer;

impl FileAnalyzer for JsonAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(keys) = extract_modified_top_level_keys(&staged_file.diff) {
            analysis.push(format!("Modified top-level keys: {}", keys.join(", ")));
        }

        if has_array_changes(&staged_file.diff) {
            analysis.push("Array structures have been modified".to_string());
        }

        if has_nested_object_changes(&staged_file.diff) {
            analysis.push("Nested objects have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "JSON configuration file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        if file == "package.json" {
            Self::extract_package_json_metadata(content, &mut metadata);
        } else if file == "tsconfig.json" {
            metadata.language = Some("TypeScript".to_string());
        }

        metadata
    }
}

impl JsonAnalyzer {
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
                    metadata.dependencies.push(dep.to_string());
                }
            }

            metadata.language = Some("JavaScript".to_string());
            metadata.build_system = Some("npm".to_string());

            // Detect framework
            if json["dependencies"].get("react").is_some() {
                metadata.framework = Some("React".to_string());
            } else if json["dependencies"].get("vue").is_some() {
                metadata.framework = Some("Vue".to_string());
            } else if json["dependencies"].get("@angular/core").is_some() {
                metadata.framework = Some("Angular".to_string());
            }

            // Detect test framework
            if json["devDependencies"].get("jest").is_some() {
                metadata.test_framework = Some("Jest".to_string());
            } else if json["devDependencies"].get("mocha").is_some() {
                metadata.test_framework = Some("Mocha".to_string());
            }
        }
    }
}

fn extract_modified_top_level_keys(diff: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = diff.lines().collect();
    let re = match JSON_TOP_LEVEL_KEY_RE.as_ref() {
        Ok(re) => re,
        Err(e) => {
            debug!("Failed to compile JSON_TOP_LEVEL_KEY_RE: {}", e);
            return None;
        }
    };
    let mut keys = HashSet::new();

    for (i, line) in lines.iter().enumerate() {
        if let Some(cap) = re.captures(line)
            && let Some(key_match) = cap.get(1)
        {
            let key = key_match.as_str();
            let prev_line = if i > 0 { lines[i - 1] } else { "" };
            let next_line = if i + 1 < lines.len() {
                lines[i + 1]
            } else {
                ""
            };

            if !prev_line.trim().ends_with('{') && !next_line.trim().starts_with('}') {
                keys.insert(key.to_string());
            }
        }
    }

    if keys.is_empty() {
        None
    } else {
        Some(keys.into_iter().collect())
    }
}

fn has_array_changes(diff: &str) -> bool {
    JSON_ARRAY_CHANGE_RE.is_match(diff)
}

fn has_nested_object_changes(diff: &str) -> bool {
    JSON_NESTED_OBJECT_CHANGE_RE.is_match(diff)
}
