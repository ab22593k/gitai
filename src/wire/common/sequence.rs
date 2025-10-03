use std::sync::Arc;

use cause::Cause;
use cause::cause;
use temp_dir::TempDir;

use super::ErrorType;
use super::ErrorType::NoItemToOperate;
use super::Parsed;
use super::Target;

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

pub fn sequence(
    target: Target,
    operation: &Arc<dyn Operation + Send + Sync>,
    mode: &Mode,
) -> Result<bool, Cause<ErrorType>> {
    let (rootdir, parsed): (String, Vec<_>) = match target {
        Target::Declared(Some(ref name)) => {
            let (rootdir, parsed) = super::parse::parse_gitwire()?;
            let parsed = parsed
                .into_iter()
                .filter(|p| match p.name {
                    Some(ref n) => n == name,
                    None => false,
                })
                .collect();
            (rootdir, parsed)
        }
        Target::Declared(None) => super::parse::parse_gitwire()?,
        Target::Direct(parsed) => (
            std::env::current_dir()
                .or(Err(cause!(ErrorType::CurrentDirRetrieve)))?
                .into_os_string()
                .into_string()
                .or(Err(cause!(ErrorType::CurrentDirConvert)))?,
            vec![parsed],
        ),
    };

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
