use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use cloy_message::{CmsgConfig, CommonArgs, handle_message};
use git2::Repository;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Serializes tests that change the current working directory to prevent
/// interference between parallel test executions.
static CWD_LOCK: Mutex<()> = Mutex::const_new(());

/// Saves the original working directory and restores it on drop.
struct CwdGuard(PathBuf);
impl CwdGuard {
    fn new(path: &Path) -> Self {
        let old = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(path).expect("Failed to change directory");
        Self(old)
    }
}
impl Drop for CwdGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).expect("Failed to restore current dir");
    }
}

/// A temporary git repository for testing.
///
/// Sets up user identity and gitai config entries in the repo's `.git/config`
/// so that `Config::load()` finds them via layered git config reads (no env
/// vars needed, so no `unsafe`).
struct TestRepo {
    _dir: TempDir,
    path: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init git repo");
        let mut config = repo.config().expect("Failed to get repo config");

        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@test.com")
            .expect("Failed to set user.email");

        config
            .set_str("gitai.instructions", "test commit")
            .expect("Failed to set gitai.instructions");
        config
            .set_str("gitai.google-apikey", "test-api-key")
            .expect("Failed to set gitai.google-apikey");
        config
            .set_str("gitai.google-model", "test-model")
            .expect("Failed to set gitai.google-model");

        let repo_path = dir.path().to_path_buf();
        Self {
            _dir: dir,
            path: repo_path,
        }
    }

    fn stage_file(&self, name: &str, content: &str) {
        let file_path = self.path.join(name);
        std::fs::write(&file_path, content).expect("Failed to write file");
        let repo = Repository::open(&self.path).expect("Failed to open repo");
        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new(name))
            .expect("Failed to add file to index");
        index.write().expect("Failed to write index");
    }
}

/// Parse CLI args and extract the parts needed for `handle_message`.
fn make_handler_args(args: &[&str]) -> (CommonArgs, Option<String>) {
    let mut cli = <CommonArgs as Parser>::try_parse_from(args.iter().copied())
        .expect("CLI args should parse");
    let repository_url = std::mem::take(&mut cli.common.repository_url);
    (cli, repository_url)
}

mod with_repo {
    use cloy_message::MessageArgs;

    use super::*;

    #[tokio::test]
    async fn generate_mode_no_staged_files_returns_ok() {
        // Risk #2: "No staged files" path should not error
        let _lock = CWD_LOCK.lock().await;
        let repo = TestRepo::new();
        let _cwd = CwdGuard::new(&repo.path);

        let (cli, repo_url) = make_handler_args(&["git-message", "--print"]);

        let result = handle_message(
            cli.common,
            CmsgConfig {
                print_only: cli.params.print,
            },
            repo_url,
            MessageArgs {
                complete: cli.params.complete,
                prefix: cli.params.prefix,
                context_ratio: cli.params.context_ratio,
            },
        )
        .await;

        assert!(
            result.is_ok(),
            "No staged files in generate mode should return Ok (prints warning)"
        );
    }

    #[tokio::test]
    async fn complete_mode_no_staged_files_returns_ok() {
        // Risk #2: completion path should also handle no-staged-files gracefully
        let _lock = CWD_LOCK.lock().await;
        let repo = TestRepo::new();
        let _cwd = CwdGuard::new(&repo.path);

        let (cli, repo_url) =
            make_handler_args(&["git-message", "--complete", "--prefix", "fix:", "--print"]);

        let result = handle_message(
            cli.common,
            CmsgConfig {
                print_only: cli.params.print,
            },
            repo_url,
            MessageArgs {
                complete: cli.params.complete,
                prefix: cli.params.prefix,
                context_ratio: cli.params.context_ratio,
            },
        )
        .await;

        assert!(
            result.is_ok(),
            "No staged files in complete mode should return Ok (prints warning)"
        );
    }

    #[tokio::test]
    async fn complete_without_prefix_errors_at_runtime() {
        // Risk #4: --complete without --prefix is accepted by clap but fails at runtime
        let _lock = CWD_LOCK.lock().await;
        let repo = TestRepo::new();
        let _cwd = CwdGuard::new(&repo.path);

        let (cli, repo_url) = make_handler_args(&["git-message", "--complete", "--print"]);

        let result = handle_message(
            cli.common,
            CmsgConfig {
                print_only: cli.params.print,
            },
            repo_url,
            MessageArgs {
                complete: cli.params.complete,
                prefix: cli.params.prefix,
                context_ratio: cli.params.context_ratio,
            },
        )
        .await;

        assert!(
            result.is_err(),
            "--complete without --prefix should be caught at runtime"
        );
        let err = result.expect_err("Already checked is_err");
        let err_msg = format!("{err}");
        assert!(
            err_msg.contains("Prefix is required"),
            "Error should mention that prefix is required, got: {err_msg}"
        );
    }

    #[tokio::test]
    #[ignore = "Requires LLM API access (fake API key triggers retry-backoff that exceeds test timeout)"]
    async fn generate_with_staged_files_past_git_checks() {
        // Verifies the flow reaches the LLM call (which will fail without a real key),
        // proving git/config validation passed. Timeboxed because LLM client may retry.
        let _lock = CWD_LOCK.lock().await;
        let repo = TestRepo::new();
        repo.stage_file("main.rs", "fn main() { println!(\"hello\"); }");
        let _cwd = CwdGuard::new(&repo.path);

        let (cli, repo_url) = make_handler_args(&["git-message", "--print"]);

        let result = timeout(
            Duration::from_secs(90),
            handle_message(
                cli.common,
                CmsgConfig {
                    print_only: cli.params.print,
                },
                repo_url,
                MessageArgs {
                    complete: cli.params.complete,
                    prefix: cli.params.prefix,
                    context_ratio: cli.params.context_ratio,
                },
            ),
        )
        .await;

        // The error should be from the LLM/API level, NOT from git or config
        let result = result.expect("LLM call should complete or fail within 90s, not hang");
        assert!(
            result.is_err(),
            "Staged files with --print should attempt LLM call and fail at API level"
        );
    }

    #[tokio::test]
    #[ignore = "Requires LLM API access (fake API key triggers retry-backoff that exceeds test timeout)"]
    async fn complete_with_staged_files_past_git_checks() {
        let _lock = CWD_LOCK.lock().await;
        let repo = TestRepo::new();
        repo.stage_file("main.rs", "fn main() { println!(\"hello\"); }");
        let _cwd = CwdGuard::new(&repo.path);

        let (cli, repo_url) =
            make_handler_args(&["git-message", "--complete", "--prefix", "fix:", "--print"]);

        let result = timeout(
            Duration::from_secs(90),
            handle_message(
                cli.common,
                CmsgConfig {
                    print_only: cli.params.print,
                },
                repo_url,
                MessageArgs {
                    complete: cli.params.complete,
                    prefix: cli.params.prefix,
                    context_ratio: cli.params.context_ratio,
                },
            ),
        )
        .await;

        let result = result.expect("LLM call should complete or fail within 90s, not hang");
        assert!(
            result.is_err(),
            "Staged files with --complete --prefix should reach LLM call"
        );
    }
}

mod dispatch {
    use cloy_message::MessageArgs;

    use super::*;

    #[tokio::test]
    #[ignore = "Requires LLM API access (fake API key triggers retry-backoff that exceeds test timeout)"]
    async fn complete_branch_takes_priority() {
        // Risk #1: dispatch logic — both branches reach the LLM (not crash on dispatch)
        let _lock = CWD_LOCK.lock().await;
        let repo = TestRepo::new();
        repo.stage_file("test.txt", "content");
        let _cwd = CwdGuard::new(&repo.path);

        let (cli_gen, repo_url_gen) = make_handler_args(&["git-message", "--print"]);
        let result_gen = timeout(
            Duration::from_secs(30),
            handle_message(
                cli_gen.common,
                CmsgConfig {
                    print_only: cli_gen.params.print,
                },
                repo_url_gen,
                MessageArgs {
                    complete: cli_gen.params.complete,
                    prefix: cli_gen.params.prefix,
                    context_ratio: cli_gen.params.context_ratio,
                },
            ),
        )
        .await
        .expect("Generate branch should complete within 30s");

        let (cli_comp, repo_url_comp) =
            make_handler_args(&["git-message", "--complete", "--prefix", "feat:", "--print"]);
        let result_comp = timeout(
            Duration::from_secs(30),
            handle_message(
                cli_comp.common,
                CmsgConfig {
                    print_only: cli_comp.params.print,
                },
                repo_url_comp,
                MessageArgs {
                    complete: cli_comp.params.complete,
                    prefix: cli_comp.params.prefix,
                    context_ratio: cli_comp.params.context_ratio,
                },
            ),
        )
        .await
        .expect("Complete branch should complete within 30s");

        assert!(
            result_gen.is_err(),
            "Generate mode with staged files should reach LLM"
        );
        assert!(
            result_comp.is_err(),
            "Complete mode with staged files should reach LLM"
        );
    }

    #[tokio::test]
    async fn generate_mode_rejects_command_line_positional() {
        // git-message has no positional args — verify clap rejects them before handler
        assert!(
            <CommonArgs as Parser>::try_parse_from(["git-message", "unexpected_positional"])
                .is_err(),
            "Positional arguments should be rejected by clap"
        );
    }
}
