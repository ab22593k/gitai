use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

#[derive(Serialize, Debug, Clone)]
pub struct CommitContext {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub user_name: String,
    pub user_email: String,
    pub author_history: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RecentCommit {
    pub hash: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct StagedFile {
    pub path: String,
    pub change_type: ChangeType,
    pub diff: String,
    pub content: Option<String>,
    pub content_excluded: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed { from: String, similarity: u32 },
    Copied { from: String, similarity: u32 },
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "Added"),
            Self::Modified => write!(f, "Modified"),
            Self::Deleted => write!(f, "Deleted"),
            Self::Renamed { from, similarity } => {
                write!(f, "Renamed from '{from}' ({similarity}% similar)")
            }
            Self::Copied { from, similarity } => {
                write!(f, "Copied from '{from}' ({similarity}% similar)")
            }
        }
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
        user_name: String,
        user_email: String,
        author_history: Vec<String>,
    ) -> Self {
        Self {
            branch,
            recent_commits,
            staged_files,
            user_name,
            user_email,
            author_history,
        }
    }

    /// Detect common commit message conventions from history
    pub fn detect_conventions(&self) -> HashMap<String, usize> {
        let mut conventions = HashMap::new();

        for msg in &self.author_history {
            if let Some(first_word) = msg.split_whitespace().next() {
                // Check for conventional commit patterns
                if first_word.ends_with(':') {
                    let convention = first_word.to_lowercase();
                    *conventions.entry(convention).or_insert(0) += 1;
                }
                // Check for imperative verbs
                else if is_imperative_verb(first_word) {
                    *conventions.entry("imperative".to_string()).or_insert(0) += 1;
                }
            }
        }

        conventions
    }

    /// Get enhanced author history (simplified to just recent history)
    pub fn get_enhanced_history(&self, max_history: usize) -> Vec<String> {
        self.author_history
            .iter()
            .take(max_history)
            .cloned()
            .collect()
    }
}

/// Check if a word is an imperative verb commonly used in commit messages
fn is_imperative_verb(word: &str) -> bool {
    let imperative_verbs = [
        "add",
        "update",
        "fix",
        "remove",
        "refactor",
        "improve",
        "change",
        "modify",
        "create",
        "delete",
        "merge",
        "revert",
        "implement",
        "optimize",
        "clean",
        "rename",
        "move",
        "extract",
        "introduce",
        "enhance",
        "simplify",
        "document",
    ];

    imperative_verbs.contains(&word.to_lowercase().as_str())
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
        assert!(buffer.push(1)); // Should succeed
        assert!(buffer.push(2)); // Should succeed
        assert!(buffer.push(3)); // Should succeed
        assert!(!buffer.push(4)); // Should fail - buffer is full

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
