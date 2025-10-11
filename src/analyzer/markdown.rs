use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;

// Regex for extracting the first H1 header (potential project title)
static MD_TITLE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^#\s+(.+)$").expect("Should compile: MD_TITLE_RE")
});
// Regex for extracting version string (case-insensitive)
static MD_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)version[:\s]+(\d+\.\d+\.\d+)").expect("Should compile: MD_VERSION_RE")
});
// Regex for extracting modified headers (H1-H6)
static MD_MODIFIED_HEADER_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*(#{1,6})\s+(.+)").expect("Should compile: MD_MODIFIED_HEADER_RE")
});
// Regex for checking list item changes
static MD_LIST_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*[-*+]\s+").expect("Should compile: MD_LIST_CHANGE_RE")
});
// Regex for checking code block changes (```)
static MD_CODE_BLOCK_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*```").expect("Should compile: MD_CODE_BLOCK_CHANGE_RE")
});
// Regex for checking link changes ([text](url))
static MD_LINK_CHANGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"[+-]\s*\[.+\]\(.+\)").expect("Should compile: MD_LINK_CHANGE_RE")
});

pub struct MarkdownAnalyzer;

impl FileAnalyzer for MarkdownAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(headers) = extract_modified_headers(&staged_file.diff) {
            analysis.push(format!("Modified headers: {}", headers.join(", ")));
        }

        if has_list_changes(&staged_file.diff) {
            analysis.push("List structures have been modified".to_string());
        }

        if has_code_block_changes(&staged_file.diff) {
            analysis.push("Code blocks have been modified".to_string());
        }

        if has_link_changes(&staged_file.diff) {
            analysis.push("Links have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Markdown file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        if file.to_lowercase() == "readme.md" {
            Self::extract_readme_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl MarkdownAnalyzer {
    fn extract_readme_metadata(content: &str, metadata: &mut ProjectMetadata) {
        // Extract project name from the first header
        if let Some(cap) = MD_TITLE_RE.captures(content) {
            metadata.language = Some(cap[1].to_string());
        }

        // Look for badges that might indicate the build system or test framework
        if content.contains("travis-ci.org") {
            metadata.build_system = Some("Travis CI".to_string());
        } else if content.contains("github.com/actions/workflows") {
            metadata.build_system = Some("GitHub Actions".to_string());
        }

        if content.contains("coveralls.io") {
            metadata.test_framework = Some("Coveralls".to_string());
        }

        // Extract version if present
        if let Some(cap) = MD_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }
    }
}

fn extract_modified_headers(diff: &str) -> Option<Vec<String>> {
    let headers: Vec<String> = MD_MODIFIED_HEADER_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if headers.is_empty() {
        None
    } else {
        Some(headers)
    }
}

fn has_list_changes(diff: &str) -> bool {
    MD_LIST_CHANGE_RE.is_match(diff)
}

fn has_code_block_changes(diff: &str) -> bool {
    MD_CODE_BLOCK_CHANGE_RE.is_match(diff)
}

fn has_link_changes(diff: &str) -> bool {
    MD_LINK_CHANGE_RE.is_match(diff)
}
