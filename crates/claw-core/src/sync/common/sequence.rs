use std::sync::Arc;

use cause::Cause;
use cause::cause;
use temp_dir::TempDir;

use super::ErrorType;
use super::ErrorType::NoItemToOperate;
use super::Parsed;
use super::TargetConfig;
use super::merge_parsed;

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

/// Get parsed items based on `TargetConfig`
fn get_parsed_from_config(
    config: &TargetConfig,
) -> Result<(String, Vec<Parsed>, Option<Parsed>), Cause<ErrorType>> {
    let root = std::env::current_dir()
        .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
        .clone();

    let gitwire_data = super::parse::parse_gitwire(&root, config.global)?;

    match (gitwire_data, &config.cli_override) {
        // Both .gitwire and CLI args provided
        (Some(mut file_items), Some(cli_parsed)) => {
            if let Some(name) = &config.name_filter {
                if let Some(entry) = file_items
                    .iter_mut()
                    .find(|p| p.name.as_ref() == Some(name))
                {
                    merge_parsed(entry, cli_parsed);
                    let matched = entry.clone();
                    file_items.retain(|p| p.name.as_ref() == Some(name));
                    Ok((
                        root.to_string_lossy().to_string(),
                        file_items,
                        Some(matched),
                    ))
                } else {
                    Ok((
                        root.to_string_lossy().to_string(),
                        vec![cli_parsed.clone()],
                        Some(cli_parsed.clone()),
                    ))
                }
            } else {
                Ok((
                    root.to_string_lossy().to_string(),
                    vec![cli_parsed.clone()],
                    Some(cli_parsed.clone()),
                ))
            }
        }

        // Only .gitwire exists
        (Some(mut file_items), None) => {
            if let Some(name) = &config.name_filter {
                file_items.retain(|p| p.name.as_ref() == Some(name));
                if file_items.is_empty() {
                    return Err(cause!(
                        NoItemToOperate,
                        format!("No entry with name '{name}' found in .gitwire")
                    ));
                }
            }
            Ok((root.to_string_lossy().to_string(), file_items, None))
        }

        // Only CLI args provided (no .gitwire)
        (None, Some(cli_parsed)) => Ok((
            root.to_string_lossy().to_string(),
            vec![cli_parsed.clone()],
            Some(cli_parsed.clone()),
        )),

        // Neither provided
        (None, None) => Err(cause!(
            NoItemToOperate,
            "No .gitwire file found and no CLI arguments provided.\n\
             \nUsage examples:\n\
             \n  git-wire check --url <URL> --rev <REV> --src <SRC> --dst <DST>\n\
             \n  git-wire check --url <URL> --rev <REV> --src '[\"lib\",\"tools\"]' --dst <DST>\n\
             \n  git-wire check  # Interactive mode"
        )),
    }
}

pub fn sequence(
    config: &TargetConfig,
    operation: &Arc<dyn Operation + Send + Sync>,
    mode: &Mode,
) -> Result<bool, Cause<ErrorType>> {
    let (rootdir, parsed, cli_parsed_for_save): (String, Vec<_>, Option<Parsed>) = {
        let (root, items, cli_parsed) = get_parsed_from_config(config)?;

        if config.save_config
            && let Some(ref p) = cli_parsed
        {
            let root_path = std::env::current_dir()
                .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                .clone();
            super::parse::save_to_gitwire(&root_path, config.global, p, config.append_config)?;
        }

        (root, items, cli_parsed)
    };

    // cli_parsed_for_save is consumed above by the save_config branch;
    // it remains when save_config is false, in which case we intentionally discard it.
    drop(cli_parsed_for_save);

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
            .map(|h| {
                h.join().unwrap_or_else(|_| {
                    Err(cause!(
                        ErrorType::NoItemToOperate,
                        "A thread panicked during execution"
                    ))
                })
            })
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
