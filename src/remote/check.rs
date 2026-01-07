use std::{path::Path, sync::Arc};

use cause::{Cause, cause};
use colored::Colorize;
use folder_compare::FolderCompare;
use temp_dir::TempDir;

use crate::remote::common::{ErrorType, sequence};

use super::common::{
    ErrorType::{CheckDifferenceExecution, CheckDifferenceStringReplace},
    Parsed, Target,
    sequence::Operation,
};

#[derive(Debug)]
struct CheckOperation {}

impl Operation for CheckOperation {
    fn operate(
        &self,
        prefix: &str,
        parsed: &Parsed,
        rootdir: &str,
        tempdir: &TempDir,
    ) -> Result<bool, Cause<ErrorType>> {
        compare_with_temp(prefix, parsed, rootdir, tempdir.path())
    }
}

pub fn check(target: &Target, mode: &sequence::Mode) -> Result<bool, Cause<ErrorType>> {
    println!("git-wire check started\n");
    let operation: Arc<dyn Operation + Send + Sync + 'static> = Arc::new(CheckOperation {});
    let result = sequence::sequence(target, &operation, mode)?;
    Ok(result)
}

fn compare_with_temp(
    prefix: &str,
    parsed: &Parsed,
    root: &str,
    temp: &Path,
) -> Result<bool, Cause<ErrorType>> {
    println!("  - {prefix}compare `src` and `dst`");

    let mut result = true;
    let temp_root = temp;

    // Handle multiple source paths
    for src_path in &parsed.src {
        let temp_src = temp.join(src_path.as_str());
        let root_dst = Path::new(root).join(parsed.dst.as_str());

        let fc1 = FolderCompare::new(&temp_src, &root_dst, &vec![])
            .map_err(|_| cause!(CheckDifferenceExecution))?;
        let fc2 = FolderCompare::new(&root_dst, &temp_src, &vec![])
            .map_err(|_| cause!(CheckDifferenceExecution))?;

        if !fc1.new_files.is_empty() {
            let temp_root_str = temp_root
                .to_str()
                .ok_or_else(|| cause!(CheckDifferenceStringReplace))?;
            for file in fc1.new_files {
                let file = file
                    .to_str()
                    .ok_or_else(|| cause!(CheckDifferenceStringReplace))?;
                let file = file.replace(temp_root_str, "");
                println!(
                    "{}",
                    format!("    {prefix}! file {file} does not exist").red()
                );
            }
            result = false;
        }
        if !fc2.new_files.is_empty() {
            for file in fc2.new_files {
                println!(
                    "{}",
                    format!(
                        "    {prefix}! file {} does not exist on original",
                        file.display()
                    )
                    .red()
                );
            }
            result = false;
        }
        if !fc2.changed_files.is_empty() {
            for file in fc2.changed_files {
                println!(
                    "{}",
                    format!(
                        "    {prefix}! file {} is not identical to original",
                        file.display()
                    )
                    .red()
                );
            }
            result = false;
        }
    }

    Ok(result)
}
