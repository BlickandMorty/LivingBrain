//! Contradiction detection between facts.
//!
//! Instead of silently overwriting old facts with new ones,
//! detects conflicts and surfaces them for human resolution.
//! Inspired by BeliefShift (arXiv:2603.23848).

use serde::{Deserialize, Serialize};

/// Type of contradiction detected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Numbers differ: "$3.00" vs "$5.00"
    Numeric,
    /// Boolean contradiction: "enabled" vs "disabled"
    Boolean,
    /// Opposite terms: "fast" vs "slow", "increase" vs "decrease"
    Antonym,
    /// Semantically reversed meaning
    SemanticReversal,
}

/// A detected contradiction between two facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub conflict_type: ConflictType,
    pub confidence: f64,
    pub old_value: String,
    pub new_value: String,
}

/// Detect if two facts contradict each other.
///
/// Returns None if no contradiction detected.
pub fn detect(old_fact: &str, new_fact: &str) -> Option<Contradiction> {
    // Numeric contradiction: extract numbers and compare
    if let Some(c) = detect_numeric(old_fact, new_fact) {
        return Some(c);
    }

    // Boolean contradiction
    if let Some(c) = detect_boolean(old_fact, new_fact) {
        return Some(c);
    }

    // Antonym pairs
    if let Some(c) = detect_antonym(old_fact, new_fact) {
        return Some(c);
    }

    None
}

fn detect_numeric(old: &str, new: &str) -> Option<Contradiction> {
    let old_nums = extract_numbers(old);
    let new_nums = extract_numbers(new);

    if old_nums.is_empty() || new_nums.is_empty() {
        return None;
    }

    // Check if texts are similar except for numbers
    let old_stripped = strip_numbers(old);
    let new_stripped = strip_numbers(new);
    let similarity = jaccard(&old_stripped, &new_stripped);

    if similarity > 0.6 && old_nums != new_nums {
        return Some(Contradiction {
            conflict_type: ConflictType::Numeric,
            confidence: similarity,
            old_value: old_nums.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", "),
            new_value: new_nums.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", "),
        });
    }
    None
}

fn detect_boolean(old: &str, new: &str) -> Option<Contradiction> {
    let old_lower = old.to_lowercase();
    let new_lower = new.to_lowercase();

    let bool_pairs = [
        ("enabled", "disabled"), ("true", "false"), ("yes", "no"),
        ("active", "inactive"), ("on", "off"), ("allow", "deny"),
        ("open", "closed"), ("public", "private"),
    ];

    for (pos, neg) in &bool_pairs {
        let old_pos = old_lower.contains(pos);
        let old_neg = old_lower.contains(neg);
        let new_pos = new_lower.contains(pos);
        let new_neg = new_lower.contains(neg);

        if (old_pos && new_neg) || (old_neg && new_pos) {
            return Some(Contradiction {
                conflict_type: ConflictType::Boolean,
                confidence: 0.85,
                old_value: if old_pos { pos.to_string() } else { neg.to_string() },
                new_value: if new_pos { pos.to_string() } else { neg.to_string() },
            });
        }
    }
    None
}

fn detect_antonym(old: &str, new: &str) -> Option<Contradiction> {
    let old_lower = old.to_lowercase();
    let new_lower = new.to_lowercase();

    let antonym_pairs = [
        ("increase", "decrease"), ("fast", "slow"), ("high", "low"),
        ("better", "worse"), ("more", "less"), ("always", "never"),
        ("before", "after"), ("above", "below"), ("success", "failure"),
    ];

    for (a, b) in &antonym_pairs {
        if (old_lower.contains(a) && new_lower.contains(b))
            || (old_lower.contains(b) && new_lower.contains(a))
        {
            let similarity = jaccard(&old_lower, &new_lower);
            if similarity > 0.4 {
                return Some(Contradiction {
                    conflict_type: ConflictType::Antonym,
                    confidence: similarity,
                    old_value: a.to_string(),
                    new_value: b.to_string(),
                });
            }
        }
    }
    None
}

fn extract_numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

fn strip_numbers(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_digit() || c == '.' { ' ' } else { c })
        .collect()
}

fn jaccard(a: &str, b: &str) -> f64 {
    let wa: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let wb: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if wa.is_empty() && wb.is_empty() { return 1.0; }
    let inter = wa.intersection(&wb).count() as f64;
    let union = wa.union(&wb).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_numeric_contradiction() {
        let c = detect(
            "Claude Sonnet input pricing is $3.00 per million tokens",
            "Claude Sonnet input pricing is $5.00 per million tokens",
        );
        assert!(c.is_some());
        assert_eq!(c.unwrap().conflict_type, ConflictType::Numeric);
    }

    #[test]
    fn detects_boolean_contradiction() {
        let c = detect("Feature flag is enabled", "Feature flag is disabled");
        assert!(c.is_some());
        assert_eq!(c.unwrap().conflict_type, ConflictType::Boolean);
    }

    #[test]
    fn detects_antonym_contradiction() {
        let c = detect(
            "Model performance shows a fast increase in accuracy",
            "Model performance shows a slow decrease in accuracy",
        );
        assert!(c.is_some());
    }

    #[test]
    fn no_contradiction_for_unrelated() {
        let c = detect("The sky is blue", "Rust is a programming language");
        assert!(c.is_none());
    }

    #[test]
    fn no_contradiction_for_identical() {
        let c = detect("same fact here", "same fact here");
        assert!(c.is_none());
    }
}
