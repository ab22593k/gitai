use std::sync::Arc;

use cause::Cause;
use cause::cause;
use temp_dir::TempDir;

use super::ErrorType;
use super::ErrorType::NoItemToOperate;
use super::Parsed;
use super::Target;
use super::TargetConfig;

pub enum Mode {
    Single,
    Parallel,
}

pub trait Operation {
    fn operate(
        &self,
        prefix: &str,
        parsed: &Parsed,
        rootdir: &str,
        tempdir: &TempDir,
    ) -> Result<bool, Cause<ErrorType>>;
}

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

/// Get parsed items based on `TargetConfig`
fn get_parsed_from_config(
    config: &TargetConfig,
) -> Result<(String, Vec<Parsed>, Option<Parsed>), Cause<ErrorType>> {
    // Try to parse .gitwire.toml
    let gitwire_data = super::parse::parse_gitwire()?;

    match (gitwire_data, &config.cli_override) {
        // Both .gitwire.toml and CLI args provided
        (Some((root, mut file_items)), Some(cli_parsed)) => {
            if let Some(name) = &config.name_filter {
                // Try to find and override entry by name
                if let Some(entry) = file_items
                    .iter_mut()
                    .find(|p| p.name.as_ref() == Some(name))
                {
                    merge_parsed(entry, cli_parsed);
                    let matched = entry.clone();
                    file_items.retain(|p| p.name.as_ref() == Some(name));
                    Ok((root, file_items, Some(matched)))
                } else {
                    // Name not found, use CLI args as new entry
                    Ok((root, vec![cli_parsed.clone()], Some(cli_parsed.clone())))
                }
            } else {
                // No name filter: use CLI args only
                Ok((root, vec![cli_parsed.clone()], Some(cli_parsed.clone())))
            }
        }

        // Only .gitwire.toml exists
        (Some((root, mut file_items)), None) => {
            if let Some(name) = &config.name_filter {
                file_items.retain(|p| p.name.as_ref() == Some(name));
                if file_items.is_empty() {
                    return Err(cause!(
                        NoItemToOperate,
                        format!("No entry with name '{name}' found in .gitwire.toml")
                    ));
                }
            }
            Ok((root, file_items, None))
        }

        // Only CLI args provided (no .gitwire.toml)
        (None, Some(cli_parsed)) => {
            let root = std::env::current_dir()
                .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                .into_os_string()
                .into_string()
                .or(Err(cause!(ErrorType::CurrentDirConvert)))?;
            Ok((root, vec![cli_parsed.clone()], Some(cli_parsed.clone())))
        }

        // Neither provided - show interactive prompt
        (None, None) => match super::parse::prompt_create_gitwire()? {
            Some(parsed) => {
                let root = std::env::current_dir()
                    .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                    .into_os_string()
                    .into_string()
                    .or(Err(cause!(ErrorType::CurrentDirConvert)))?;

                // Save the prompted config
                super::parse::save_to_gitwire_toml(&parsed, false)?;

                Ok((root, vec![parsed.clone()], Some(parsed)))
            }
            None => Err(cause!(
                NoItemToOperate,
                "No .gitwire.toml file found and no CLI arguments provided.\n\
                 \nUsage examples:\n\
                 \n  git-wire check --url <URL> --rev <REV> --src <SRC> --dst <DST>\n\
                 \n  git-wire check --url <URL> --rev <REV> --src '[\"lib\",\"tools\"]' --dst <DST>\n\
                 \n  git-wire check  # Interactive mode"
            )),
        },
    }
}

pub fn sequence(
    target: &Target,
    operation: &Arc<dyn Operation + Send + Sync>,
    mode: &Mode,
) -> Result<bool, Cause<ErrorType>> {
    let (rootdir, parsed, cli_parsed_for_save): (String, Vec<_>, Option<Parsed>) = match target {
        Target::Declared(config) => {
            let (root, items, cli_parsed) = get_parsed_from_config(config)?;

            // Handle --save flag if applicable
            if config.save_config
                && let Some(ref p) = cli_parsed
            {
                super::parse::save_to_gitwire_toml(p, config.append_config)?;
            }

            (root, items, cli_parsed)
        }
    };

    // Suppress unused variable warning
    let _ = cli_parsed_for_save;

    let len = parsed.len();
    if len == 0 {
        Err(cause!(NoItemToOperate, "There are no items to operate."))?;
    }

    match mode {
        Mode::Single => single(parsed.as_slice(), rootdir.as_str(), operation.as_ref()),
        Mode::Parallel => parallel(parsed, rootdir.as_str(), operation),
    }
}

fn single(
    parsed: &[Parsed],
    rootdir: &str,
    operation: &dyn Operation,
) -> Result<bool, Cause<ErrorType>> {
    let len = parsed.len();

    let mut result = true;
    for (i, parsed) in parsed.iter().enumerate() {
        println!(">> {}/{} started{}", i + 1, len, additional_message(parsed));
        let tempdir = super::fetch::fetch_target_to_tempdir("", parsed)?;
        let success = operation.operate("", parsed, rootdir, &tempdir)?;
        if !success {
            result = false;
        }
    }
    println!(">> All check tasks have done!\n");
    Ok(result)
}

fn parallel(
    parsed: Vec<Parsed>,
    rootdir: &str,
    operation: &Arc<dyn Operation + Send + Sync>,
) -> Result<bool, Cause<ErrorType>> {
    use colored::Colorize;

    let len = parsed.len();
    let operation = operation.clone();

    let results: Vec<_> = std::thread::scope(|s| {
        let results: Vec<_> = parsed
            .into_iter()
            .enumerate()
            .map(|(i, parsed)| {
                s.spawn({
                    let operation = operation.clone();
                    move || -> Result<bool, Cause<ErrorType>> {
                        let prefix = format!("No.{i} ");
                        println!(
                            "{}",
                            format!(
                                ">> {prefix}({}/{len}) started{}",
                                i + 1,
                                additional_message(&parsed)
                            )
                            .blue()
                        );
                        let tempdir = super::fetch::fetch_target_to_tempdir(&prefix, &parsed)?;
                        let success = operation.operate(&prefix, &parsed, rootdir, &tempdir)?;
                        if success {
                            println!(
                                "{}",
                                format!(
                                    ">> {prefix}({}/{len}) succeeded{}",
                                    i + 1,
                                    additional_message(&parsed)
                                )
                                .blue()
                            );
                            Ok(true)
                        } else {
                            println!(
                                "{}",
                                format!(
                                    ">> {prefix}({}/{len}) failed{}",
                                    i + 1,
                                    additional_message(&parsed)
                                )
                                .magenta()
                            );
                            Ok(false)
                        }
                    }
                })
            })
            .collect();
        results
            .into_iter()
            .map(|h| h.join().expect("A thread panicked during execution"))
            .collect()
    });
    println!("{}", ">> All check tasks have done!\n".to_string().blue());

    let result = if results.iter().any(|r| matches!(r, Ok(false))) {
        Ok(false)
    } else {
        Ok(true)
    };
    if let Some(err) = results.into_iter().find(|r| matches!(r, Err(..))) {
        return err;
    }
    result
}

fn additional_message(parsed: &Parsed) -> String {
    match (&parsed.name, &parsed.dsc) {
        (Some(name), Some(dsc)) => format!(" ({name}: {dsc})"),
        (Some(name), None) => format!(" ({name})"),
        (None, Some(dsc)) => format!(" ({dsc})"),
        (None, None) => String::new(),
    }
}
