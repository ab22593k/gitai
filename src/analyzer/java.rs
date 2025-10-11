use super::{FileAnalyzer, ProjectMetadata};
use crate::core::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

// Regex for extracting Maven version from pom.xml
static MAVEN_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"<version>(.+?)</version>").expect("Should compile: MAVEN_VERSION_RE")
});
// Regex for extracting Maven dependencies from pom.xml
static MAVEN_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"<dependency>\s*<groupId>(.+?)</groupId>\s*<artifactId>(.+?)</artifactId>")
        .expect("Should compile: MAVEN_DEPENDENCY_RE")
});
// Regex for extracting Gradle version from build.gradle
static GRADLE_VERSION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"version\s*=\s*['"](.*?)['"]"#).expect("Should compile: GRADLE_VERSION_RE")
});
// Regex for extracting Gradle dependencies from build.gradle
static GRADLE_DEPENDENCY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"implementation\s+['"](.+?):(.+?):"#)
        .expect("Should compile: GRADLE_DEPENDENCY_RE")
});
// Regex for extracting modified Java classes
static JAVA_CLASS_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(public\s+|private\s+)?(class|interface|enum)\s+(\w+)")
        .expect("Should compile: JAVA_CLASS_RE")
});
// Regex for extracting modified Java methods
static JAVA_METHOD_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*(public|protected|private)?\s*\w+\s+(\w+)\s*\([^\)]*\)")
        .expect("Should compile: JAVA_METHOD_RE")
});
// Regex for checking Java import changes
static JAVA_IMPORT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?m)^[+-]\s*import\s+").expect("Should compile: JAVA_IMPORT_RE")
});

pub struct JavaAnalyzer;

impl FileAnalyzer for JavaAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if let Some(methods) = extract_modified_methods(&staged_file.diff) {
            analysis.push(format!("Modified methods: {}", methods.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Java source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some("Java".to_string()),
            ..Default::default()
        };

        if file == "pom.xml" {
            Self::extract_maven_metadata(content, &mut metadata);
        } else if file == "build.gradle" {
            Self::extract_gradle_metadata(content, &mut metadata);
        } else {
            Self::extract_java_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl JavaAnalyzer {
    fn extract_maven_metadata(content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("Maven".to_string());

        if let Some(cap) = MAVEN_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        for cap in MAVEN_DEPENDENCY_RE.captures_iter(content) {
            metadata
                .dependencies
                .push(format!("{}:{}", &cap[1], &cap[2]));
        }
    }

    fn extract_gradle_metadata(content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("Gradle".to_string());

        if let Some(cap) = GRADLE_VERSION_RE.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        for cap in GRADLE_DEPENDENCY_RE.captures_iter(content) {
            metadata
                .dependencies
                .push(format!("{}:{}", &cap[1], &cap[2]));
        }
    }

    fn extract_java_file_metadata(content: &str, metadata: &mut ProjectMetadata) {
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
    let classes: HashSet<String> = JAVA_CLASS_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(3).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes.into_iter().collect())
    }
}

fn extract_modified_methods(diff: &str) -> Option<Vec<String>> {
    let methods: HashSet<String> = JAVA_METHOD_RE
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if methods.is_empty() {
        None
    } else {
        Some(methods.into_iter().collect())
    }
}

fn has_import_changes(diff: &str) -> bool {
    JAVA_IMPORT_RE.is_match(diff)
}
