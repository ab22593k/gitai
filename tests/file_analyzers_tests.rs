use gitai::{
    analyzer::get_analyzer,
    core::context::{ChangeType, StagedFile},
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::MockDataBuilder;

#[test]
fn test_rust_analyzer() {
    let analyzer = get_analyzer("test.rs");
    let change = MockDataBuilder::staged_file_for_analysis(
        "test.rs",
        ChangeType::Modified,
        r#"
+pub fn new_function() {
+    println!("Hello, world!");
+}
-struct OldStruct {
-    field: i32,
-}
+struct NewStruct {
+    field: String,
+}
        "#,
    );

    let analysis = analyzer.analyze("test.rs", &change);
    println!("Rust Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Modified functions: new_function".to_string()));
    assert!(analysis.contains(&"Modified structs: OldStruct, NewStruct".to_string()));
}

#[test]
fn test_javascript_analyzer() {
    let analyzer = get_analyzer("test.js");
    let change = MockDataBuilder::staged_file_for_analysis(
        "test.js",
        ChangeType::Modified,
        r#"
+function newFunction() {
+    console.log("Hello, world!");
+}
-class OldClass {
-    constructor() {}
-}
+class NewClass extends React.Component {
+    render() {
+        return <div>Hello</div>;
+    }
+}
+const FunctionalComponent = () => {
+    return <div>Functional</div>;
+}
+import { useState } from 'react';
        "#,
    );

    let analysis = analyzer.analyze("test.js", &change);
    println!("JavaScript Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Modified functions: newFunction, FunctionalComponent".to_string()));
    assert!(analysis.contains(&"Modified classes: OldClass, NewClass".to_string()));
    assert!(analysis.contains(&"Import statements have been modified".to_string()));

    let react_components = analysis
        .iter()
        .find(|&s| s.starts_with("Modified React components:"))
        .expect("No React components analysis found");
    assert!(react_components.contains("NewClass"));
    assert!(react_components.contains("FunctionalComponent"));
}

#[test]
fn test_python_analyzer() {
    let analyzer = get_analyzer("test.py");
    let change = MockDataBuilder::staged_file_for_analysis(
        "test.py",
        ChangeType::Modified,
        r#"
+def new_function():
+    print("Hello, world!")
-class OldClass:
-    pass
+class NewClass:
+    def __init__(self):
+        pass
+@decorator
+def decorated_function():
+    pass
+from module import something
        "#,
    );

    let analysis = analyzer.analyze("test.py", &change);
    println!("Python Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Modified functions: new_function, decorated_function".to_string()));
    assert!(analysis.contains(&"Modified classes: OldClass, NewClass".to_string()));
    assert!(analysis.contains(&"Import statements have been modified".to_string()));
    assert!(analysis.contains(&"Modified decorators: decorator".to_string()));
}

#[test]
fn test_yaml_analyzer() {
    let analyzer = get_analyzer("test.yaml");
    let change = MockDataBuilder::staged_file_for_analysis(
        "test.yaml",
        ChangeType::Modified,
        r"
+new_key: value
-old_key: value
 list:
+  - new item
-  - old item
 nested:
+  inner_key: value
        ",
    );

    let analysis = analyzer.analyze("test.yaml", &change);
    println!("YAML Test Debug: Analysis results: {analysis:?}");

    let top_level_keys_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified top-level keys:"))
        .expect("No top-level keys analysis found");
    assert!(top_level_keys_analysis.contains("new_key"));
    assert!(top_level_keys_analysis.contains("old_key"));
    assert!(!top_level_keys_analysis.contains("inner_key"));

    assert!(analysis.contains(&"List structures have been modified".to_string()));
    assert!(analysis.contains(&"Nested structures have been modified".to_string()));
}

#[test]
fn test_json_analyzer() {
    let analyzer = get_analyzer("test.json");
    let change = StagedFile {
        path: "test.json".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+  "new_key": "value",
-  "old_key": "value",
   "array": [
+    "new item",
-    "old item"
   ],
   "nested": {
+    "inner_key": "value"
   }
        "#
        .to_string(),
        analysis: Vec::new(),
        content_excluded: false,
        content: None,
    };

    let analysis = analyzer.analyze("test.json", &change);
    println!("JSON Test Debug: Analysis results: {analysis:?}");

    assert!(
        analysis
            .iter()
            .any(|s| s.starts_with("Modified top-level keys:"))
    );
    assert!(analysis.contains(&"Array structures have been modified".to_string()));
    assert!(analysis.contains(&"Nested objects have been modified".to_string()));

    let top_level_keys_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified top-level keys:"))
        .expect("No top-level keys analysis found");

    assert!(top_level_keys_analysis.contains("new_key"));
    assert!(top_level_keys_analysis.contains("old_key"));
    assert!(!top_level_keys_analysis.contains("inner_key"));
}

#[test]
fn test_markdown_analyzer() {
    let analyzer = get_analyzer("test.md");
    let change = StagedFile {
        path: "test.md".to_string(),
        change_type: ChangeType::Modified,
        diff: r"
+# New Header
-## Old Header
+ - New list item
- * Old list item
+```
+New code block
+```
+[New link](https://example.com)
-[Old link](https://old-example.com)
        "
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("test.md", &change);
    println!("Markdown Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Modified headers: New Header, Old Header".to_string()));
    assert!(analysis.contains(&"List structures have been modified".to_string()));
    assert!(analysis.contains(&"Code blocks have been modified".to_string()));
    assert!(analysis.contains(&"Links have been modified".to_string()));
}

#[test]
fn test_default_analyzer() {
    let analyzer = get_analyzer("unknown.xyz");
    let change = StagedFile {
        path: "unknown.xyz".to_string(),
        change_type: ChangeType::Modified,
        diff: "Some changes".to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("unknown.xyz", &change);
    println!("Default Analyzer Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Unable to analyze non-text or binary file".to_string()));
}

#[test]
fn test_java_analyzer() {
    let analyzer = get_analyzer("test.java");
    let change = StagedFile {
        path: "test.java".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+public class NewClass {
+    public void newMethod() {
+        System.out.println("Hello, World!");
+    }
+}
-private class OldClass {
-    private void oldMethod() {
-        // Do nothing
-    }
-}
+import java.util.List;
-import java.util.ArrayList;
        "#
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("test.java", &change);
    println!("Java analysis results: {analysis:?}");

    // Check for modified classes
    let class_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified classes:"))
        .expect("No class analysis found");
    assert!(class_analysis.contains("NewClass"), "NewClass not detected");
    assert!(class_analysis.contains("OldClass"), "OldClass not detected");

    // Check for modified methods
    let method_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified methods:"))
        .expect("No method analysis found");
    assert!(
        method_analysis.contains("newMethod"),
        "newMethod not detected"
    );
    assert!(
        method_analysis.contains("oldMethod"),
        "oldMethod not detected"
    );

    // Check for import changes
    assert!(
        analysis.contains(&"Import statements have been modified".to_string()),
        "Failed to detect import changes. Analysis: {analysis:?}"
    );
}

#[test]
fn test_kotlin_analyzer() {
    let analyzer = get_analyzer("test.kt");
    let change = StagedFile {
        path: "test.kt".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+class NewClass {
+    fun newFunction() {
+        println("Hello, Kotlin!")
+    }
+}
-object OldObject {
-    fun oldFunction() {
-        // Do nothing
-    }
-}
+import kotlin.collections.List
-import kotlin.collections.ArrayList
        "#
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("test.kt", &change);
    println!("Kotlin analysis results: {analysis:?}");

    // Helper function to check if any string in the analysis contains all expected substrings
    #[allow(clippy::items_after_statements)]
    fn contains_all_substrings(analysis: &[String], substrings: &[&str]) -> bool {
        analysis
            .iter()
            .any(|s| substrings.iter().all(|&sub| s.contains(sub)))
    }

    // Check for the presence of the entire expected strings
    assert!(contains_all_substrings(
        &analysis,
        &["NewClass", "OldObject"]
    ));
    assert!(contains_all_substrings(
        &analysis,
        &["newFunction", "oldFunction"]
    ));
    assert!(analysis.contains(&"Import statements have been modified".to_string()));
}

#[test]
fn test_gradle_analyzer() {
    let analyzer = get_analyzer("build.gradle");
    let change = StagedFile {
        path: "build.gradle".to_string(),
        change_type: ChangeType::Modified,
        diff: r"
+    implementation 'com.example:new-library:1.0.0'
-    implementation 'com.example:old-library:0.9.0'
+plugins {
+    id 'com.android.application'
+}
-apply plugin: 'java'
+task newTask {
+    doLast {
+        println 'Executing new task'
+    }
+}
        "
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("build.gradle", &change);
    assert!(analysis.contains(&"Dependencies have been modified".to_string()));
    assert!(analysis.contains(&"Plugins have been modified".to_string()));
    assert!(analysis.contains(&"Tasks have been modified".to_string()));
}

#[test]
fn test_c_analyzer() {
    let analyzer = get_analyzer("test.c");
    let change = StagedFile {
        path: "test.c".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+void new_function() {
+    printf("Hello, world!");
+}
-struct OldStruct {
-    int field;
-}
+struct NewStruct {
+    char field;
+}
+#include <stdio.h>
-#include <stdlib.h>
        "#
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("test.c", &change);
    println!("C Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Modified functions: new_function".to_string()));
    assert!(
        analysis
            .iter()
            .any(|s| s.contains("OldStruct") && s.contains("NewStruct"))
    );
    assert!(analysis.contains(&"Include statements have been modified".to_string()));
}

#[test]
fn test_c_analyzer_metadata() {
    let analyzer = get_analyzer("Makefile");
    let content = r"
VERSION = 1.0
LIBS += -lm
    ";

    let metadata = analyzer.extract_metadata("Makefile", content);
    assert_eq!(metadata.language, Some("C".to_string()));
    assert_eq!(metadata.build_system, Some("Makefile".to_string()));
    assert_eq!(metadata.version, Some("1.0".to_string()));
    assert_eq!(metadata.dependencies, vec!["-lm".to_string()]);
}

#[test]
fn test_cpp_analyzer() {
    let analyzer = get_analyzer("test.cpp");
    let change = StagedFile {
        path: "test.cpp".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+void newFunction() {
+    std::cout << "Hello, world!" << std::endl;
+}
-class OldClass {
-    OldClass() {}
-}
+class NewClass {
+    NewClass() {}
+}
+#include <iostream>
-#include <vector>
        "#
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("test.cpp", &change);
    println!("C++ Test Debug: Analysis results: {analysis:?}");
    assert!(analysis.contains(&"Modified functions: newFunction".to_string()));
    assert!(
        analysis
            .iter()
            .any(|s| s.contains("OldClass") && s.contains("NewClass"))
    );
    assert!(analysis.contains(&"Include statements have been modified".to_string()));
}

#[test]
fn test_cpp_analyzer_metadata() {
    let analyzer = get_analyzer("CMakeLists.txt");
    let content = r"
project(MyProject VERSION 1.0)
find_package(Boost REQUIRED)
    ";

    let metadata = analyzer.extract_metadata("CMakeLists.txt", content);
    assert_eq!(metadata.language, Some("C++".to_string()));
    assert_eq!(metadata.build_system, Some("CMake".to_string()));
    assert_eq!(metadata.version, Some("1.0".to_string()));
    assert!(metadata.dependencies.contains(&"Boost".to_string()));
}

#[test]
fn test_toml_analyzer() {
    let analyzer = get_analyzer("Cargo.toml");
    let change = StagedFile {
        path: "Cargo.toml".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+[dependencies]
+serde = "1.0.104"
-serde = "1.0.100"
+[dev-dependencies]
+tokio = { version = "1.0", features = ["full"] }
-version = "0.1.0"
+version = "0.1.0"
+[package]
+authors = ["User <user@example.com>"]
        "#
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("Cargo.toml", &change);
    println!("TOML Test Debug: Analysis results: {analysis:?}");

    // Verify basic TOML analysis
    assert!(analysis.contains(&"Dependencies have been updated".to_string()));
    assert!(analysis.contains(&"Version updated to 0.1.0".to_string()));

    // Verify section detection
    let sections_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified sections:"))
        .expect("No sections analysis found");
    assert!(sections_analysis.contains("dependencies"));
    assert!(sections_analysis.contains("dev-dependencies"));
    assert!(sections_analysis.contains("package"));

    // Verify Rust-specific detection
    assert!(analysis.contains(&"Rust project configuration changed".to_string()));

    // Test metadata extraction
    let content = r#"
[package]
name = "gitai"
version = "0.2.0"
authors = ["Author <author@example.com>"]

[dependencies]
serde = "1.0.104"
tokio = "1.0"
"#;

    let metadata = analyzer.extract_metadata("Cargo.toml", content);
    assert_eq!(metadata.language, Some("Rust".to_string()));
    assert_eq!(metadata.version, Some("0.2.0".to_string()));
    assert!(metadata.dependencies.contains(&"serde".to_string()));
    assert!(metadata.dependencies.contains(&"tokio".to_string()));
}

#[test]
fn test_generic_text_analyzer() {
    // Test basic text file
    let analyzer = get_analyzer("config.txt");
    let change = StagedFile {
        path: "config.txt".to_string(),
        change_type: ChangeType::Modified,
        diff: r"
+New line added
-Old line removed
 Unchanged line
        "
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let analysis = analyzer.analyze("config.txt", &change);
    println!("Generic Text Test Debug: Analysis results: {analysis:?}");

    // Verify basic line counting
    let line_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Changes:"))
        .expect("No line count analysis found");
    assert!(line_analysis.contains("1 line(s) added"));
    assert!(line_analysis.contains("1 line(s) removed"));

    // Test config file
    let config_analyzer = get_analyzer("app.config");
    let config_change = StagedFile {
        path: "app.config".to_string(),
        change_type: ChangeType::Modified,
        diff: r"
+server.host=localhost
-server.port=8080
+server.port=9000
 log.level=info
        "
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let config_analysis = config_analyzer.analyze("app.config", &config_change);
    println!("Config Test Debug: Analysis results: {config_analysis:?}");

    let keys_analysis = config_analysis
        .iter()
        .find(|&s| s.starts_with("Modified configuration keys:"))
        .expect("No config keys analysis found");
    assert!(keys_analysis.contains("server.host"));
    assert!(keys_analysis.contains("server.port"));

    // Test XML file
    let xml_analyzer = get_analyzer("web.xml");
    let xml_change = StagedFile {
        path: "web.xml".to_string(),
        change_type: ChangeType::Modified,
        diff: r"
+  <servlet>
+    <servlet-name>MyServlet</servlet-name>
+  </servlet>
-  <filter>
-    <filter-name>OldFilter</filter-name>
-  </filter>
        "
        .to_string(),
        analysis: Vec::new(),
        content: None,
        content_excluded: false,
    };

    let xml_analysis = xml_analyzer.analyze("web.xml", &xml_change);
    println!("XML Test Debug: Analysis results: {xml_analysis:?}");

    let tags_analysis = xml_analysis
        .iter()
        .find(|&s| s.starts_with("Modified tags:"))
        .expect("No tags analysis found");

    // Only check for the main tags, not the nested ones
    assert!(tags_analysis.contains("servlet"));
    assert!(tags_analysis.contains("filter"));
}


