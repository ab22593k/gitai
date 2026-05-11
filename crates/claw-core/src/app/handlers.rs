use crate::sync::{
    check,
    common::{Parsed, TargetConfig, infer_from_url, normalize_github_url, sequence},
};

use super::args::{Gitai, WireArgs, WireCommand, WireSource};
use crate::llm::engine::init_tracing_to_file;
use anyhow::Result;
use colored::Colorize;

pub async fn handle_command(command: Gitai, _repository_url: Option<String>) -> Result<()> {
    init_tracing_to_file();

    match command {
        Gitai::Wire(args) => handle_wire(args).await,
    }
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
