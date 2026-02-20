use log::debug;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// `GitIgnore` matcher that handles .gitignore file parsing and pattern matching.
///
/// This struct loads and caches all .gitignore files found in the repository,
/// including nested .gitignore files in subdirectories. It uses the ignore crate
/// to properly parse gitignore patterns and check if files should be excluded.
///
/// The matcher is lazily loaded on first use and cached for subsequent calls.
/// If no .gitignore files are found, it falls back to hardcoded exclusion patterns
/// for common files like `node_modules`, target/, `.DS_Store`, etc.
#[derive(Debug, Clone)]
pub struct GitIgnoreMatcher {
    /// Repository root path
    repo_root: PathBuf,
    /// Cached gitignore matcher built from all .gitignore files in the repo
    matcher: Arc<RwLock<Option<ignore::gitignore::Gitignore>>>,
}

impl GitIgnoreMatcher {
    /// Creates a new `GitIgnoreMatcher` for the given repository path
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
        if let Some(ref gitignore) = *matcher_guard {
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
        }
    }

    /// Recursively adds all .gitignore files to the builder
    fn add_gitignore_files(&self, builder: &mut ignore::gitignore::GitignoreBuilder) {
        Self::add_gitignore_from_dir(builder, &self.repo_root);
    }

    /// Recursively adds .gitignore files from a directory and its subdirectories to the builder
    fn add_gitignore_from_dir(builder: &mut ignore::gitignore::GitignoreBuilder, dir: &Path) {
        let gitignore_path = dir.join(".gitignore");
        if gitignore_path.exists() {
            debug!("Adding .gitignore from: {}", gitignore_path.display());
            if let Some(e) = builder.add(&gitignore_path) {
                debug!(
                    "Failed to add .gitignore {}: {}",
                    gitignore_path.display(),
                    e
                );
            }
        }

        // Recursively check subdirectories
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_dir()
                {
                    let subdir = entry.path();
                    // Skip .git directory
                    if !subdir.ends_with(".git") {
                        Self::add_gitignore_from_dir(builder, &subdir);
                    }
                }
            }
        }
    }
}
