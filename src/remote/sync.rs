use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use cause::{Cause, cause};
use fs_extra::{copy_items, dir::CopyOptions, remove_items};
use log::{debug, info};
use tokio::task::JoinSet;

use super::cache::{
    fetcher::RepositoryFetcher, key_generator::CacheKeyGenerator, manager::CacheManager,
};
use super::common::{ErrorType, Parsed, Target, TargetConfig, parse};
use super::models::repo_config::RepositoryConfiguration;
use super::models::wire_operation::WireOperation;

/// Merge CLI-provided Parsed with an existing Parsed from .gitwire.toml
/// CLI values take precedence (override) when non-empty
fn merge_parsed(target: &mut Parsed, source: &Parsed) {
    if !source.url.is_empty() {
        target.url.clone_from(&source.url);
    }
    if !source.rev.is_empty() {
        target.rev.clone_from(&source.rev);
    }
    if !source.src.is_empty() {
        target.src.clone_from(&source.src);
    }
    if !source.dst.is_empty() {
        target.dst.clone_from(&source.dst);
    }
    if source.name.is_some() {
        target.name.clone_from(&source.name);
    }
    if source.dsc.is_some() {
        target.dsc.clone_from(&source.dsc);
    }
    if source.mtd.is_some() {
        target.mtd.clone_from(&source.mtd);
    }
}

/// Convert Parsed items to `RepositoryConfiguration`
fn parsed_to_config(parsed: Parsed) -> RepositoryConfiguration {
    RepositoryConfiguration::new(
        parsed.url, parsed.rev, parsed.dst, parsed.src, None, parsed.mtd,
    )
}

fn get_repo_configs(
    target: &Target,
) -> Result<(String, Vec<RepositoryConfiguration>, Option<Parsed>), Cause<ErrorType>> {
    let Target::Declared(config) = target;
    get_repo_configs_declared(config)
}

fn get_repo_configs_declared(
    config: &TargetConfig,
) -> Result<(String, Vec<RepositoryConfiguration>, Option<Parsed>), Cause<ErrorType>> {
    // Try to parse .gitwire.toml
    let gitwire_data = parse::parse_gitwire()?;

    let (root, parsed_items, cli_parsed_for_save) = match (gitwire_data, &config.cli_override) {
        // Both .gitwire.toml and CLI args provided
        (Some((root, mut file_items)), Some(cli_parsed)) => {
            if let Some(name) = &config.name_filter {
                // Try to find and override entry by name
                if let Some(entry) = file_items
                    .iter_mut()
                    .find(|p| p.name.as_ref() == Some(name))
                {
                    merge_parsed(entry, cli_parsed);
                    // Keep only the matched entry
                    let matched = entry.clone();
                    file_items.retain(|p| p.name.as_ref() == Some(name));
                    (root, file_items, Some(matched))
                } else {
                    // Name not found, use CLI args as new entry
                    (root, vec![cli_parsed.clone()], Some(cli_parsed.clone()))
                }
            } else {
                // No name filter: use CLI args only (override all)
                (root, vec![cli_parsed.clone()], Some(cli_parsed.clone()))
            }
        }

        // Only .gitwire.toml exists
        (Some((root, mut file_items)), None) => {
            if let Some(name) = &config.name_filter {
                file_items.retain(|p| p.name.as_ref() == Some(name));
                if file_items.is_empty() {
                    return Err(cause!(
                        ErrorType::NoItemToOperate,
                        format!("No entry with name '{name}' found in .gitwire.toml")
                    ));
                }
            }
            (root, file_items, None)
        }

        // Only CLI args provided (no .gitwire.toml)
        (None, Some(cli_parsed)) => {
            let root = env::current_dir()
                .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                .to_string_lossy()
                .to_string();
            (root, vec![cli_parsed.clone()], Some(cli_parsed.clone()))
        }

        // Neither provided - show interactive prompt
        (None, None) => match parse::prompt_create_gitwire()? {
            Some(parsed) => {
                let root = env::current_dir()
                    .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                    .to_string_lossy()
                    .to_string();

                // Save the prompted config
                parse::save_to_gitwire_toml(&parsed, false)?;

                (root, vec![parsed.clone()], Some(parsed))
            }
            None => {
                return Err(cause!(
                    ErrorType::NoItemToOperate,
                    "No .gitwire.toml file found and no CLI arguments provided.\n\
                     \nUsage examples:\n\
                     \n  git-wire sync --url <URL> --rev <REV> --src <SRC> --dst <DST>\n\
                     \n  git-wire sync --url <URL> --rev <REV> --src '[\"lib\",\"tools\"]' --dst <DST>\n\
                     \n  git-wire sync  # Interactive mode"
                ));
            }
        },
    };

    // Convert parsed items to RepositoryConfigurations
    let repo_configs = parsed_items.into_iter().map(parsed_to_config).collect();

    Ok((root, repo_configs, cli_parsed_for_save))
}

/// Validate that the target path is within the project root to prevent path traversal attacks.
/// Returns an error if the path would escape the root directory.
fn validate_dest_path(
    root_dir: &str,
    target_path: &str,
) -> Result<std::path::PathBuf, Cause<ErrorType>> {
    let root_path = Path::new(root_dir)
        .canonicalize()
        .map_err(|e| cause!(ErrorType::MoveFromTempToDest).src(e))?;

    let dest_path = Path::new(root_dir)
        .join(target_path)
        .canonicalize()
        .map_err(|e| {
            cause!(ErrorType::MoveFromTempToDest).msg(format!(
                "Cannot resolve destination path '{target_path}': {e}"
            ))
        })?;

    // Check that the destination is within the root directory
    if !dest_path.starts_with(&root_path) {
        return Err(cause!(ErrorType::MoveFromTempToDest).msg(format!(
            "Destination path '{target_path}' escapes the project root (path traversal not allowed)"
        )));
    }

    Ok(dest_path)
}

// Enhanced sync functionality that integrates caching
pub async fn sync_with_caching(
    target: &Target,
    _mode: super::common::sequence::Mode,
) -> Result<bool, Cause<ErrorType>> {
    info!("git-wire sync with caching started");

    let (root_dir, repo_configs, cli_parsed_for_save) = get_repo_configs(target)?;
    handle_save_config(target, cli_parsed_for_save.as_ref())?;

    let cache_manager = CacheManager::new();
    let fetcher = RepositoryFetcher;

    let (unique_configs, mut wire_operations) = cache_manager
        .plan_fetch_operations(&repo_configs)
        .map_err(|e| cause!(ErrorType::NoItemToOperate).msg(e))?;

    info!(
        "Identified {} unique repositories to fetch ({} redundant fetches avoided)",
        unique_configs.len(),
        repo_configs.len().saturating_sub(unique_configs.len())
    );

    let fetch_results = fetch_repositories(unique_configs, fetcher).await;

    update_wire_operations_with_cache(&mut wire_operations, fetch_results)?;

    execute_wire_operations(&root_dir, &wire_operations)?;

    info!("git-wire sync with caching completed");
    Ok(true)
}

fn handle_save_config(
    target: &Target,
    cli_parsed_for_save: Option<&Parsed>,
) -> Result<(), Cause<ErrorType>> {
    let Target::Declared(config) = target;
    if config.save_config
        && let Some(parsed) = cli_parsed_for_save
    {
        parse::save_to_gitwire_toml(parsed, config.append_config)?;
    }
    Ok(())
}

async fn fetch_repositories(
    unique_configs: Vec<RepositoryConfiguration>,
    fetcher: RepositoryFetcher,
) -> Vec<Result<(RepositoryConfiguration, String), Cause<ErrorType>>> {
    let mut join_set = JoinSet::new();
    let fetcher = Arc::new(fetcher);

    for config in unique_configs.clone() {
        let fetcher = fetcher.clone();
        join_set.spawn(async move {
            let cache_key = CacheKeyGenerator::generate_key(&config);
            let cache_dir = env::temp_dir().join("git-wire-cache").join(cache_key);
            fs::create_dir_all(&cache_dir)
                .map_err(|e| cause!(ErrorType::TempDirCreation).src(e))?;
            let cache_path = cache_dir.to_string_lossy().to_string();

            debug!(
                "Fetching repository {} to cache path {}",
                config.url, cache_path
            );
            fetcher.fetch_repository(&config, &cache_path).await?;
            debug!("Repository {} successfully cached", config.url);
            Ok((config, cache_path))
        });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(res)) => results.push(Ok(res)),
            Ok(Err(e)) => results.push(Err(e)),
            Err(e) => results.push(Err(
                cause!(ErrorType::GitCloneCommand).msg(format!("Task join error: {e}"))
            )),
        }
    }
    results
}

fn update_wire_operations_with_cache(
    wire_operations: &mut Vec<WireOperation>,
    fetch_results: Vec<Result<(RepositoryConfiguration, String), Cause<ErrorType>>>,
) -> Result<(), Cause<ErrorType>> {
    for result in fetch_results {
        let (config, cache_path) = result?;
        for op in &mut *wire_operations {
            if op.source_config.url == config.url && op.source_config.branch == config.branch {
                op.cached_repo_path.clone_from(&cache_path);
            }
        }
    }
    Ok(())
}

fn execute_wire_operations(
    root_dir: &str,
    wire_operations: &[WireOperation],
) -> Result<(), Cause<ErrorType>> {
    for wire_op in wire_operations {
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

        let dest_dir = validate_dest_path(root_dir, &wire_op.source_config.target_path)?;

        if dest_dir.exists() {
            remove_items(&[dest_dir.as_path()]).map_err(|e| {
                cause!(ErrorType::MoveFromTempToDest)
                    .src(e)
                    .msg(format!("Could not remove {}", dest_dir.display()))
            })?;
        }

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
    Ok(())
}
