use anyhow::Result;
use git2::Repository;

/// Checks if the current directory is inside a Git work tree.
///
/// # Returns
///
/// A Result containing a boolean indicating if inside a work tree or an error.
#[inline]
pub fn is_inside_work_tree() -> Result<bool> {
    match Repository::discover(".") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Determines if the given diff represents a binary file.
#[inline]
pub fn is_binary_diff(diff: &str) -> bool {
    diff.contains("Binary files")
        || diff.contains("GIT binary patch")
        || diff.contains("[Binary file changed]")
}
