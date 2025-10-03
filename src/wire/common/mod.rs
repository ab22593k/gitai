pub mod fetch;
pub mod parse;
pub mod sequence;

use serde::{Deserialize, Serialize};

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
}

/*# [derive(Debug)]
pub enum ErrorType {
    RepositoryRootPathCommand,
    RepositoryRootPathParse,
    CurrentDirRetrieve,
    CurrentDirConvert,
    DotGitWireFileOpen,
    DotGitWireFileParse,
    DotGitWireFileSoundness,
    DotGitWireFileNameNotUnique,
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
} */

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Method {
    #[serde(rename = "shallow")]
    Shallow,

    #[serde(rename = "shallow_no_sparse")]
    ShallowNoSparse,

    #[serde(rename = "partial")]
    Partial,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Parsed {
    pub name: Option<String>,
    pub dsc: Option<String>,
    pub url: String,
    pub rev: String,
    pub src: String,
    pub dst: String,
    pub mtd: Option<Method>,
}

pub enum Target {
    Declared(Option<String>),
    Direct(Parsed),
}
