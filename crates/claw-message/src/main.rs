use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use claw_core::{
    app::{
        args::{self, CmsgConfig, MessageArgs, MessageParams},
        handlers,
    },
    common::CommonParams,
    init_app,
    output::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-message",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a commit message using AI",
    after_help = args::get_dynamic_help(),
    styles = args::get_styles(),
)]
struct CliArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: MessageParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let cli_args = CliArgs::parse();
    let CliArgs { mut common, params } = cli_args;
    let repository_url = std::mem::take(&mut common.repository_url);

    if let Err(e) = handlers::handle_message(
        common,
        CmsgConfig {
            print_only: params.print,
        },
        repository_url,
        MessageArgs {
            complete: params.complete,
            prefix: params.prefix,
            context_ratio: params.context_ratio,
        },
    )
    .await
    {
        print_error(&format!("Error: {e}"));
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[test]
    fn verify_cli() {
        CliArgs::command().debug_assert();
    }

    #[test]
    fn test_cli_constraints() {
        // --prefix requires --complete
        let res = CliArgs::try_parse_from(["git-message", "--prefix", "test"]);
        assert!(res.is_err(), "Should fail: --prefix requires --complete");

        // --context-ratio requires --complete
        let res = CliArgs::try_parse_from(["git-message", "--context-ratio", "0.5"]);
        assert!(
            res.is_err(),
            "Should fail: --context-ratio requires --complete"
        );

        // --prefix and --complete should succeed
        let res = CliArgs::try_parse_from(["git-message", "--complete", "--prefix", "test"]);
        assert!(
            res.is_ok(),
            "Should succeed: --complete and --prefix together"
        );

        // --context-ratio and --complete should succeed
        let res = CliArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "0.5"]);
        assert!(
            res.is_ok(),
            "Should succeed: --complete and --context-ratio together"
        );
    }

    #[test]
    fn test_cli_invalid_float() {
        let res = CliArgs::try_parse_from([
            "git-message",
            "--complete",
            "--context-ratio",
            "not-a-float",
        ]);
        assert!(res.is_err(), "Should fail: --context-ratio must be a float");
    }

    #[test]
    fn test_cli_complex_combinations() {
        // Valid combination: print + complete + prefix + ratio
        let res = CliArgs::try_parse_from([
            "git-message",
            "--print",
            "--complete",
            "--prefix",
            "feat:",
            "--context-ratio",
            "0.7",
        ]);
        assert!(res.is_ok(), "Failed to parse args");
        let args = res.expect("Failed to parse args");
        assert!(args.params.print);
        assert!(args.params.complete);
        assert_eq!(args.params.prefix, Some("feat:".to_string()));
        assert_eq!(args.params.context_ratio, Some(0.7));

        // Valid: just print
        let res = CliArgs::try_parse_from(["git-message", "--print"]);
        assert!(res.is_ok(), "Failed to parse --print");
        assert!(res.expect("Failed to parse --print").params.print);

        // Valid: just complete (will fail in handler due to missing prefix, but CLI should allow)
        let res = CliArgs::try_parse_from(["git-message", "--complete"]);
        assert!(res.is_ok(), "Failed to parse --complete");
    }

    #[test]
    fn test_common_params_mapping() {
        // Test that global flags like --repo are parsed correctly
        let res =
            CliArgs::try_parse_from(["git-message", "--repo", "https://github.com/test/repo"]);
        assert!(res.is_ok(), "Failed to parse --repo");
        let args = res.expect("Failed to parse --repo");
        assert_eq!(
            args.common.repository_url,
            Some("https://github.com/test/repo".to_string())
        );
    }
}
