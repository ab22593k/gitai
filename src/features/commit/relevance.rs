use crate::core::context::{ChangeType, CommitContext};
use std::collections::HashMap;
use std::path::Path;

pub struct RelevanceScorer {
    scorers: Vec<Box<dyn Scorer>>,
}

trait Scorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32>;
}

struct FileTypeScorer;
impl Scorer for FileTypeScorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut scores = HashMap::new();
        for file in &context.staged_files {
            let score = match file.path.split('.').next_back() {
                Some("rs") => 1.0,
                Some("js" | "ts") => 0.9,
                Some("py") => 0.8,
                _ => 0.5,
            };
            scores.insert(file.path.clone(), score);
        }
        scores
    }
}

struct ChangeTypeScorer;
impl Scorer for ChangeTypeScorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut scores = HashMap::new();
        for file in &context.staged_files {
            let score = match file.change_type {
                ChangeType::Added => 0.9,
                ChangeType::Modified => 1.0,
                ChangeType::Deleted => 0.7,
            };
            scores.insert(file.path.clone(), score);
        }
        scores
    }
}

struct PathScorer;
impl Scorer for PathScorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut scores = HashMap::new();
        for file in &context.staged_files {
            let path = file.path.to_lowercase();
            let mut score = 0.0;

            if path.starts_with("src/") || path.contains("/src/") {
                score += 0.5;
            } else if path.starts_with("tests/")
                || path.contains("/tests/")
                || path.ends_with("test.rs")
                || path.ends_with("spec.ts")
                || path.ends_with("test.ts")
            {
                score -= 0.2; // Tests are important but secondary to implementation
            } else if path.starts_with("docs/")
                || Path::new(&path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                score -= 0.3; // Documentation is supporting context
            } else if path.contains("lock")
                || Path::new(&path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("map"))
            {
                score -= 0.8; // generated files are low relevance
            } else if path.contains("config")
                || Path::new(&path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
                || Path::new(&path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                score += 0.2; // Config changes are often significant
            }

            scores.insert(file.path.clone(), score);
        }
        scores
    }
}

impl RelevanceScorer {
    pub fn new() -> Self {
        Self {
            scorers: vec![
                Box::new(FileTypeScorer),
                Box::new(ChangeTypeScorer),
                Box::new(PathScorer),
            ],
        }
    }

    pub fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut final_scores = HashMap::new();
        for scorer in &self.scorers {
            let scores = scorer.score(context);
            for (key, value) in scores {
                *final_scores.entry(key).or_insert(0.0) += value;
            }
        }
        final_scores
    }
}
