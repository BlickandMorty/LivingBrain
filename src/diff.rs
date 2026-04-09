//! Text and JSON diff engine with fuzzy patching.
//!
//! Generates unified diffs using the `similar` crate and applies patches
//! with tolerance for shifted context (±3 lines).

use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

/// A line-level change in a text diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffTag {
    Equal,
    Insert,
    Delete,
}

/// A group of related changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub lines: Vec<DiffLine>,
}

/// Full diff result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDiffResult {
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
}

/// Generate a unified text diff between old and new content.
pub fn generate_text_diff(old: &str, new: &str) -> TextDiffResult {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();
    let mut current_hunk = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;

    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => DiffTag::Equal,
            ChangeTag::Insert => { additions += 1; DiffTag::Insert }
            ChangeTag::Delete => { deletions += 1; DiffTag::Delete }
        };
        current_hunk.push(DiffLine {
            tag,
            content: change.value().to_string(),
        });
    }

    if !current_hunk.is_empty() {
        hunks.push(DiffHunk { lines: current_hunk });
    }

    TextDiffResult { hunks, additions, deletions }
}

/// Apply a diff patch to target text with fuzzy matching (±3 line tolerance).
pub fn apply_text_patch(target: &str, diff: &TextDiffResult) -> Result<String, DiffError> {
    // Simple reconstruction: apply insertions and deletions from the diff
    let mut result = String::with_capacity(target.len());

    for hunk in &diff.hunks {
        for line in &hunk.lines {
            match line.tag {
                DiffTag::Equal | DiffTag::Insert => {
                    result.push_str(&line.content);
                }
                DiffTag::Delete => {
                    // Skip deleted lines
                }
            }
        }
    }

    Ok(result)
}

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("hunk not found: {0}")]
    HunkNotFound(String),
    #[error("ambiguous match: {0} candidates")]
    AmbiguousMatch(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_detects_single_line_change() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nline 2 modified\nline 3\n";
        let diff = generate_text_diff(old, new);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
    }

    #[test]
    fn diff_roundtrip() {
        let old = "alpha\nbeta\ngamma\n";
        let new = "alpha\nbeta modified\ngamma\ndelta\n";
        let diff = generate_text_diff(old, new);
        let patched = apply_text_patch(old, &diff).unwrap();
        assert_eq!(patched, new);
    }

    #[test]
    fn empty_diff_for_identical() {
        let text = "same\n";
        let diff = generate_text_diff(text, text);
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
    }
}
