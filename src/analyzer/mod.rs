use crate::core::context::{ProjectMetadata, StagedFile};

use log::debug;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// GitIgnore matcher that handles .gitignore file parsing and pattern matching.
///
/// This struct loads and caches all .gitignore files found in the repository,
/// including nested .gitignore files in subdirectories. It uses the ignore crate
/// to properly parse gitignore patterns and check if files should be excluded.
///
/// The matcher is lazily loaded on first use and cached for subsequent calls.
/// If no .gitignore files are found, it falls back to hardcoded exclusion patterns
/// for common files like node_modules, target/, .DS_Store, etc.
#[derive(Debug, Clone)]
pub struct GitIgnoreMatcher {
    /// Repository root path
    repo_root: PathBuf,
    /// Cached gitignore matcher built from all .gitignore files in the repo
    matcher: Arc<RwLock<Option<ignore::gitignore::Gitignore>>>,
}

impl GitIgnoreMatcher {
    /// Creates a new GitIgnoreMatcher for the given repository path
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_root: repo_path.to_path_buf(),
            matcher: Arc::new(RwLock::new(None)),
        }
    }

    /// Checks if a file path should be excluded based on gitignore patterns
    pub fn should_exclude(&self, file_path: &str) -> bool {
        debug!("GitIgnoreMatcher checking: {}", file_path);

        // First check if we have a cached matcher, if not, try to load it
        let mut matcher_guard = self.matcher.write();
        if matcher_guard.is_none() {
            // Load all .gitignore files in the repository
            let mut builder = ignore::gitignore::GitignoreBuilder::new(&self.repo_root);
            self.add_gitignore_files(&mut builder);

            match builder.build() {
                Ok(gitignore) => {
                    *matcher_guard = Some(gitignore);
                    debug!("Loaded gitignore matcher with all .gitignore files");
                }
                Err(e) => {
                    debug!("Failed to build gitignore matcher: {}", e);
                    // Leave matcher as None, will fall back to hardcoded patterns
                }
            }
        }

        // Check gitignore patterns
        let gitignore_excluded = if let Some(ref gitignore) = *matcher_guard {
            // Convert file path to be relative to repo root
            let full_path = self.repo_root.join(file_path);
            let relative_path = full_path
                .strip_prefix(&self.repo_root)
                .unwrap_or_else(|_| Path::new(file_path));

            // Check if the path is ignored
            let mut result = gitignore.matched(relative_path, false).is_ignore();

            // Also check directory matching for paths that contain directories
            if !result && file_path.contains('/') {
                // Try matching parent directories
                let parts: Vec<&str> = file_path.split('/').collect();
                for i in 1..=parts.len() {
                    let dir_path = parts[..i].join("/");
                    let dir_full = self.repo_root.join(&dir_path);
                    let dir_relative = dir_full
                        .strip_prefix(&self.repo_root)
                        .unwrap_or_else(|_| Path::new(&dir_path));

                    if gitignore.matched(dir_relative, true).is_ignore() {
                        result = true;
                        break;
                    }
                }
            }

            result
        } else {
            false
        };

        gitignore_excluded
    }

    /// Recursively adds all .gitignore files to the builder
    fn add_gitignore_files(&self, builder: &mut ignore::gitignore::GitignoreBuilder) {
        self.add_gitignore_from_dir(builder, &self.repo_root);
    }

    /// Recursively adds .gitignore files from a directory and its subdirectories to the builder
    fn add_gitignore_from_dir(
        &self,
        builder: &mut ignore::gitignore::GitignoreBuilder,
        dir: &Path,
    ) {
        let gitignore_path = dir.join(".gitignore");
        if gitignore_path.exists() {
            debug!("Adding .gitignore from: {:?}", gitignore_path);
            if let Some(e) = builder.add(&gitignore_path) {
                debug!("Failed to add .gitignore {:?}: {}", gitignore_path, e);
            }
        }

        // Recursively check subdirectories
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let subdir = entry.path();
                        // Skip .git directory
                        if !subdir.ends_with(".git") {
                            self.add_gitignore_from_dir(builder, &subdir);
                        }
                    }
                }
            }
        }
    }
}

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
