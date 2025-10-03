use std::env;
use std::fs;
use std::path::Path;

use cause::{Cause, cause};
use fs_extra::{copy_items, dir::CopyOptions, remove_items};
use log::{debug, info};

use super::cache::{
    fetcher::RepositoryFetcher, key_generator::CacheKeyGenerator, manager::CacheManager,
};
use super::common::{ErrorType, Target, parse};
use super::models::repo_config::RepositoryConfiguration;

fn get_repo_configs(
    target: &Target,
) -> Result<(String, Vec<RepositoryConfiguration>), Cause<ErrorType>> {
    match target {
        Target::Declared(opt_name) => {
            let (root, mut parsed_items) = parse::parse_gitwire()?;
            if let Some(name) = opt_name {
                parsed_items.retain(|p| p.name.as_ref() == Some(name));
                if parsed_items.is_empty() {
                    return Err(cause!(
                        ErrorType::NoItemToOperate,
                        "No item with specified name"
                    ));
                }
            }
            let repo_configs = parsed_items
                .into_iter()
                .map(|parsed| {
                    RepositoryConfiguration::new(
                        parsed.url,
                        parsed.rev,
                        parsed.dst,
                        vec![parsed.src],
                        None,
                    )
                })
                .collect();
            Ok((root, repo_configs))
        }
        Target::Direct(parsed) => {
            let root = env::current_dir()
                .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                .to_string_lossy()
                .to_string();
            let repo_configs = vec![RepositoryConfiguration::new(
                parsed.url.clone(),
                parsed.rev.clone(),
                parsed.dst.clone(),
                vec![parsed.src.clone()],
                None,
            )];
            Ok((root, repo_configs))
        }
    }
}

// Enhanced sync functionality that integrates caching
pub fn sync_with_caching(
    target: &Target,
    _mode: super::common::sequence::Mode,
) -> Result<bool, Cause<ErrorType>> {
    info!("git-wire sync with caching started");

    let (root_dir, repo_configs) = get_repo_configs(target)?;

    info!("Found {} repository configurations", repo_configs.len());

    // Create components needed for caching
    let cache_manager = CacheManager::new();
    let fetcher = RepositoryFetcher;

    // Plan fetch operations to identify unique repositories
    let (unique_configs, mut wire_operations) = cache_manager
        .plan_fetch_operations(&repo_configs)
        .map_err(|e| cause!(ErrorType::NoItemToOperate).msg(e))?;

    info!(
        "Identified {} unique repositories to fetch ({} redundant fetches avoided)",
        unique_configs.len(),
        repo_configs.len().saturating_sub(unique_configs.len())
    );

    // Fetch each unique repository to its cache location
    for config in &unique_configs {
        let cache_key = CacheKeyGenerator::generate_key(config);
        let cache_dir = env::temp_dir().join("git-wire-cache").join(cache_key);
        fs::create_dir_all(&cache_dir).map_err(|e| cause!(ErrorType::TempDirCreation).src(e))?;
        let cache_path = cache_dir.to_string_lossy().to_string();

        debug!(
            "Fetching repository {} to cache path {}",
            config.url, cache_path
        );
        fetcher.fetch_repository(config, &cache_path)?;
        debug!("Repository {} successfully cached", config.url);

        // Update the wire operations to use the actual cache path
        for op in &mut wire_operations {
            if op.source_config.url == config.url && op.source_config.branch == config.branch {
                op.cached_repo_path.clone_from(&cache_path);
            }
        }
    }

    // Execute wire operations using cached repositories
    for wire_op in &wire_operations {
        if wire_op.source_config.filters.is_empty() {
            debug!(
                "Skipping wire operation with no filters: {}",
                wire_op.operation_id
            );
            continue;
        }

        let source_subdir = &wire_op.source_config.filters[0];
        let source_content = Path::new(&wire_op.cached_repo_path).join(source_subdir);
        if !source_content.exists() {
            info!(
                "Source path {} does not exist in cached repo {}",
                source_subdir, wire_op.source_config.url
            );
            continue;
        }

        let dest_dir = Path::new(&root_dir).join(&wire_op.source_config.target_path);

        // Remove destination if it exists
        if dest_dir.exists() {
            remove_items(&[dest_dir.as_path()]).map_err(|e| {
                cause!(ErrorType::MoveFromTempToDest)
                    .src(e)
                    .msg(format!("Could not remove {}", dest_dir.display()))
            })?;
        }

        // Create destination directory
        fs::create_dir_all(&dest_dir).map_err(|e| cause!(ErrorType::MoveFromTempToDest).src(e))?;

        let mut opt = CopyOptions::new();
        opt.overwrite = true;
        opt.copy_inside = true;

        copy_items(&[source_content.as_path()], &dest_dir, &opt).map_err(|e| {
            cause!(ErrorType::MoveFromTempToDest).src(e).msg(format!(
                "Could not copy {} to {}",
                source_content.display(),
                dest_dir.display()
            ))
        })?;

        debug!(
            "Copied contents of {source_subdir} to {}",
            wire_op.source_config.target_path
        );
    }

    info!("git-wire sync with caching completed");
    Ok(true)
}
