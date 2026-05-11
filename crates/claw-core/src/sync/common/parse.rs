use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use cause::Cause;
use cause::cause;
use colored::Colorize;
use git2::Config as GitConfig;

use super::ErrorType::{
    self, DotGitWireFileNameNotUnique, DotGitWireFileOpen, DotGitWireFileParse,
    DotGitWireFileSoundness, DotGitWireFileWrite,
};
use super::{MergeStrategy, Method, Parsed, is_path_sound};

const GITWIRE_FILENAME: &str = ".gitwire";
const GITWIRE_CONFIG_PREFIX: &str = "wire";

#[derive(Debug)]
enum ParseError {
    GitConfig(git2::Error),
    Custom(String),
}

impl From<ParseError> for Cause<ErrorType> {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::GitConfig(e) => {
                cause!(DotGitWireFileOpen, "Git config error reading .gitwire").src(e)
            }
            ParseError::Custom(msg) => cause!(DotGitWireFileParse, msg),
        }
    }
}

fn get_gitwire_path(base_dir: &Path, global: bool) -> PathBuf {
    if global {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(GITWIRE_FILENAME)
    } else {
        base_dir.join(GITWIRE_FILENAME)
    }
}

fn open_gitwire_config(base_dir: &Path, global: bool) -> Result<GitConfig, ParseError> {
    let config_path = get_gitwire_path(base_dir, global);

    if !config_path.exists() {
        return Err(ParseError::Custom("Config file does not exist".to_string()));
    }

    GitConfig::open(&config_path).map_err(ParseError::GitConfig)
}

pub fn parse_gitwire(
    base_dir: &Path,
    global: bool,
) -> Result<Option<Vec<Parsed>>, Cause<ErrorType>> {
    let config_path = get_gitwire_path(base_dir, global);

    if !config_path.exists() {
        return Ok(None);
    }

    let config = match open_gitwire_config(base_dir, global) {
        Ok(c) => c,
        Err(e) => {
            return Err(Cause::from(e));
        }
    };

    let entries = parse_wire_entries(&config)?;
    Ok(Some(entries))
}

fn parse_wire_entries(config: &GitConfig) -> Result<Vec<Parsed>, Cause<ErrorType>> {
    let mut entries = Vec::new();

    let prefix = format!("{GITWIRE_CONFIG_PREFIX}.");

    if let Ok(mut iter) = config.entries(Some(&prefix)) {
        let mut current_entry: Option<Parsed> = None;
        let mut current_subsection: Option<String> = None;

        while let Some(entry_result) = iter.next() {
            let Ok(entry) = entry_result else { continue };

            let Some(name) = entry.name() else { continue };
            let Some(value) = entry.value() else { continue };

            let after_prefix = name.strip_prefix(&prefix).unwrap_or(name);

            if after_prefix.contains('.') {
                let parts: Vec<&str> = after_prefix.splitn(2, '.').collect();
                let subsection = parts[0].to_string();
                let key = parts.get(1).unwrap_or(&"").to_string();

                if current_subsection.as_ref() != Some(&subsection) {
                    if let Some(entry) = current_entry.take() {
                        entries.push(entry);
                    }
                    current_subsection = Some(subsection.clone());
                    current_entry = Some(Parsed {
                        name: Some(subsection),
                        dsc: None,
                        url: String::new(),
                        rev: String::new(),
                        src: Vec::new(),
                        dst: String::new(),
                        mtd: None,
                        last_sync_hash: None,
                        merge_strategy: None,
                    });
                }

                if let Some(ref mut entry) = current_entry {
                    match key.as_str() {
                        "name" => entry.name = Some(value.to_string()),
                        "description" | "dsc" => entry.dsc = Some(value.to_string()),
                        "url" => entry.url = value.to_string(),
                        "rev" => entry.rev = value.to_string(),
                        "dst" => entry.dst = value.to_string(),
                        "src" => entry.src = vec![value.to_string()],
                        "method" => entry.mtd = parse_method(value),
                        "last-sync-hash" | "last_sync_hash" => {
                            entry.last_sync_hash = Some(value.to_string());
                        }
                        "merge-strategy" | "merge_strategy" => {
                            entry.merge_strategy = parse_merge_strategy(value);
                        }
                        _ => {}
                    }
                }
            } else {
                if current_entry.is_none() {
                    current_entry = Some(Parsed {
                        name: None,
                        dsc: None,
                        url: String::new(),
                        rev: String::new(),
                        src: Vec::new(),
                        dst: String::new(),
                        mtd: None,
                        last_sync_hash: None,
                        merge_strategy: None,
                    });
                }

                if let Some(ref mut entry) = current_entry {
                    match after_prefix {
                        "name" => entry.name = Some(value.to_string()),
                        "description" | "dsc" => entry.dsc = Some(value.to_string()),
                        "url" => entry.url = value.to_string(),
                        "rev" => entry.rev = value.to_string(),
                        "dst" => entry.dst = value.to_string(),
                        "src" => entry.src = vec![value.to_string()],
                        "method" => entry.mtd = parse_method(value),
                        "last-sync-hash" | "last_sync_hash" => {
                            entry.last_sync_hash = Some(value.to_string());
                        }
                        "merge-strategy" | "merge_strategy" => {
                            entry.merge_strategy = parse_merge_strategy(value);
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(entry) = current_entry {
            entries.push(entry);
        }
    }

    for (i, entry) in entries.iter().enumerate() {
        validate_entry_paths(entry, i)?;
    }

    validate_unique_names(&entries)?;

    Ok(entries)
}

fn parse_method(value: &str) -> Option<Method> {
    match value {
        "shallow" => Some(Method::Shallow),
        "shallow_no_sparse" => Some(Method::ShallowNoSparse),
        "partial" => Some(Method::Partial),
        _ => None,
    }
}

fn parse_merge_strategy(value: &str) -> Option<MergeStrategy> {
    match value {
        "overwrite" => Some(MergeStrategy::Overwrite),
        "auto" => Some(MergeStrategy::Auto),
        "manual" => Some(MergeStrategy::Manual),
        "ai" => Some(MergeStrategy::Ai),
        _ => None,
    }
}

pub fn save_to_gitwire(
    base_dir: &Path,
    global: bool,
    entry: &Parsed,
    append: bool,
) -> Result<(), Cause<ErrorType>> {
    let config_path = get_gitwire_path(base_dir, global);
    let parent = config_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|e| cause!(DotGitWireFileOpen, "Failed to create directory").src(e))?;

    let mut content = if append && config_path.exists() {
        fs::read_to_string(&config_path).unwrap_or_else(|e| {
            log::warn!("Failed to read existing .gitwire for append: {e}; starting fresh");
            String::new()
        })
    } else {
        String::new()
    };

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if !content.is_empty() {
        content.push('\n');
    }

    let subsection = entry
        .name
        .clone()
        .or_else(|| Some(entry.dst.clone()))
        .unwrap_or_default();

    if subsection.is_empty() {
        let _ = writeln!(content, "[{GITWIRE_CONFIG_PREFIX}]");
    } else {
        let _ = writeln!(content, "[{GITWIRE_CONFIG_PREFIX} \"{subsection}\"]");
    }

    let _ = writeln!(content, "    url = {}", entry.url);

    if !entry.rev.is_empty() {
        let _ = writeln!(content, "    rev = {}", entry.rev);
    }

    if !entry.dst.is_empty() {
        let _ = writeln!(content, "    dst = {}", entry.dst);
    }

    if !entry.src.is_empty() {
        let _ = writeln!(content, "    src = {}", entry.src[0]);
    }

    if let Some(ref dsc) = entry.dsc
        && !dsc.is_empty()
    {
        let _ = writeln!(content, "    description = {dsc}");
    }

    if let Some(ref mtd) = entry.mtd {
        let method_str = match mtd {
            Method::Shallow => "shallow",
            Method::ShallowNoSparse => "shallow_no_sparse",
            Method::Partial => "partial",
        };
        let _ = writeln!(content, "    method = {method_str}");
    }

    if let Some(ref last_hash) = entry.last_sync_hash
        && !last_hash.is_empty()
    {
        let _ = writeln!(content, "    last-sync-hash = {last_hash}");
    }

    if let Some(ref strategy) = entry.merge_strategy {
        let strategy_str = match strategy {
            MergeStrategy::Overwrite => "overwrite",
            MergeStrategy::Auto => "auto",
            MergeStrategy::Manual => "manual",
            MergeStrategy::Ai => "ai",
        };
        let _ = writeln!(content, "    merge-strategy = {strategy_str}");
    }

    fs::write(&config_path, content)
        .map_err(|e| cause!(DotGitWireFileWrite, "Failed to write .gitwire").src(e))?;

    println!("{}", "Configuration saved to .gitwire".green());

    Ok(())
}

fn validate_entry_paths(entry: &Parsed, i: usize) -> Result<(), Cause<ErrorType>> {
    if !is_path_sound(&entry.dst) {
        return Err(cause!(
            DotGitWireFileSoundness,
            format!("Entry {i}: dst path is not sound")
        ));
    }
    for src in &entry.src {
        if !is_path_sound(src) {
            return Err(cause!(
                DotGitWireFileSoundness,
                format!("Entry {i}: src path is not sound")
            ));
        }
    }
    Ok(())
}

fn validate_unique_names(parsed: &[Parsed]) -> Result<(), Cause<ErrorType>> {
    let mut name_set: HashSet<&str> = HashSet::new();
    for (i, p) in parsed.iter().enumerate() {
        if let Some(ref name) = p.name
            && !name_set.insert(name.as_str())
        {
            Err(cause!(
                DotGitWireFileNameNotUnique,
                format!("Entry {i}: name '{name}' is not unique")
            ))?;
        }
    }
    Ok(())
}
