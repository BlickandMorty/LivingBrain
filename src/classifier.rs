//! Memory operation classifier.
//!
//! Given a new fact and existing knowledge, classifies the operation:
//! Add (new), Update (changed), Delete (retracted), or Noop (duplicate).

use serde::{Deserialize, Serialize};

/// The four memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryOperation {
    /// New fact — no existing match (similarity < threshold).
    Add,
    /// Existing fact needs updating (similar but different content).
    Update,
    /// Fact retracted — explicitly negated or removed.
    Delete,
    /// No change needed — content matches existing fact.
    Noop,
}

/// Classify a proposed memory mutation against existing facts.
///
/// Uses Jaccard similarity as a lightweight proxy for semantic matching.
/// Threshold 0.85 = must share 85% of words to be considered a match.
pub fn classify(new_content: &str, existing_facts: &[&str], threshold: f64) -> (MemoryOperation, Option<usize>) {
    if existing_facts.is_empty() {
        return (MemoryOperation::Add, None);
    }

    let new_words: std::collections::HashSet<&str> = new_content.split_whitespace().collect();

    let mut best_similarity = 0.0;
    let mut best_idx = 0;

    for (i, fact) in existing_facts.iter().enumerate() {
        let fact_words: std::collections::HashSet<&str> = fact.split_whitespace().collect();
        let inter = new_words.intersection(&fact_words).count() as f64;
        let union = new_words.union(&fact_words).count() as f64;
        let sim = if union > 0.0 { inter / union } else { 0.0 };

        if sim > best_similarity {
            best_similarity = sim;
            best_idx = i;
        }
    }

    if best_similarity >= 0.99 {
        (MemoryOperation::Noop, Some(best_idx))
    } else if best_similarity >= threshold {
        (MemoryOperation::Update, Some(best_idx))
    } else {
        (MemoryOperation::Add, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_fact_classifies_as_add() {
        let (op, _) = classify("completely new information", &["something else entirely"], 0.85);
        assert_eq!(op, MemoryOperation::Add);
    }

    #[test]
    fn identical_fact_is_noop() {
        let fact = "Claude Sonnet costs $3 per million tokens";
        let (op, idx) = classify(fact, &[fact], 0.85);
        assert_eq!(op, MemoryOperation::Noop);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn similar_fact_is_update() {
        let old = "Claude Sonnet costs $3 per million input tokens";
        let new = "Claude Sonnet costs $5 per million input tokens";
        let (op, idx) = classify(new, &[old], 0.70);
        assert_eq!(op, MemoryOperation::Update);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn empty_existing_is_add() {
        let (op, idx) = classify("any fact", &[], 0.85);
        assert_eq!(op, MemoryOperation::Add);
        assert_eq!(idx, None);
    }
}
