use std::{fs, path::Path};

pub struct RepositoryFilter;

impl RepositoryFilter {
    /// Filter repository content based on the configuration filters.
    /// This function copies only the specified paths from source to destination.
    pub fn filter_repository_content(
        &self,
        source_path: &str,
        destination_path: &str,
        filters: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create destination directory if it doesn't exist.
        fs::create_dir_all(destination_path)?;

        // Process each filter path.
        for filter_path in filters {
            self.copy_filtered_content(source_path, destination_path, filter_path)?;
        }

        Ok(())
    }

    /// Copy specific content from source to destination based on a filter path.
    fn copy_filtered_content(
        &self,
        source_path: &str,
        destination_path: &str,
        filter_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source_dir = Path::new(source_path);
        let dest_dir = Path::new(destination_path);

        // Sanitize the filter path to prevent directory traversal.
        let normalized_filter = Self::normalize_path(filter_path);
        let source_filtered_path = source_dir.join(&normalized_filter);

        if source_filtered_path.exists() {
            if source_filtered_path.is_file() {
                // Copy a single file, preserving only the filename.
                let file_name = source_filtered_path
                    .file_name()
                    .ok_or("Invalid file path")?;
                let dest_file = dest_dir.join(file_name);
                fs::copy(&source_filtered_path, &dest_file)?;
            } else if source_filtered_path.is_dir() {
                // Copy the entire directory recursively, preserving the path structure.
                let dest_subdir = dest_dir.join(&normalized_filter);
                self.copy_dir_all(&source_filtered_path, &dest_subdir)?;
            }
        } else {
            // The filter path doesn't exist in the source; skip it.
            eprintln!("Warning: Filter path '{filter_path}' does not exist in source repository");
        }

        Ok(())
    }

    /// Recursively copy a directory and its contents.
    /// Skips symlinks and other non-regular file types.
    #[allow(clippy::only_used_in_recursion)]
    fn copy_dir_all(&self, src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure the destination directory exists.
        fs::create_dir_all(dst)?;

        // Read and process each entry in the source directory.
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_type = metadata.file_type();

            let entry_path = entry.path();
            let dest_entry_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                // Recurse into subdirectory.
                self.copy_dir_all(&entry_path, &dest_entry_path)?;
            } else if file_type.is_file() {
                // Copy the file.
                fs::copy(&entry_path, &dest_entry_path)?;
            }
            // Intentionally skip symlinks, sockets, etc.
        }

        Ok(())
    }

    /// Normalize a path to prevent directory traversal attacks (e.g., removes "../" sequences).
    /// This is a basic implementation; production code may require more robust validation.
    fn normalize_path(path: &str) -> String {
        // Split the path into components.
        let parts: Vec<&str> = path.split('/').collect();

        // Build a stack of valid path components.
        let mut stack: Vec<&str> = Vec::new();
        for &part in &parts {
            match part {
                "" | "." => {
                    // Skip empty parts and current directory markers.
                }
                ".." => {
                    // Go up one level if possible.
                    stack.pop();
                }
                _ => {
                    // Add normal directory or file name.
                    stack.push(part);
                }
            }
        }

        // Join the stack into a path string.
        let mut normalized = stack.join("/");

        // Remove leading slash to avoid absolute paths.
        if normalized.starts_with('/') {
            normalized = normalized[1..].to_string();
        }

        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_normalize_path() {
        // Test basic path normalization.
        assert_eq!(
            RepositoryFilter::normalize_path("src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            RepositoryFilter::normalize_path("./src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            RepositoryFilter::normalize_path("src/./main.rs"),
            "src/main.rs"
        );

        // Test path traversal removal.
        assert_eq!(
            RepositoryFilter::normalize_path("../src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            RepositoryFilter::normalize_path("src/../lib/utils.rs"),
            "lib/utils.rs"
        );
    }

    #[test]
    fn test_filter_repository_content() {
        // Create a temporary source directory structure.
        let src_dir = TempDir::new().expect("Failed to create temporary source directory");
        let src_path = src_dir.path();

        // Create test files and directories.
        fs::create_dir_all(src_path.join("src")).expect("Failed to create src directory");
        fs::create_dir_all(src_path.join("docs")).expect("Failed to create docs directory");
        fs::write(src_path.join("src").join("main.rs"), "fn main() {}")
            .expect("Failed to write main.rs");
        fs::write(src_path.join("docs").join("README.md"), "# Docs")
            .expect("Failed to write README.md");
        fs::write(src_path.join("LICENSE"), "MIT License").expect("Failed to write LICENSE");

        // Create a temporary destination directory.
        let dest_dir = TempDir::new().expect("Failed to create temporary destination directory");
        let dest_path = dest_dir.path();

        let filter = RepositoryFilter;
        let filters = vec!["src/".to_string()];

        // Apply the filter.
        filter
            .filter_repository_content(
                src_path.to_str().expect("Source path is not valid UTF-8"),
                dest_path
                    .to_str()
                    .expect("Destination path is not valid UTF-8"),
                &filters,
            )
            .expect("Failed to filter repository content");

        // Verify that only the filtered content was copied.
        assert!(dest_path.join("src").exists());
        assert!(dest_path.join("src").join("main.rs").exists());
        assert!(!dest_path.join("docs").exists());
        assert!(!dest_path.join("LICENSE").exists());
    }
}
