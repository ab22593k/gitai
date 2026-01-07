pub mod fetch;
pub mod parse;
pub mod sequence;

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Method {
    Shallow,
    ShallowNoSparse,
    Partial,
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
}

pub enum Target {
    Declared(TargetConfig),
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
