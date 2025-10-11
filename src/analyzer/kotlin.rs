use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

// Regex for extracting Gradle version from build.gradle.kts
static GRADLE_KTS_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"version\s*=\s*['"](.*?)['"]"#).expect("Should compile: GRADLE_KTS_VERSION_RE")
});
// Regex for extracting Gradle dependencies from build.gradle.kts
static GRADLE_KTS_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"implementation\s*\(\s*["'](.+?):(.+?):(.+?)["']\)"#)
        .expect("Should compile: GRADLE_KTS_DEPENDENCY_RE")
});
// Regex for extracting modified Kotlin classes/interfaces/objects
static KOTLIN_CLASS_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(class|interface|object)\s+(\w+)")
        .expect("Should compile: KOTLIN_CLASS_RE")
});
// Regex for extracting modified Kotlin functions
static KOTLIN_FUNCTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(fun)\s+(\w+)").expect("Should compile: KOTLIN_FUNCTION_RE")
});
// Regex for checking Kotlin import changes
static KOTLIN_IMPORT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*import\s+").expect("Should compile: KOTLIN_IMPORT_RE")
});

pub struct KotlinAnalyzer;

impl FileAnalyzer for KotlinAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Kotlin source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some("Kotlin".to_string()),
            ..Default::default()
        };

        if file == "build.gradle.kts" {
            Self::extract_gradle_metadata(content, &mut metadata);
        } else {
            Self::extract_kotlin_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl KotlinAnalyzer {
    fn extract_gradle_metadata(content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("Gradle".to_string());

        if let Some(cap) = GRADLE_KTS_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        for cap in GRADLE_KTS_DEPENDENCY_RE.captures_iter(content) {
            metadata
                .dependencies
                .push(format!("{}:{}:{}", &cap[1], &cap[2], &cap[3]));
        }
    }

    fn extract_kotlin_file_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("import org.springframework") {
            metadata.framework = Some("Spring".to_string());
        } else if content.contains("import javax.ws.rs") {
            metadata.framework = Some("JAX-RS".to_string());
        }

        if content.contains("import org.junit.") {
            metadata.test_framework = Some("JUnit".to_string());
        } else if content.contains("import org.testng.") {
            metadata.test_framework = Some("TestNG".to_string());
        }
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let classes: HashSet<String> = KOTLIN_CLASS_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes.into_iter().collect())
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let functions: HashSet<String> = KOTLIN_FUNCTION_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions.into_iter().collect())
    }
}

fn has_import_changes(diff: &str) -> bool {
    KOTLIN_IMPORT_RE.is_match(diff)
}
