use clap::{CommandFactory, Parser};
use cloy_message::{CmsgConfig, CommonArgs};

#[test]
fn verify_cli() {
    CommonArgs::command().debug_assert();
}

mod constraints {
    use super::*;

    #[test]
    fn prefix_requires_complete() {
        let res = CommonArgs::try_parse_from(["git-message", "--prefix", "test"]);
        assert!(res.is_err(), "--prefix without --complete should fail");
    }

    #[test]
    fn context_ratio_requires_complete() {
        let res = CommonArgs::try_parse_from(["git-message", "--context-ratio", "0.5"]);
        assert!(
            res.is_err(),
            "--context-ratio without --complete should fail"
        );
    }

    #[test]
    fn prefix_with_complete_succeeds() {
        let res = CommonArgs::try_parse_from(["git-message", "--complete", "--prefix", "test"]);
        assert!(res.is_ok(), "--complete --prefix should succeed");
    }

    #[test]
    fn context_ratio_with_complete_succeeds() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "0.5"]);
        assert!(res.is_ok(), "--complete --context-ratio should succeed");
    }

    #[test]
    fn print_and_complete_together() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--print",
            "--complete",
            "--prefix",
            "feat",
        ]);
        assert!(
            res.is_ok(),
            "--print --complete --prefix should be valid (print completion output)"
        );
    }

    #[test]
    fn context_ratio_rejects_out_of_range_low() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "-1.0"]);
        assert!(res.is_err(), "negative context ratio should be rejected");
    }

    #[test]
    fn context_ratio_rejects_out_of_range_high() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "2.0"]);
        assert!(res.is_err(), "context ratio > 1.0 should be rejected");
    }

    #[test]
    fn context_ratio_boundary_low() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "0.0"]);
        assert!(res.is_ok(), "0.0 is a valid boundary value");
        let args = res.expect("0.0 should parse");
        assert_eq!(args.params.context_ratio, Some(0.0));
    }

    #[test]
    fn context_ratio_boundary_high() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "1.0"]);
        assert!(res.is_ok(), "1.0 is a valid boundary value");
        let args = res.expect("1.0 should parse");
        assert_eq!(args.params.context_ratio, Some(1.0));
    }

    #[test]
    fn prefix_empty_string() {
        let res = CommonArgs::try_parse_from(["git-message", "--complete", "--prefix", ""]);
        assert!(res.is_ok(), "empty --prefix should be accepted by parser");
        let args = res.expect("empty prefix should parse");
        assert_eq!(args.params.prefix, Some(String::new()));
    }

    #[test]
    fn prefix_with_unicode() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--prefix", "feat(🌐):"]);
        assert!(res.is_ok(), "unicode in --prefix should be accepted");
        let args = res.expect("unicode prefix should parse");
        assert_eq!(args.params.prefix, Some("feat(🌐):".to_string()));
    }

    #[test]
    fn context_ratio_rejects_nan() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "NaN"]);
        assert!(res.is_err(), "NaN should be rejected");
    }

    #[test]
    fn context_ratio_rejects_inf() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--complete", "--context-ratio", "inf"]);
        assert!(res.is_err(), "inf should be rejected");
    }
}

mod invalid_inputs {
    use super::*;

    #[test]
    fn invalid_float() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--complete",
            "--context-ratio",
            "not-a-float",
        ]);
        assert!(res.is_err(), "--context-ratio must be a number");
    }

    #[test]
    fn invalid_detail_level() {
        let res = CommonArgs::try_parse_from(["git-message", "--detail-level", "bogus"]);
        assert!(res.is_err(), "invalid --detail-level should fail");
    }

    #[test]
    fn invalid_theme() {
        let res = CommonArgs::try_parse_from(["git-message", "--theme", "bogus"]);
        assert!(res.is_err(), "invalid --theme should fail");
    }

    #[test]
    fn unknown_flag() {
        let res = CommonArgs::try_parse_from(["git-message", "--nonexistent"]);
        assert!(res.is_err(), "unknown flag should fail");
    }

    #[test]
    fn flag_after_positional() {
        let res = CommonArgs::try_parse_from(["git-message", "positional", "--print"]);
        assert!(res.is_err(), "positional arguments should fail");
    }
}

mod complex_combinations {
    use super::*;

    #[test]
    fn print_complete_prefix_ratio() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--print",
            "--complete",
            "--prefix",
            "feat:",
            "--context-ratio",
            "0.7",
        ]);
        assert!(res.is_ok(), "full combo should parse");
        let args = res.expect("full combo should unwrap");
        assert!(args.params.print);
        assert!(args.params.complete);
        assert_eq!(args.params.prefix, Some("feat:".to_string()));
        assert_eq!(args.params.context_ratio, Some(0.7));
    }

    #[test]
    fn print_only() {
        let res = CommonArgs::try_parse_from(["git-message", "--print"]);
        assert!(res.is_ok(), "--print alone should parse");
        assert!(res.expect("--print alone should unwrap").params.print);
    }

    #[test]
    fn complete_only() {
        let res = CommonArgs::try_parse_from(["git-message", "--complete"]);
        assert!(
            res.is_ok(),
            "--complete alone should parse (handler validates prefix)"
        );
    }

    #[test]
    fn all_enums_minimal_and_dark() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--detail-level",
            "minimal",
            "--theme",
            "dark",
        ]);
        assert!(res.is_ok(), "minimal + dark should parse");
        let args = res.expect("minimal + dark should unwrap");
        assert_eq!(args.common.detail_level.as_str(), "minimal");
    }

    #[test]
    fn all_enums_detailed_and_light() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--detail-level",
            "detailed",
            "--theme",
            "light",
        ]);
        assert!(res.is_ok(), "detailed + light should parse");
    }

    #[test]
    fn model_and_instructions() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--model",
            "gemini-2.0-flash",
            "--instructions",
            "use emoji prefixes",
        ]);
        assert!(res.is_ok(), "--model --instructions should parse");
        let args = res.expect("model + instructions should unwrap");
        assert_eq!(args.common.model, Some("gemini-2.0-flash".to_string()));
        assert_eq!(
            args.common.instructions,
            Some("use emoji prefixes".to_string())
        );
    }

    #[test]
    fn complete_prefix_model_instructions() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--complete",
            "--prefix",
            "fix:",
            "--model",
            "gemini-2.0-flash",
            "--instructions",
            "short messages only",
        ]);
        assert!(res.is_ok(), "complex multi-flag should parse");
        let args = res.expect("complex multi-flag should unwrap");
        assert!(args.params.complete);
        assert_eq!(args.params.prefix, Some("fix:".to_string()));
    }
}

mod common_params {
    use super::*;

    #[test]
    fn repo_url() {
        let res =
            CommonArgs::try_parse_from(["git-message", "--repo", "https://github.com/test/repo"]);
        assert!(res.is_ok(), "--repo should parse");
        let args = res.expect("--repo url should unwrap");
        assert_eq!(
            args.common.repository_url,
            Some("https://github.com/test/repo".to_string())
        );
    }

    #[test]
    fn repo_url_with_complete() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--repo",
            "https://github.com/test/repo",
            "--complete",
            "--prefix",
            "fix:",
        ]);
        assert!(res.is_ok(), "--repo with --complete should parse");
        let args = res.expect("--repo with --complete should unwrap");
        assert_eq!(
            args.common.repository_url,
            Some("https://github.com/test/repo".to_string())
        );
        assert!(args.params.complete);
    }

    #[test]
    fn repo_url_short_form() {
        let res = CommonArgs::try_parse_from(["git-message", "-r", "https://github.com/test/repo"]);
        assert!(res.is_ok(), "-r short form should parse");
    }

    #[test]
    fn instructions_short_form() {
        let res = CommonArgs::try_parse_from(["git-message", "-i", "focus on tests"]);
        assert!(res.is_ok(), "-i short form should parse");
        let args = res.expect("-i should unwrap");
        assert_eq!(args.common.instructions, Some("focus on tests".to_string()));
    }

    #[test]
    fn defaults() {
        let res = CommonArgs::try_parse_from(["git-message"]);
        assert!(res.is_ok(), "no args should produce defaults");
        let args = res.expect("defaults should unwrap");
        assert!(!args.params.print);
        assert!(!args.params.complete);
        assert_eq!(args.params.prefix, None);
        assert_eq!(args.params.context_ratio, None);
        assert_eq!(args.common.repository_url, None);
        assert_eq!(args.common.model, None);
        assert_eq!(args.common.instructions, None);
    }
}

mod data_flow {
    use cloy_message::MessageArgs;

    use super::*;

    #[test]
    fn print_flag_survives_into_cmsg_config() {
        let res = CommonArgs::try_parse_from(["git-message", "--print"]);
        assert!(res.is_ok());
        let args = res.expect("--print should unwrap");
        let config = CmsgConfig {
            print_only: args.params.print,
        };
        assert!(
            config.print_only,
            "--print must map to CmsgConfig.print_only"
        );
    }

    #[test]
    fn message_args_assembled_correctly_for_complete() {
        let res = CommonArgs::try_parse_from([
            "git-message",
            "--complete",
            "--prefix",
            "fix(api): ",
            "--context-ratio",
            "0.3",
        ]);
        assert!(res.is_ok());
        let args = res.expect("complete flags should unwrap");
        let message_args = MessageArgs {
            complete: args.params.complete,
            prefix: args.params.prefix,
            context_ratio: args.params.context_ratio,
        };
        assert!(message_args.complete);
        assert_eq!(message_args.prefix, Some("fix(api): ".to_string()));
        assert_eq!(message_args.context_ratio, Some(0.3));
    }

    #[test]
    fn message_args_for_default_no_flags() {
        let res = CommonArgs::try_parse_from(["git-message"]);
        assert!(res.is_ok());
        let args = res.expect("default no flags should unwrap");
        let message_args = MessageArgs {
            complete: args.params.complete,
            prefix: args.params.prefix,
            context_ratio: args.params.context_ratio,
        };
        assert!(!message_args.complete);
        assert_eq!(message_args.prefix, None);
        assert_eq!(message_args.context_ratio, None);
    }
}
