use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

// Regex for detecting line additions/removals in diffs
static DIFF_ADDED_REMOVED_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-][^+-]").expect("Should compile: DIFF_ADDED_REMOVED_RE")
});

// Regex for detecting key-value pairs in config-like files
static CONFIG_KV_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]?\s*(\w+[\w\.\-]*)\s*[=:]\s*(.+?)$")
        .expect("Should compile: CONFIG_KV_RE")
});

// Regex for detecting XML/HTML-like tags
static TAG_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"</?(\w+)[\s>]").expect("Should compile: TAG_RE"));

/// Generic analyzer for text-based files without specialized analyzers
pub struct GenericTextAnalyzer;

impl FileAnalyzer for GenericTextAnalyzer {
    fn analyze(&self, file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        // Count lines added/removed
        let added = DIFF_ADDED_REMOVED_RE
            .captures_iter(&staged_file.diff)
            .filter(|cap| cap[0].starts_with('+'))
            .count();

        let removed = DIFF_ADDED_REMOVED_RE
            .captures_iter(&staged_file.diff)
            .filter(|cap| cap[0].starts_with('-'))
            .count();

        if added > 0 || removed > 0 {
            analysis.push(format!(
                "Changes: {added} line(s) added, {removed} line(s) removed"
            ));
        }

        // For config-like files, analyze key-value pairs
        if is_likely_config_file(file)
            && let Some(keys) = extract_modified_keys(&staged_file.diff)
        {
            analysis.push(format!("Modified configuration keys: {}", keys.join(", ")));
        }

        // For XML/HTML-like files, analyze tag changes
        if (has_extension(file, "xml") || has_extension(file, "html") || has_extension(file, "htm"))
            && let Some(tags) = extract_modified_tags(&staged_file.diff)
        {
            analysis.push(format!("Modified tags: {}", tags.join(", ")));
        }

        // Add a fallback analysis if nothing else was found
        if analysis.is_empty() {
            analysis.push("Text file changed".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Text file"
    }

    fn extract_metadata(&self, file: &str, _content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        // Try to infer file type based on extension or name
        if has_extension(file, "xml") {
            metadata.language = Some("XML".to_string());
        } else if has_extension(file, "html") || has_extension(file, "htm") {
            metadata.language = Some("HTML".to_string());
        } else if has_extension(file, "css") {
            metadata.language = Some("CSS".to_string());
        } else if has_extension(file, "scss") || has_extension(file, "sass") {
            metadata.language = Some("SASS/SCSS".to_string());
        } else if has_extension(file, "sql") {
            metadata.language = Some("SQL".to_string());
        } else if has_extension(file, "sh") || has_extension(file, "bash") {
            metadata.language = Some("Shell script".to_string());
        } else if has_extension(file, "bat") || has_extension(file, "cmd") {
            metadata.language = Some("Batch script".to_string());
        } else if has_extension(file, "ps1") {
            metadata.language = Some("PowerShell script".to_string());
        } else if file.to_lowercase().contains("dockerfile") {
            metadata.language = Some("Dockerfile".to_string());
        } else if has_extension(file, "env") || file.contains(".env.") {
            metadata.language = Some("Environment variables".to_string());
        } else if has_extension(file, "ini")
            || has_extension(file, "cfg")
            || has_extension(file, "conf")
        {
            metadata.language = Some("Configuration file".to_string());
        } else {
            metadata.language = Some("Plain text".to_string());
        }

        metadata
    }
}

/// Check if a file has the given extension in a case-insensitive way
fn has_extension(file: &str, ext: &str) -> bool {
    Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Determine if a file is likely a configuration file based on extension or name
fn is_likely_config_file(file: &str) -> bool {
    let file_lower = file.to_lowercase();
    has_extension(file, "ini")
        || has_extension(file, "cfg")
        || has_extension(file, "conf")
        || has_extension(file, "config")
        || has_extension(file, "properties")
        || has_extension(file, "env")
        || file_lower.contains(".env.")
        || file_lower.contains("config")
        || file_lower.contains("nginx")
        || file_lower.contains(".rc")
}

/// Extract modified keys from config-like files
fn extract_modified_keys(diff: &str) -> Option<Vec<String>> {
    let keys: HashSet<String> = CONFIG_KV_RE
        .captures_iter(diff)
        .filter_map(|cap| {
            let key = cap.get(1)?.as_str().to_string();
            Some(key)
        })
        .collect();

    if keys.is_empty() {
        None
    } else {
        Some(keys.into_iter().collect())
    }
}

/// Extract modified tags from XML/HTML-like files
fn extract_modified_tags(diff: &str) -> Option<Vec<String>> {
    let tags: HashSet<String> = TAG_RE
        .captures_iter(diff)
        .filter_map(|cap| {
            let tag = cap.get(1)?.as_str().to_string();
            Some(tag)
        })
        .collect();

    if tags.is_empty() {
        None
    } else {
        Some(tags.into_iter().collect())
    }
}
