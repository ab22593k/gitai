use std::collections::HashSet;
use std::fs;
use std::path::Path;

use cause::Cause;
use cause::cause;
use colored::Colorize;
use git2::Repository;
use inquire::{Confirm, Text};
use toml::de::Error as TomlError;
use toml::value::Array;

use super::ErrorType::{
    self, DotGitWireFileNameNotUnique, DotGitWireFileOpen, DotGitWireFileParse,
    DotGitWireFileSoundness, DotGitWireFileWrite, PromptError, RepositoryRootPathCommand,
};
use super::{Method, Parsed, is_path_sound};

const DOT_GIT_WIRE: &str = ".gitwire.toml";

#[derive(Debug)]
enum ParseError {
    Io(std::io::Error),
    Toml(TomlError),
    Custom(String),
}

impl ParseError {
    fn from_str(s: &str) -> Self {
        ParseError::Custom(s.to_string())
    }
}

impl From<ParseError> for Cause<ErrorType> {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::Io(e) => {
                cause!(DotGitWireFileOpen, "IO error reading .gitwire.toml").src(e)
            }
            ParseError::Toml(e) => cause!(
                DotGitWireFileParse,
                format!(".gitwire.toml format error: {e}")
            ),
            ParseError::Custom(msg) => cause!(DotGitWireFileParse, msg),
        }
    }
}

/// Parse .gitwire.toml file if it exists
/// Returns None if the file doesn't exist (not an error)
pub fn parse_gitwire() -> Result<Option<(String, Vec<Parsed>)>, Cause<ErrorType>> {
    match get_dotgitwire_file_path()? {
        Some((root, file)) => {
            let parsed = parse_dotgitwire_file(&file)?;
            Ok(Some((root, parsed)))
        }
        None => Ok(None),
    }
}

/// Get the repository root directory
pub fn get_repo_root() -> Result<String, Cause<ErrorType>> {
    let repo = Repository::discover(".").map_err(|e| cause!(RepositoryRootPathCommand).src(e))?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| cause!(RepositoryRootPathCommand))?;
    Ok(workdir.to_string_lossy().to_string())
}

/// Get path to .gitwire.toml if it exists
/// Returns None if file doesn't exist (not an error)
fn get_dotgitwire_file_path() -> Result<Option<(String, String)>, Cause<ErrorType>> {
    let repo = Repository::discover(".").map_err(|e| cause!(RepositoryRootPathCommand).src(e))?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| cause!(RepositoryRootPathCommand))?;
    let root = workdir.to_string_lossy().to_string();

    let file = format!("{root}/{DOT_GIT_WIRE}");
    if !Path::new(&file).exists() {
        return Ok(None);
    }
    Ok(Some((root, file)))
}

fn parse_dotgitwire_file(file: &str) -> Result<Vec<Parsed>, Cause<ErrorType>> {
    let content = read_file_content(file)?;
    let entries = extract_wire_entries(&content)?;
    parse_entries(&entries)
}

fn read_file_content(file: &str) -> Result<String, ParseError> {
    fs::read_to_string(file).map_err(ParseError::Io)
}

fn extract_wire_entries(content: &str) -> Result<Array, ParseError> {
    let value: toml::Value = toml::from_str(content).map_err(ParseError::Toml)?;
    let table = value
        .as_table()
        .ok_or_else(|| ParseError::from_str("Root must be a table"))?;

    let wire_section = table
        .get("wire")
        .ok_or_else(|| ParseError::from_str("Missing [wire] section"))?;

    let wire_table = wire_section
        .as_table()
        .ok_or_else(|| ParseError::from_str("[wire] must be a table"))?;

    let entries_section = wire_table
        .get("entries")
        .ok_or_else(|| ParseError::from_str("Missing entries array in [wire]"))?;

    entries_section
        .as_array()
        .ok_or_else(|| ParseError::from_str("entries must be an array"))
        .cloned()
}

fn parse_entries(entries: &Array) -> Result<Vec<Parsed>, Cause<ErrorType>> {
    let mut parsed: Vec<Parsed> = Vec::with_capacity(entries.len());

    for (i, entry_value) in entries.iter().enumerate() {
        let entry_table = entry_value.as_table().ok_or_else(|| {
            cause!(
                DotGitWireFileParse,
                format!("Entry {i} in [wire].entries must be a table")
            )
        })?;

        let name = entry_table
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from);
        let dsc = entry_table
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);
        let url = entry_table
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| cause!(DotGitWireFileParse, format!("Entry {i}: 'url' is required")))?
            .to_string();
        let rev = entry_table
            .get("rev")
            .and_then(|v| v.as_str())
            .ok_or_else(|| cause!(DotGitWireFileParse, format!("Entry {i}: 'rev' is required")))?
            .to_string();

        // Handle src as string or array
        let src = match entry_table.get("src") {
            Some(toml::Value::String(s)) => vec![s.clone()],
            Some(toml::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            _ => {
                return Err(cause!(
                    DotGitWireFileParse,
                    format!(
                        "Entry {i}: 'src' is required and must be a string or array of strings"
                    )
                ));
            }
        };

        if src.is_empty() {
            return Err(cause!(
                DotGitWireFileParse,
                format!("Entry {i}: 'src' must have at least one path")
            ));
        }

        let dist = entry_table
            .get("dst")
            .and_then(|v| v.as_str())
            .ok_or_else(|| cause!(DotGitWireFileParse, format!("Entry {i}: 'dst' is required")))?
            .to_string();

        let mtd = match entry_table.get("method").and_then(|v| v.as_str()) {
            Some("shallow") => Some(Method::Shallow),
            Some("shallow_no_sparse") => Some(Method::ShallowNoSparse),
            Some("partial") => Some(Method::Partial),
            Some(s) => {
                return Err(cause!(
                    DotGitWireFileParse,
                    format!("Entry {i}: unknown method '{s}'")
                ));
            }
            None => None,
        };

        let item = Parsed {
            name,
            dsc,
            url,
            rev,
            src,
            dst: dist,
            mtd,
        };

        // Validate all src paths
        for s in &item.src {
            if !is_path_sound(s) {
                return Err(cause!(
                    DotGitWireFileSoundness,
                    format!("Entry {i}: src path '{s}' must not include '.', '..', or '.git'.")
                ));
            }
        }

        if !is_path_sound(&item.dst) {
            return Err(cause!(
                DotGitWireFileSoundness,
                format!(
                    "Entry {i}: dst path '{}' must not include '.', '..', or '.git'.",
                    item.dst
                )
            ));
        }

        parsed.push(item);
    }

    validate_unique_names(&parsed)?;

    Ok(parsed)
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

// ============================================================================
// SAVE FUNCTIONALITY
// ============================================================================

/// Save a Parsed entry to .gitwire.toml
pub fn save_to_gitwire_toml(parsed: &Parsed, append: bool) -> Result<(), Cause<ErrorType>> {
    let repo = Repository::discover(".").map_err(|e| cause!(RepositoryRootPathCommand).src(e))?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| cause!(RepositoryRootPathCommand))?;
    let file_path = workdir.join(DOT_GIT_WIRE);

    if append && file_path.exists() {
        append_entry_to_toml(&file_path, parsed)?;
    } else {
        create_gitwire_toml(&file_path, &[parsed])?;
    }

    println!("{}", "Configuration saved to .gitwire.toml".green());
    Ok(())
}

fn create_gitwire_toml(path: &Path, entries: &[&Parsed]) -> Result<(), Cause<ErrorType>> {
    let mut content = String::from("[wire]\nentries = [\n");

    for entry in entries {
        content.push_str(&format_entry(entry));
    }

    content.push_str("]\n");

    fs::write(path, content)
        .map_err(|e| cause!(DotGitWireFileWrite, "Failed to write .gitwire.toml").src(e))?;

    Ok(())
}

fn append_entry_to_toml(path: &Path, entry: &Parsed) -> Result<(), Cause<ErrorType>> {
    // Read existing file
    let content = fs::read_to_string(path)
        .map_err(|e| cause!(DotGitWireFileOpen, "Failed to read .gitwire.toml").src(e))?;

    // Parse existing TOML
    let mut toml_value: toml::Value = toml::from_str(&content).map_err(|e| {
        cause!(
            DotGitWireFileParse,
            "Failed to parse existing .gitwire.toml"
        )
        .src(e)
    })?;

    // Get or create wire.entries array
    let entries_array = toml_value
        .get_mut("wire")
        .and_then(|w| w.as_table_mut())
        .and_then(|t| t.get_mut("entries"))
        .and_then(|e| e.as_array_mut())
        .ok_or_else(|| cause!(DotGitWireFileParse, "Invalid .gitwire.toml structure"))?;

    // Convert Parsed to TOML value
    let new_entry = parsed_to_toml_value(entry);
    entries_array.push(new_entry);

    // Write back
    let new_content = toml::to_string_pretty(&toml_value)
        .map_err(|e| cause!(DotGitWireFileParse, "Failed to serialize TOML").src(e))?;

    fs::write(path, new_content)
        .map_err(|e| cause!(DotGitWireFileWrite, "Failed to write .gitwire.toml").src(e))?;

    Ok(())
}

fn format_entry(entry: &Parsed) -> String {
    let mut parts = Vec::new();

    if let Some(name) = &entry.name {
        parts.push(format!("name = \"{}\"", escape_toml_string(name)));
    }
    if let Some(dsc) = &entry.dsc {
        parts.push(format!("description = \"{}\"", escape_toml_string(dsc)));
    }
    parts.push(format!("url = \"{}\"", escape_toml_string(&entry.url)));
    parts.push(format!("rev = \"{}\"", escape_toml_string(&entry.rev)));

    // Handle src as array or single string
    if entry.src.len() == 1 {
        parts.push(format!("src = \"{}\"", escape_toml_string(&entry.src[0])));
    } else {
        let src_array = entry
            .src
            .iter()
            .map(|s| format!("\"{}\"", escape_toml_string(s)))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("src = [{src_array}]"));
    }

    parts.push(format!("dst = \"{}\"", escape_toml_string(&entry.dst)));

    if let Some(mtd) = &entry.mtd {
        let method_str = match mtd {
            Method::Shallow => "shallow",
            Method::ShallowNoSparse => "shallow_no_sparse",
            Method::Partial => "partial",
        };
        parts.push(format!("method = \"{method_str}\""));
    }

    format!("    {{ {} }},\n", parts.join(", "))
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parsed_to_toml_value(entry: &Parsed) -> toml::Value {
    let mut map = toml::map::Map::new();

    if let Some(ref name) = entry.name {
        map.insert("name".to_string(), toml::Value::String(name.clone()));
    }
    if let Some(ref dsc) = entry.dsc {
        map.insert("description".to_string(), toml::Value::String(dsc.clone()));
    }
    map.insert("url".to_string(), toml::Value::String(entry.url.clone()));
    map.insert("rev".to_string(), toml::Value::String(entry.rev.clone()));

    // Handle src as array or single value
    if entry.src.len() == 1 {
        map.insert("src".to_string(), toml::Value::String(entry.src[0].clone()));
    } else {
        let src_array: Vec<toml::Value> = entry
            .src
            .iter()
            .map(|s| toml::Value::String(s.clone()))
            .collect();
        map.insert("src".to_string(), toml::Value::Array(src_array));
    }

    map.insert("dst".to_string(), toml::Value::String(entry.dst.clone()));

    if let Some(ref mtd) = entry.mtd {
        let method_str = match mtd {
            Method::Shallow => "shallow",
            Method::ShallowNoSparse => "shallow_no_sparse",
            Method::Partial => "partial",
        };
        map.insert(
            "method".to_string(),
            toml::Value::String(method_str.to_string()),
        );
    }

    toml::Value::Table(map)
}

// ============================================================================
// INTERACTIVE PROMPT
// ============================================================================

/// Interactive prompt to create .gitwire.toml when no config exists
pub fn prompt_create_gitwire() -> Result<Option<Parsed>, Cause<ErrorType>> {
    let should_create = Confirm::new(
        "No .gitwire.toml found and no CLI arguments provided. Would you like to create one?",
    )
    .with_default(true)
    .prompt()
    .map_err(|e| cause!(PromptError, format!("Prompt error: {e}")))?;

    if !should_create {
        return Ok(None);
    }

    println!(
        "\n{}",
        "Let's create your .gitwire.toml configuration:"
            .cyan()
            .bold()
    );

    let url = Text::new("Repository URL:")
        .with_help_message("e.g., https://github.com/user/repo.git")
        .prompt()
        .map_err(|e| cause!(PromptError, format!("Prompt error: {e}")))?;

    let rev = Text::new("Git revision (branch/tag/commit):")
        .with_default("main")
        .prompt()
        .map_err(|e| cause!(PromptError, format!("Prompt error: {e}")))?;

    let src_input = Text::new("Source path(s):")
        .with_help_message("Single path or JSON array like [\"lib\", \"tools\"]")
        .with_default("src")
        .prompt()
        .map_err(|e| cause!(PromptError, format!("Prompt error: {e}")))?;

    // Try to parse as JSON array, otherwise treat as single path
    let src = if src_input.trim().starts_with('[') {
        serde_json::from_str::<Vec<String>>(&src_input).unwrap_or_else(|_| vec![src_input])
    } else {
        vec![src_input]
    };

    let dst = Text::new("Destination path:")
        .with_default("vendor")
        .prompt()
        .map_err(|e| cause!(PromptError, format!("Prompt error: {e}")))?;

    let name_input = Text::new("Entry name (optional, press Enter to skip):")
        .prompt()
        .map_err(|e| cause!(PromptError, format!("Prompt error: {e}")))?;

    let name = if name_input.is_empty() {
        None
    } else {
        Some(name_input)
    };

    let parsed = Parsed {
        name,
        dsc: None,
        url,
        rev,
        src,
        dst,
        mtd: None,
    };

    // Validate
    parsed
        .validate()
        .map_err(|e| cause!(DotGitWireFileParse, e))?;

    Ok(Some(parsed))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_wire_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(content.as_bytes())
            .expect("Failed to write to temp file");
        temp_file
    }

    #[test]
    fn test_parse_valid_toml_minimal() {
        let content = r#"[wire]
entries = [
    { name = "myrepo", url = "https://github.com/example/repo.git", rev = "main", src = "src", dst = "vendor/repo" }
]
"#;
        let temp_file = write_wire_file(content);

        let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
            .expect("Failed to parse valid TOML");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, Some("myrepo".to_string()));
        assert_eq!(parsed[0].url, "https://github.com/example/repo.git");
        assert_eq!(parsed[0].rev, "main");
        assert_eq!(parsed[0].src, vec!["src".to_string()]);
        assert_eq!(parsed[0].dst, "vendor/repo");
        assert!(parsed[0].dsc.is_none());
        assert!(parsed[0].mtd.is_none());
    }

    #[test]
    fn test_parse_valid_toml_with_src_array() {
        let content = r#"[wire]
entries = [
    { name = "myrepo", url = "https://github.com/example/repo.git", rev = "main", src = ["lib", "tools", "src"], dst = "vendor/repo" }
]
"#;
        let temp_file = write_wire_file(content);

        let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
            .expect("Failed to parse valid TOML");
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].src,
            vec!["lib".to_string(), "tools".to_string(), "src".to_string()]
        );
    }

    #[test]
    fn test_parse_valid_toml_full() {
        let content = r#"[wire]
entries = [
    { name = "my-lib", description = "My awesome library", url = "https://github.com/user/repo.git", rev = "v1.0.0", src = "lib/src", dst = "third_party/my-lib", method = "shallow" }
]
"#;
        let temp_file = write_wire_file(content);

        let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
            .expect("Failed to parse valid TOML");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, Some("my-lib".to_string()));
        assert_eq!(parsed[0].dsc, Some("My awesome library".to_string()));
        assert_eq!(parsed[0].url, "https://github.com/user/repo.git");
        assert_eq!(parsed[0].rev, "v1.0.0");
        assert_eq!(parsed[0].src, vec!["lib/src".to_string()]);
        assert_eq!(parsed[0].dst, "third_party/my-lib");
        assert_eq!(parsed[0].mtd, Some(Method::Shallow));
    }

    #[test]
    fn test_parse_multiple_entries() {
        let content = r#"[wire]
entries = [
    { name = "lib1", url = "https://github.com/example/lib1.git", rev = "main", src = "src", dst = "vendor/lib1" },
    { name = "lib2", url = "https://github.com/example/lib2.git", rev = "develop", src = "include", dst = "vendor/lib2", method = "partial" }
]
"#;
        let temp_file = write_wire_file(content);

        let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
            .expect("Failed to parse valid TOML");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, Some("lib1".to_string()));
        assert_eq!(parsed[1].name, Some("lib2".to_string()));
        assert_eq!(parsed[1].mtd, Some(Method::Partial));
    }

    #[test]
    fn test_parse_all_methods() {
        for method in ["shallow", "shallow_no_sparse", "partial"] {
            let content = format!(
                "[wire]\nentries = [\n    {{ name = \"myrepo\", url = \"https://github.com/example/repo.git\", rev = \"main\", src = \"src\", dst = \"vendor/repo\", method = \"{method}\" }}\n]\n"
            );
            let temp_file = write_wire_file(&content);

            let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
                .expect("Failed to parse valid TOML");
            let expected = match method {
                "shallow" => Method::Shallow,
                "shallow_no_sparse" => Method::ShallowNoSparse,
                "partial" => Method::Partial,
                _ => panic!("unexpected method"),
            };
            assert_eq!(
                parsed[0].mtd,
                Some(expected),
                "Method {method} should parse correctly"
            );
        }
    }

    #[test]
    fn test_parse_missing_required_fields() {
        let content = r#"[wire]
entries = [
    { name = "test", description = "test" }
]
"#;
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_method() {
        let content = r#"[wire]
entries = [
    { name = "myrepo", url = "https://github.com/example/repo.git", rev = "main", src = "src", dst = "vendor/repo", method = "unknown_method" }
]
"#;
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duplicate_names() {
        let content = r#"[wire]
entries = [
    { name = "lib", url = "https://github.com/example/lib1.git", rev = "main", src = "src", dst = "vendor/lib1" },
    { name = "lib", url = "https://github.com/example/lib2.git", rev = "main", src = "src", dst = "vendor/lib2" }
]
"#;
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_path_traversal_in_src() {
        let content = r#"[wire]
entries = [
    { name = "myrepo", url = "https://github.com/example/repo.git", rev = "main", src = "../etc", dst = "vendor/repo" }
]
"#;
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_path_traversal_in_dst() {
        let content = r#"[wire]
entries = [
    { name = "myrepo", url = "https://github.com/example/repo.git", rev = "main", src = "src", dst = "../../sensitive" }
]
"#;
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_git_in_path() {
        let content = r#"[wire]
entries = [
    { name = "myrepo", url = "https://github.com/example/repo.git", rev = "main", src = "src/.git", dst = "vendor/repo" }
]
"#;
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = "this is not valid toml\n";
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_wire_section() {
        let content = "[other]\nsomething = true\n";
        let temp_file = write_wire_file(content);

        let result = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_entries_array() {
        let content = r"[wire]
entries = []
";
        let temp_file = write_wire_file(content);

        let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
            .expect("Failed to parse valid TOML");
        assert_eq!(parsed.len(), 0);
    }

    #[test]
    fn test_parse_entry_without_name() {
        let content = r#"[wire]
entries = [
    { url = "https://github.com/example/repo.git", rev = "main", src = "src", dst = "vendor/repo" }
]
"#;
        let temp_file = write_wire_file(content);

        let parsed = parse_dotgitwire_file(temp_file.path().to_str().expect("Invalid path"))
            .expect("Failed to parse valid TOML");
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].name.is_none());
    }

    #[test]
    fn test_format_entry_single_src() {
        let entry = Parsed {
            name: Some("test".to_string()),
            dsc: None,
            url: "https://github.com/test/repo.git".to_string(),
            rev: "main".to_string(),
            src: vec!["src".to_string()],
            dst: "vendor".to_string(),
            mtd: None,
        };

        let formatted = format_entry(&entry);
        assert!(formatted.contains("src = \"src\""));
        assert!(!formatted.contains("src = ["));
    }

    #[test]
    fn test_format_entry_multiple_src() {
        let entry = Parsed {
            name: Some("test".to_string()),
            dsc: None,
            url: "https://github.com/test/repo.git".to_string(),
            rev: "main".to_string(),
            src: vec!["lib".to_string(), "tools".to_string()],
            dst: "vendor".to_string(),
            mtd: None,
        };

        let formatted = format_entry(&entry);
        assert!(formatted.contains("src = [\"lib\", \"tools\"]"));
    }

    #[test]
    fn test_validate_parsed_valid() {
        let parsed = Parsed {
            name: Some("test".to_string()),
            dsc: None,
            url: "https://github.com/test/repo.git".to_string(),
            rev: "main".to_string(),
            src: vec!["src".to_string()],
            dst: "vendor".to_string(),
            mtd: None,
        };

        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn test_validate_parsed_empty_url() {
        let parsed = Parsed {
            name: None,
            dsc: None,
            url: String::new(),
            rev: "main".to_string(),
            src: vec!["src".to_string()],
            dst: "vendor".to_string(),
            mtd: None,
        };

        assert!(parsed.validate().is_err());
    }

    #[test]
    fn test_validate_parsed_empty_src() {
        let parsed = Parsed {
            name: None,
            dsc: None,
            url: "https://github.com/test/repo.git".to_string(),
            rev: "main".to_string(),
            src: vec![],
            dst: "vendor".to_string(),
            mtd: None,
        };

        assert!(parsed.validate().is_err());
    }

    #[test]
    fn test_validate_parsed_invalid_src_path() {
        let parsed = Parsed {
            name: None,
            dsc: None,
            url: "https://github.com/test/repo.git".to_string(),
            rev: "main".to_string(),
            src: vec!["../escape".to_string()],
            dst: "vendor".to_string(),
            mtd: None,
        };

        assert!(parsed.validate().is_err());
    }

    #[test]
    fn test_validate_unique_names_no_duplicates() {
        let parsed = vec![
            Parsed {
                name: Some("lib1".to_string()),
                dsc: None,
                url: "https://github.com/example/lib1.git".to_string(),
                rev: "main".to_string(),
                src: vec!["src".to_string()],
                dst: "vendor/lib1".to_string(),
                mtd: None,
            },
            Parsed {
                name: Some("lib2".to_string()),
                dsc: None,
                url: "https://github.com/example/lib2.git".to_string(),
                rev: "main".to_string(),
                src: vec!["src".to_string()],
                dst: "vendor/lib2".to_string(),
                mtd: None,
            },
        ];
        assert!(validate_unique_names(&parsed).is_ok());
    }

    #[test]
    fn test_validate_unique_names_with_duplicates() {
        let parsed = vec![
            Parsed {
                name: Some("lib".to_string()),
                dsc: None,
                url: "https://github.com/example/lib1.git".to_string(),
                rev: "main".to_string(),
                src: vec!["src".to_string()],
                dst: "vendor/lib1".to_string(),
                mtd: None,
            },
            Parsed {
                name: Some("lib".to_string()),
                dsc: None,
                url: "https://github.com/example/lib2.git".to_string(),
                rev: "main".to_string(),
                src: vec!["src".to_string()],
                dst: "vendor/lib2".to_string(),
                mtd: None,
            },
        ];
        assert!(validate_unique_names(&parsed).is_err());
    }

    #[test]
    fn test_validate_unique_names_with_none_names() {
        let parsed = vec![
            Parsed {
                name: None,
                dsc: None,
                url: "https://github.com/example/lib1.git".to_string(),
                rev: "main".to_string(),
                src: vec!["src".to_string()],
                dst: "vendor/lib1".to_string(),
                mtd: None,
            },
            Parsed {
                name: None,
                dsc: None,
                url: "https://github.com/example/lib2.git".to_string(),
                rev: "main".to_string(),
                src: vec!["src".to_string()],
                dst: "vendor/lib2".to_string(),
                mtd: None,
            },
        ];
        assert!(validate_unique_names(&parsed).is_ok());
    }
}
