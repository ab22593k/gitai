use crate::analyzer;
use crate::core::context::ProjectMetadata;
use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use tokio::task;
use log::debug;

/// Analyzes a single file and extracts its metadata
#[allow(dead_code)]


/// Synchronous version of analyze_file for use with rayon
fn analyze_file_sync(file_path: &str, gitignore_matcher: &crate::analyzer::GitIgnoreMatcher) -> Option<ProjectMetadata> {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    let analyzer: Box<dyn analyzer::FileAnalyzer + Send + Sync> = analyzer::get_analyzer(file_name);

    debug!("Analyzing file (sync): {}", file_path);

    if gitignore_matcher.should_exclude(file_path) {
        debug!("File excluded: {}", file_path);
        None
    } else if let Ok(content) = std::fs::read_to_string(file_path) {
        let metadata = analyzer.extract_metadata(file_name, &content);
        debug!("Extracted metadata for {}: {:?}", file_name, metadata);
        Some(metadata)
    } else {
        debug!("Failed to read file: {}", file_path);
        None
    }
}

/// Extracts project metadata from a collection of files
///
/// Uses parallel processing with rayon to maximize CPU utilization
pub async fn extract_project_metadata(
    changed_files: &[String],
    _batch_size: usize, // Kept for API compatibility but not used
    gitignore_matcher: &crate::analyzer::GitIgnoreMatcher,
) -> Result<ProjectMetadata> {
    debug!(
        "Getting project metadata for {} changed files",
        changed_files.len()
    );

    // Clone the file paths to avoid lifetime issues
    let file_paths: Vec<String> = changed_files.to_vec();

    // Use tokio::task::spawn_blocking to run CPU-intensive parallel processing
    let gitignore_matcher_clone = gitignore_matcher.clone();
    let combined_metadata = task::spawn_blocking(move || {
        // Use rayon for parallel processing of file analysis
        let metadata_results: Vec<Option<ProjectMetadata>> = file_paths
            .par_iter()
            .map(|file_path| {
                // Use the synchronous version for rayon parallel processing
                analyze_file_sync(file_path, &gitignore_matcher_clone)
            })
            .collect();

        let mut combined_metadata = ProjectMetadata::default();
        let mut any_file_analyzed = false;

        for metadata in metadata_results.into_iter().flatten() {
            debug!("Merging metadata: {:?}", metadata);
            combined_metadata.merge(metadata);
            any_file_analyzed = true;
        }

        if !any_file_analyzed {
            debug!("No files were analyzed!");
            combined_metadata.language = Some("Unknown".to_string());
        } else if combined_metadata.language.is_none() {
            combined_metadata.language = Some("Unknown".to_string());
        }

        combined_metadata
    })
    .await?;

    debug!("Final combined metadata: {:?}", combined_metadata);

    Ok(combined_metadata)
}
