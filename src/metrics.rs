//! Heuristic summaries of tool-call sequences.
//!
//! Text-overlap distances, repetition, and errors describe a trace. Category
//! names do not establish reasoning quality, progress, or hallucination.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Metrics computed from an agent's tool call sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryMetrics {
    /// Semantic distance from first tool call to last (Jaccard proxy).
    pub displacement: f32,
    /// Total semantic distance across all consecutive tool calls.
    pub path_length: f32,
    /// path_length / displacement. >4.0 = hesitation loop.
    pub curvature_ratio: f32,
    /// Repeated tool+args hash count.
    pub loop_count: u32,
    /// Tool calls that returned errors.
    pub error_count: u32,
    /// Total tool calls.
    pub total_calls: u32,
    /// displacement / total_calls.
    pub efficiency: f32,
    /// Overall quality classification.
    pub classification: Classification,
}

/// Quality classification of an agent's reasoning trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classification {
    /// Direct path to goal. Curvature < 2.0, no loops.
    Efficient,
    /// Broad search making progress. Curvature 2.0–4.0.
    Exploratory,
    /// Going in circles. Curvature > 4.0 or 3+ loops.
    Hesitating,
    /// No progress despite effort. Displacement < 0.1.
    Stuck,
    /// More than half the calls errored.
    Failed,
}

/// Compute trajectory metrics from tool call log.
///
/// Each entry: (tool_name, args_json, result_text, is_error).
pub fn compute(tool_calls: &[(String, String, String, bool)]) -> TrajectoryMetrics {
    let total_calls = tool_calls.len() as u32;

    if tool_calls.is_empty() {
        return TrajectoryMetrics {
            displacement: 0.0, path_length: 0.0, curvature_ratio: 0.0,
            loop_count: 0, error_count: 0, total_calls: 0,
            efficiency: 0.0, classification: Classification::Stuck,
        };
    }

    // Loop detection
    let mut call_hashes: HashMap<u64, u32> = HashMap::new();
    for (name, args, _, _) in tool_calls {
        let hash = djb2_hash(&format!("{name}:{args}"));
        *call_hashes.entry(hash).or_insert(0) += 1;
    }
    let loop_count: u32 = call_hashes.values().filter(|&&c| c >= 2).map(|c| c - 1).sum();

    let error_count = tool_calls.iter().filter(|(_, _, _, err)| *err).count() as u32;

    // Pairwise Jaccard distances
    let mut path_length: f32 = 0.0;
    for i in 1..tool_calls.len() {
        path_length += jaccard_distance(&tool_calls[i - 1].2, &tool_calls[i].2);
    }

    let displacement = if tool_calls.len() >= 2 {
        jaccard_distance(&tool_calls[0].2, &tool_calls[tool_calls.len() - 1].2)
    } else {
        0.5
    };

    let curvature_ratio = if displacement > 0.001 {
        path_length / displacement
    } else {
        f32::MAX
    };

    let efficiency = if total_calls > 0 { displacement / total_calls as f32 } else { 0.0 };

    let classification = if error_count > total_calls / 2 {
        Classification::Failed
    } else if curvature_ratio > 4.0 || loop_count >= 3 {
        Classification::Hesitating
    } else if displacement < 0.1 && total_calls > 3 {
        Classification::Stuck
    } else if curvature_ratio > 2.0 && displacement > 0.3 {
        Classification::Exploratory
    } else {
        Classification::Efficient
    };

    TrajectoryMetrics {
        displacement, path_length, curvature_ratio, loop_count,
        error_count, total_calls, efficiency, classification,
    }
}

fn jaccard_distance(a: &str, b: &str) -> f32 {
    let wa: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let wb: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if wa.is_empty() && wb.is_empty() { return 0.0; }
    let inter = wa.intersection(&wb).count();
    let union = wa.union(&wb).count();
    if union == 0 { 0.0 } else { 1.0 - (inter as f32 / union as f32) }
}

fn djb2_hash(s: &str) -> u64 {
    let mut h: u64 = 5381;
    for b in s.bytes() { h = h.wrapping_mul(33).wrapping_add(b as u64); }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_stuck() {
        assert_eq!(compute(&[]).classification, Classification::Stuck);
    }

    #[test]
    fn single_call_is_efficient() {
        let calls = vec![("search".into(), "q".into(), "found results".into(), false)];
        assert_eq!(compute(&calls).classification, Classification::Efficient);
    }

    #[test]
    fn repeated_calls_are_hesitating() {
        let calls: Vec<_> = (0..4).map(|_| ("s".into(), "q".into(), "r".into(), false)).collect();
        assert_eq!(compute(&calls).classification, Classification::Hesitating);
    }

    #[test]
    fn diverse_calls_are_efficient() {
        let calls = vec![
            ("search".into(), "a".into(), "found training docs".into(), false),
            ("read".into(), "b".into(), "full content with 15 categories".into(), false),
            ("read".into(), "c".into(), "evaluation showing 92% accuracy".into(), false),
        ];
        let m = compute(&calls);
        assert!(m.classification == Classification::Efficient || m.classification == Classification::Exploratory);
    }

    #[test]
    fn mostly_errors_is_failed() {
        let calls = vec![
            ("a".into(), "x".into(), "err".into(), true),
            ("b".into(), "y".into(), "err".into(), true),
            ("c".into(), "z".into(), "err".into(), true),
            ("d".into(), "w".into(), "ok".into(), false),
        ];
        assert_eq!(compute(&calls).classification, Classification::Failed);
    }
}
