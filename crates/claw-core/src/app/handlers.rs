use crate::commands::changelog::{
    ChangelogCommandConfig, handle_changelog_command, handle_release_notes_command,
};
use crate::commands::commit;
use crate::common::CommonParams;
use crate::sync::{
    check,
    common::{Parsed, TargetConfig, infer_from_url, normalize_github_url, sequence},
};

use super::args::{
    ChangelogArgs, CmsgConfig, Gitai, MessageArgs, WireArgs, WireCommand, WireSource,
};
use crate::llm::engine::init_tracing_to_file;
use anyhow::Result;
use colored::Colorize;
use log::debug;

pub async fn handle_command(command: Gitai, repository_url: Option<String>) -> Result<()> {
    init_tracing_to_file();

    match command {
        Gitai::Message { common, params } => {
            handle_message(
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
        }
        Gitai::Changelog { common, params } => {
            handle_changelog(
                common,
                ChangelogArgs {
                    from: params.from,
                    to: params.to,
                    repository_url,
                    update: params.update,
                    save: params.save,
                    file: params.file,
                    version_name: params.version_name,
                },
            )
            .await
        }
        Gitai::Notes { common, params } => {
            handle_notes(
                common,
                params.from,
                params.to,
                repository_url,
                params.version_name,
            )
            .await
        }
        Gitai::Pr { common, params } => {
            handle_pr_command(common, params.from, params.to, repository_url).await
        }
        Gitai::Wire(args) => handle_wire(args).await,
    }
}

pub async fn handle_message(
    common: CommonParams,
    config: CmsgConfig,
    repository_url: Option<String>,
    args: MessageArgs,
) -> Result<()> {
    debug!(
        "Handling 'message' command with common: {common:?}, print: {}, complete: {}, prefix: {:?}, context_ratio: {:?}",
        config.print_only, args.complete, args.prefix, args.context_ratio,
    );

    if args.complete {
        let prefix_text = args
            .prefix
            .ok_or_else(|| anyhow::anyhow!("Prefix is required for completion mode"))?;
        let context_ratio_val = args.context_ratio.unwrap_or(0.5);

        commit::handle_completion_command(
            common,
            prefix_text,
            Some(context_ratio_val),
            commit::MessageConfig {
                print: config.print_only,
            },
            repository_url,
        )
        .await
    } else {
        commit::handle_message_command(
            common,
            commit::MessageConfig {
                print: config.print_only,
            },
            repository_url,
        )
        .await
    }
}

pub async fn handle_changelog(common: CommonParams, args: ChangelogArgs) -> Result<()> {
    debug!(
        "Handling 'changelog' command with common: {common:?}, from: {:?}, to: {:?}, update: {}, save: {}, file: {:?}, version_name: {:?}",
        args.from, args.to, args.update, args.save, args.file, args.version_name
    );
    handle_changelog_command(
        common,
        ChangelogCommandConfig {
            from: args.from,
            to: args.to,
            repository_url: args.repository_url,
            update_file: args.update,
            save: args.save,
            changelog_path: args.file,
            version_name: args.version_name,
        },
    )
    .await
}

pub async fn handle_notes(
    common: CommonParams,
    from: String,
    to: Option<String>,
    repository_url: Option<String>,
    version_name: Option<String>,
) -> Result<()> {
    debug!(
        "Handling 'notes' command with common: {common:?}, from: {from}, to: {to:?}, version_name: {version_name:?}"
    );
    handle_release_notes_command(common, from, to, repository_url, version_name).await
}

pub async fn handle_pr_command(
    common: CommonParams,
    from: Option<String>,
    to: Option<String>,
    repository_url: Option<String>,
) -> Result<()> {
    debug!("Handling 'pr' command with common: {common:?}, from: {from:?}, to: {to:?}");
    commit::handle_pr_command(common, repository_url, from, to).await
}

pub async fn handle_wire(args: WireArgs) -> Result<()> {
    let target_name = args.target.or(args.name);

    let mode = if args.singlethread {
        sequence::Mode::Single
    } else {
        sequence::Mode::Parallel
    };

    let result = match args.command {
        WireCommand::Sync {
            source,
            save,
            no_save,
            append,
            global,
        } => {
            let has_cli_args = source.url.is_some() || !source.src.is_empty();
            let auto_save = has_cli_args && !no_save;
            let target_config =
                build_target_config(target_name, &source, save || auto_save, append, global)?;
            crate::sync::wire::operation::sync_with_caching(&target_config, mode).await
        }

        WireCommand::Check {
            source,
            save,
            no_save,
            append,
            global,
        } => {
            let has_cli_args = source.url.is_some() || !source.src.is_empty();
            let auto_save = has_cli_args && !no_save;
            let target_config =
                build_target_config(target_name, &source, save || auto_save, append, global)?;
            check::check(&target_config, &mode)
        }
    };

    match result {
        Ok(true) => {
            println!("{}", "Success".green().bold());
            Ok(())
        }
        Ok(false) => {
            println!("{}", "Failure".red().bold());
            Err(anyhow::anyhow!("Wire operation failed"))
        }
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

fn build_target_config(
    name_filter: Option<String>,
    source: &WireSource,
    save_config: bool,
    append_config: bool,
    global: bool,
) -> Result<TargetConfig> {
    let cli_override = build_parsed_from_cli(source);

    if let Some(ref parsed) = cli_override {
        parsed
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid arguments: {e}"))?;
    }

    Ok(TargetConfig {
        name_filter,
        cli_override,
        save_config,
        append_config,
        global,
    })
}

fn build_parsed_from_cli(source: &WireSource) -> Option<Parsed> {
    let url = &source.url;
    let rev = &source.rev;
    let src = &source.src;
    let dst = &source.dst;

    if url.is_none() && rev.is_none() && src.is_empty() && dst.is_none() {
        return None;
    }

    // Infer rev and src from URL if not explicitly provided
    let (inferred_rev, inferred_src) = url
        .as_ref()
        .and_then(|u| infer_from_url(u))
        .map_or((None, None), |(r, s)| (Some(r), Some(s)));

    // Use explicit values if provided, otherwise use inferred values
    let rev = rev.clone().or(inferred_rev);
    let src_paths: Vec<String> = if src.is_empty() {
        inferred_src.unwrap_or_default()
    } else if src.len() == 1 && src[0].trim().starts_with('[') {
        serde_json::from_str::<Vec<String>>(&src[0]).unwrap_or_else(|_| src.clone())
    } else {
        src.clone()
    };

    let mtd = source.method.clone();

    Some(Parsed {
        name: source.entry_name.clone(),
        dsc: source.description.clone(),
        url: source
            .url
            .as_ref()
            .map(|u| normalize_github_url(u))
            .unwrap_or_default(),
        rev: rev.unwrap_or_default(),
        src: src_paths,
        dst: source.dst.clone().unwrap_or_default(),
        mtd,
        last_sync_hash: None,
        merge_strategy: None,
    })
}
