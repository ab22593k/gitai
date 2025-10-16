use serde::Serialize;
use std::fmt;

use crate::core::token_optimizer::TokenOptimizer;

#[derive(Serialize, Debug, Clone)]
pub struct CommitContext {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub project_metadata: ProjectMetadata,
    pub user_name: String,
    pub user_email: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct RecentCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct StagedFile {
    pub path: String,
    pub change_type: ChangeType,
    pub diff: String,
    pub analysis: Vec<String>,
    pub content: Option<String>,
    pub content_excluded: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "Added"),
            Self::Modified => write!(f, "Modified"),
            Self::Deleted => write!(f, "Deleted"),
        }
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct ProjectMetadata {
    pub language: Option<String>,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub version: Option<String>,
    pub build_system: Option<String>,
    pub test_framework: Option<String>,
    pub plugins: Vec<String>,
}

impl ProjectMetadata {
    pub fn merge(&mut self, new: ProjectMetadata) {
        if let Some(new_lang) = new.language {
            match &mut self.language {
                Some(lang) if !lang.contains(&new_lang) => {
                    lang.push_str(", ");
                    lang.push_str(&new_lang);
                }
                None => self.language = Some(new_lang),
                _ => {}
            }
        }
        self.dependencies.extend(new.dependencies.clone());
        self.framework = self.framework.take().or(new.framework);
        self.version = self.version.take().or(new.version);
        self.build_system = self.build_system.take().or(new.build_system);
        self.test_framework = self.test_framework.take().or(new.test_framework);
        self.plugins.extend(new.plugins);
        self.dependencies.sort();
        self.dependencies.dedup();
    }
}

/// Fixed-size buffer with const generic size parameter
#[derive(Debug, Clone)]
pub struct FixedSizeBuffer<T, const N: usize> {
    data: [T; N],
    size: usize, // Current number of elements in the buffer
}

impl<T: Clone + Default, const N: usize> FixedSizeBuffer<T, N> {
    /// Create a new buffer with all elements initialized to default values
    pub fn new() -> Self {
        Self {
            data: [(); N].map(|()| T::default()),
            size: 0,
        }
    }

    /// Add an element to the buffer
    /// Returns true if the element was added, false if the buffer is full
    pub fn push(&mut self, item: T) -> bool {
        if self.size < N {
            self.data[self.size] = item;
            self.size += 1;
            true
        } else {
            false // Buffer is full
        }
    }

    /// Get an element by index
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.size {
            Some(&self.data[index])
        } else {
            None
        }
    }

    /// Get the number of elements currently in the buffer
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Check if the buffer is full
    pub fn is_full(&self) -> bool {
        self.size == N
    }

    /// Get the maximum capacity of the buffer
    pub fn capacity(&self) -> usize {
        N
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.size = 0;
    }

    /// Iterate over the elements in the buffer
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter().take(self.size)
    }
}

impl<T: Clone + Default, const N: usize> Default for FixedSizeBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitContext {
    pub fn new(
        branch: String,
        recent_commits: Vec<RecentCommit>,
        staged_files: Vec<StagedFile>,
        project_metadata: ProjectMetadata,
        user_name: String,
        user_email: String,
    ) -> Self {
        Self {
            branch,
            recent_commits,
            staged_files,
            project_metadata,
            user_name,
            user_email,
        }
    }
    pub fn optimize(&mut self, max_tokens: usize) {
        let optimizer = TokenOptimizer::new(max_tokens).expect(
            "Failed to initialize token optimizer. Ensure the tokenizer data is available.",
        );

        let _ = optimizer.optimize_context(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_size_buffer() {
        // Create a buffer of size 3
        let mut buffer: FixedSizeBuffer<i32, 3> = FixedSizeBuffer::new();

        // Initially empty
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.capacity(), 3);

        // Add elements
        assert_eq!(buffer.push(1), true); // Should succeed
        assert_eq!(buffer.push(2), true); // Should succeed
        assert_eq!(buffer.push(3), true); // Should succeed
        assert_eq!(buffer.push(4), false); // Should fail - buffer is full

        // Check length and capacity
        assert_eq!(buffer.len(), 3);
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());

        // Check elements
        assert_eq!(buffer.get(0), Some(&1));
        assert_eq!(buffer.get(1), Some(&2));
        assert_eq!(buffer.get(2), Some(&3));
        assert_eq!(buffer.get(3), None); // Out of bounds

        // Test iteration
        let collected: Vec<&i32> = buffer.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3]);

        // Clear the buffer
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.get(0), None);
    }

    #[test]
    fn test_const_generic_different_sizes() {
        let buffer_5: FixedSizeBuffer<u8, 5> = FixedSizeBuffer::new();
        let buffer_10: FixedSizeBuffer<u8, 10> = FixedSizeBuffer::new();

        assert_eq!(buffer_5.capacity(), 5);
        assert_eq!(buffer_10.capacity(), 10);
    }
}
