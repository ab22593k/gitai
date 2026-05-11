pub mod fetch;
pub mod parse;
pub mod sequence;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Component, Path};

#[derive(Debug)]
pub enum ErrorType {
    RepositoryRootPathCommand,
    RepositoryRootPathParse,
    CurrentDirRetrieve,
    CurrentDirConvert,
    DotGitWireFileOpen,
    DotGitWireFileParse,
    DotGitWireFileSoundness,
    DotGitWireFileNameNotUnique,
    DotGitWireFileWrite,
    TempDirCreation,
    GitCloneCommand,
    GitCloneCommandExitStatus,
    GitCheckoutCommand,
    GitCheckoutCommandExitStatus,
    GitCheckoutChangeDirectory,
    GitFetchCommand,
    GitFetchCommandExitStatus,
    MoveFromTempToDest,
    NoItemToOperate,
    CheckDifferenceExecution,
    CheckDifferenceStringReplace,
    GitLsRemoteCommand,
    GitLsRemoteCommandExitStatus,
    GitLsRemoteCommandStdoutDecode,
    GitLsRemoteCommandStdoutRegex,
    PromptError,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, ValueEnum)]
pub enum Method {
    Shallow,
    ShallowNoSparse,
    Partial,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum MergeStrategy {
    Overwrite,
    #[default]
    Auto,
    Manual,
    Ai,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Parsed {
    pub name: Option<String>,
    pub dsc: Option<String>,
    pub url: String,
    pub rev: String,
    pub src: Vec<String>,
    pub dst: String,
    pub mtd: Option<Method>,
    pub last_sync_hash: Option<String>,
    pub merge_strategy: Option<MergeStrategy>,
}

impl Parsed {
    /// Validate that all required fields are present and valid
    pub fn validate(&self) -> Result<(), String> {
        if self.url.is_empty() {
            return Err("URL is required".to_string());
        }
        if self.rev.is_empty() {
            return Err("Revision is required".to_string());
        }
        if self.src.is_empty() {
            return Err("At least one source path is required".to_string());
        }
        if self.dst.is_empty() {
            return Err("Destination is required".to_string());
        }

        // Validate path soundness for all src paths
        for s in &self.src {
            if !is_path_sound(s) {
                return Err(format!(
                    "Source path '{s}' contains invalid components (., .., or .git)"
                ));
            }
        }

        if !is_path_sound(&self.dst) {
            return Err(format!(
                "Destination path '{}' contains invalid components (., .., or .git)",
                self.dst
            ));
        }

        Ok(())
    }
}

/// Configuration for `Target::Declared` variant
#[derive(Debug, Clone, Default)]
pub struct TargetConfig {
    /// Filter by entry name (--name or -n flag)
    pub name_filter: Option<String>,
    /// CLI-provided configuration (overrides .gitwire.toml)
    pub cli_override: Option<Parsed>,
    /// Save configuration to .gitwire.toml after operation
    pub save_config: bool,
    /// Append to existing .gitwire.toml instead of overwriting
    pub append_config: bool,
    /// Use global config (~/.gitwire) instead of local (.gitwire)
    pub global: bool,
}

/// Merge CLI-provided Parsed with an existing Parsed from .gitwire.toml
/// CLI values take precedence (override) when non-empty
pub fn merge_parsed(target: &mut Parsed, source: &Parsed) {
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
    if source.last_sync_hash.is_some() {
        target.last_sync_hash.clone_from(&source.last_sync_hash);
    }
    if source.merge_strategy.is_some() {
        target.merge_strategy.clone_from(&source.merge_strategy);
    }
}

/// Helper function for path validation
/// Returns true if path is sound (doesn't contain ., .., or .git)
pub fn is_path_sound(path: &str) -> bool {
    Path::new(path).components().all(|c| match c {
        Component::Prefix(_) | Component::RootDir => true,
        Component::Normal(name) => name != OsStr::new(".git"),
        Component::ParentDir | Component::CurDir => false,
    })
}

/// Normalize a GitHub browser URL to a git clone URL.
pub fn normalize_github_url(url: &str) -> String {
    if !url.contains("github.com") {
        return url.to_string();
    }
    let url = url.trim_end_matches('/');
    if let Some(pos) = url.find("/tree/") {
        return format!("{}.git", &url[..pos]);
    }
    if let Some(pos) = url.find("/blob/") {
        return format!("{}.git", &url[..pos]);
    }
    if std::path::Path::new(url)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("git"))
    {
        return url.to_string();
    }
    format!("{url}.git")
}

/// Infer revision and source path from a GitHub browser URL.
/// Infers revision and source path from a GitHub browser URL.
/// Returns Some((rev, `src_paths`)) if inference is possible, None otherwise.
pub fn infer_from_url(url: &str) -> Option<(String, Vec<String>)> {
    if !url.contains("github.com") {
        return None;
    }

    let url = url.trim_end_matches('/');

    // Try /tree/ first, then /blob/
    let (separator, base_pos) = if let Some(pos) = url.find("/tree/") {
        ("/tree/", pos)
    } else if let Some(pos) = url.find("/blob/") {
        ("/blob/", pos)
    } else {
        return None;
    };

    // Find the part after /tree/ or /blob/
    let after_separator = &url[base_pos + separator.len()..];

    // Split by the next '/' to get rev and remaining path
    let slash_pos = after_separator.find('/')?;
    let rev = after_separator[..slash_pos].to_string();
    let src_path = after_separator[slash_pos + 1..].to_string();

    if rev.is_empty() || src_path.is_empty() {
        return None;
    }

    // For /blob/ URLs, the path may include a file, so extract just the directory
    let src_path = if separator == "/blob/" {
        let path = std::path::Path::new(&src_path);
        path.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(src_path)
    } else {
        src_path
    };

    Some((rev, vec![src_path]))
}
