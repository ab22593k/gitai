use regex::Regex;
use std::path::Path;

use crate::{
    core::context::{ProjectMetadata, StagedFile},
    debug,
};

/// Trait for analyzing files and extracting relevant information
pub trait FileAnalyzer: Send + Sync {
    fn analyze(&self, file: &str, staged_file: &StagedFile) -> Vec<String>;
    fn get_file_type(&self) -> &'static str;
    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata;
}

/// Module for analyzing C files
mod c;
/// Module for analyzing C++ files
mod cpp;
/// Module for analyzing Gradle files
mod gradle;
/// Module for analyzing Java files
mod java;
/// Module for analyzing JavaScript files
mod javascript;
/// Module for analyzing JSON files
mod json;
/// Module for analyzing Kotlin files
mod kotlin;
/// Module for analyzing Markdown files
mod markdown;
/// Module for analyzing Python files
mod python;
/// Module for analyzing Rust files
mod rust;
/// Module for analyzing TOML files
mod toml;
/// Module for analyzing YAML files
mod yaml;

/// Module for analyzing generic text files
mod text;

/// Get the appropriate file analyzer based on the file extension
pub fn get_analyzer(file: &str) -> Box<dyn FileAnalyzer + Send + Sync> {
    let file_lower = file.to_lowercase();
    let path = std::path::Path::new(&file_lower);

    // Special cases for files with specific names
    if file == "Makefile" {
        return Box::new(c::CAnalyzer);
    } else if file == "CMakeLists.txt" {
        return Box::new(cpp::CppAnalyzer);
    }

    // Special cases for compound extensions
    if file_lower.ends_with(".gradle") || file_lower.ends_with(".gradle.kts") {
        return Box::new(gradle::GradleAnalyzer);
    }

    // Standard extension-based matching
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            let ext_lower = ext_str.to_lowercase();
            match ext_lower.as_str() {
                "c" => return Box::new(c::CAnalyzer),
                "cpp" | "cc" | "cxx" => return Box::new(cpp::CppAnalyzer),
                "rs" => return Box::new(rust::RustAnalyzer),
                "py" => return Box::new(python::PythonAnalyzer),
                "js" | "jsx" | "ts" | "tsx" => return Box::new(javascript::JavaScriptAnalyzer),
                "java" => return Box::new(java::JavaAnalyzer),
                "kt" | "kts" => return Box::new(kotlin::KotlinAnalyzer),
                "json" => return Box::new(json::JsonAnalyzer),
                "md" | "markdown" => return Box::new(markdown::MarkdownAnalyzer),
                "yaml" | "yml" => return Box::new(yaml::YamlAnalyzer),
                "toml" => return Box::new(toml::TomlAnalyzer),
                // Text-like extensions should use the generic text analyzer
                "txt" | "cfg" | "ini" | "properties" | "env" | "conf" | "config" | "xml"
                | "htm" | "html" | "css" | "scss" | "sass" | "less" | "sql" | "sh" | "bash"
                | "zsh" | "bat" | "cmd" | "ps1" | "dockerfile" | "editorconfig" | "gitignore"
                | "gitattributes" | "nginx" | "service" => {
                    return Box::new(text::GenericTextAnalyzer);
                }
                _ => {
                    // Try to determine if this is likely a text file
                    if is_likely_text_file(file) {
                        return Box::new(text::GenericTextAnalyzer);
                    }
                }
            }
        }
    } else {
        // Files without extension - check if they're likely text files
        if is_likely_text_file(file) {
            return Box::new(text::GenericTextAnalyzer);
        }
    }

    // Fall back to default analyzer for binary or unknown formats
    Box::new(DefaultAnalyzer)
}

/// Heuristic to determine if a file is likely text-based
fn is_likely_text_file(file: &str) -> bool {
    let file_name = std::path::Path::new(file).file_name();
    if let Some(name) = file_name
        && let Some(name_str) = name.to_str()
    {
        // Common configuration files without extensions
        let config_file_names = [
            "dockerfile",
            ".gitignore",
            ".gitattributes",
            ".env",
            "makefile",
            "readme",
            "license",
            "authors",
            "contributors",
            "changelog",
            "config",
            "codeowners",
            ".dockerignore",
            ".npmrc",
            ".yarnrc",
            ".eslintrc",
            ".prettierrc",
            ".babelrc",
            ".stylelintrc",
        ];

        for name in config_file_names {
            if name_str.to_lowercase() == name.to_lowercase() {
                return true;
            }
        }
    }

    false
}

/// Default analyzer for unsupported file types (likely binary)
struct DefaultAnalyzer;

impl FileAnalyzer for DefaultAnalyzer {
    fn analyze(&self, _file: &str, _staged_file: &StagedFile) -> Vec<String> {
        vec!["Unable to analyze non-text or binary file".to_string()]
    }

    fn get_file_type(&self) -> &'static str {
        "Unknown or binary file"
    }

    fn extract_metadata(&self, _file: &str, _content: &str) -> ProjectMetadata {
        ProjectMetadata {
            language: Some("Binary/Unknown".to_string()),
            ..Default::default()
        }
    }
}

/// Checks if a file should be excluded from analysis.
///
/// # Arguments
///
/// * `path` - The path of the file to check.
///
/// # Returns
///
/// A boolean indicating whether the file should be excluded.
pub fn should_exclude_file(path: &str) -> bool {
    debug!("Checking if file should be excluded: {}", path);
    let exclude_patterns = vec![
        (String::from(r"(^|/)\.git(/|$)"), false), // Only exclude .git directory, not .github
        (String::from(r"\.svn"), false),
        (String::from(r"\.hg"), false),
        (String::from(r"\.DS_Store"), false),
        (String::from(r"node_modules"), false),
        (String::from(r"target"), false),
        (String::from(r"build"), false),
        (String::from(r"dist"), false),
        (String::from(r"\.vscode"), false),
        (String::from(r"\.idea"), false),
        (String::from(r"\.vs"), false),
        (String::from(r"package-lock\.json$"), true),
        (String::from(r"\.lock$"), true),
        (String::from(r"\.log$"), true),
        (String::from(r"\.tmp$"), true),
        (String::from(r"\.temp$"), true),
        (String::from(r"\.swp$"), true),
        (String::from(r"\.min\.js$"), true),
    ];

    let path = Path::new(path);

    for (pattern, is_extension) in exclude_patterns {
        let re = match Regex::new(&pattern) {
            Ok(re) => re,
            Err(e) => {
                debug!("Failed to compile regex '{}': {}", pattern, e);
                continue;
            }
        };

        if is_extension {
            if let Some(file_name) = path.file_name()
                && let Some(file_name_str) = file_name.to_str()
                && re.is_match(file_name_str)
            {
                debug!("File excluded: {}", path.display());
                return true;
            }
        } else if let Some(path_str) = path.to_str()
            && re.is_match(path_str)
        {
            debug!("File excluded: {}", path.display());
            return true;
        }
    }
    debug!("File not excluded: {}", path.display());
    false
}
