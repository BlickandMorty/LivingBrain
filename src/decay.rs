//! Exponential decay with an access-count adjustment.
//!
//! strength(t) = s0 * exp(-effective_rate * elapsed_days), where
//! effective_rate = base_rate / (1 + ln(max(access_count, 1))).
//! These are software rules, not parameters fitted to human memory data.
//! Pinned facts are exempt from decay.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default strength threshold for garbage collection.
pub const DEFAULT_GARBAGE_THRESHOLD: f64 = 0.15;

/// Importance level determines base decay rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Importance {
    /// Safety-critical, identity, API keys. λ=0.005/day (~139-day half-life).
    Critical,
    /// Active projects, key decisions. λ=0.01/day (~69-day half-life).
    High,
    /// Regular notes, meeting summaries. λ=0.05/day (~14-day half-life).
    Normal,
    /// Drafts, temporary thoughts. λ=0.1/day (~7-day half-life).
    Low,
}

impl Importance {
    /// Base decay rate (λ) per day.
    pub fn decay_rate(self) -> f64 {
        match self {
            Self::Critical => 0.005,
            Self::High => 0.01,
            Self::Normal => 0.05,
            Self::Low => 0.1,
        }
    }
}

/// Strength metadata for a memory node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeStrength {
    /// Current strength (0.0–1.0). Decays over time.
    pub strength: f64,
    /// Importance level — determines base decay rate.
    pub importance: Importance,
    /// Base decay rate (precomputed from importance).
    pub decay_rate: f64,
    /// When this fact was last accessed (read or reinforced).
    pub last_accessed: DateTime<Utc>,
    /// Total access count — drives conceptual inertia.
    pub access_count: u32,
    /// Pinned facts never decay (strength locked at 1.0).
    pub pinned: bool,
}

impl NodeStrength {
    pub fn new(importance: Importance, strength: f64, last_accessed: DateTime<Utc>) -> Self {
        Self {
            strength: strength.clamp(0.0, 1.0),
            importance,
            decay_rate: importance.decay_rate(),
            last_accessed,
            access_count: 0,
            pinned: false,
        }
    }

    /// Access-count heuristic: frequently accessed entries decay more slowly.
    ///
    /// Logarithmic damping: access_count=1 → 1.0x decay, 50 → ~0.20x, 1000 → ~0.14x.
    pub fn effective_decay_rate(&self) -> f64 {
        self.decay_rate / (1.0 + (self.access_count.max(1) as f64).ln())
    }

    /// Whether this fact should be garbage collected.
    pub fn should_gc(&self, threshold: f64) -> bool {
        !self.pinned && self.strength < threshold
    }
}

/// Apply the exponential decay rule to a single node.
pub fn decay(node: &mut NodeStrength, now: DateTime<Utc>) {
    if now <= node.last_accessed {
        return;
    }
    if node.pinned {
        node.strength = 1.0;
        node.last_accessed = now;
        return;
    }

    let elapsed_seconds = (now - node.last_accessed).num_seconds();
    if elapsed_seconds <= 0 {
        return;
    }

    let elapsed_days = elapsed_seconds as f64 / 86_400.0;
    let effective_rate = node.effective_decay_rate();
    let decay_factor = (-effective_rate * elapsed_days).exp();
    node.strength = (node.strength * decay_factor).clamp(0.0, 1.0);
    node.last_accessed = now;
}

/// Reinforce a fact — resets strength to 1.0 and increments access count.
pub fn access(node: &mut NodeStrength) {
    node.strength = 1.0;
    node.access_count = node.access_count.saturating_add(1);
    node.last_accessed = Utc::now();
}

/// Pin a fact — strength locked at 1.0, immune to decay.
pub fn pin(node: &mut NodeStrength) {
    node.pinned = true;
    node.strength = 1.0;
}

/// Apply decay to a batch of nodes efficiently.
pub fn batch_decay(nodes: &mut [NodeStrength], now: DateTime<Utc>) {
    for node in nodes {
        decay(node, now);
    }
}

/// Remove facts with strength below threshold. Returns removed nodes.
pub fn collect_garbage(nodes: &mut Vec<NodeStrength>, threshold: f64) -> Vec<NodeStrength> {
    let threshold = if threshold <= 0.0 { DEFAULT_GARBAGE_THRESHOLD } else { threshold };
    let mut removed = Vec::new();
    let mut kept = Vec::with_capacity(nodes.len());

    for node in nodes.drain(..) {
        if !node.pinned && node.strength < threshold {
            removed.push(node);
        } else {
            kept.push(node);
        }
    }

    *nodes = kept;
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn node(importance: Importance, strength: f64) -> NodeStrength {
        NodeStrength::new(importance, strength, Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap())
    }

    #[test]
    fn decay_follows_ebbinghaus_curve() {
        let mut n = node(Importance::Normal, 1.0);
        let now = n.last_accessed + Duration::days(20);
        decay(&mut n, now);
        assert!((n.strength - std::f64::consts::E.powf(-1.0)).abs() < 0.02);
    }

    #[test]
    fn pinned_nodes_never_decay() {
        let mut n = node(Importance::Low, 0.42);
        pin(&mut n);
        let future = n.last_accessed + Duration::days(365);
        decay(&mut n, future);
        assert_eq!(n.strength, 1.0);
    }

    #[test]
    fn access_resets_and_increments() {
        let mut n = node(Importance::High, 0.14);
        n.access_count = 3;
        access(&mut n);
        assert_eq!(n.strength, 1.0);
        assert_eq!(n.access_count, 4);
    }

    #[test]
    fn conceptual_inertia_slows_decay() {
        let mut rarely = node(Importance::Normal, 1.0);
        rarely.access_count = 1;
        let mut frequently = node(Importance::Normal, 1.0);
        frequently.access_count = 50;

        let now = rarely.last_accessed + Duration::days(30);
        decay(&mut rarely, now);
        decay(&mut frequently, now);

        assert!(frequently.strength > rarely.strength * 2.0);
    }

    #[test]
    fn garbage_collection_removes_weak() {
        let mut nodes = vec![
            node(Importance::Low, 0.05),
            node(Importance::Normal, 0.8),
            { let mut n = node(Importance::Low, 0.01); pin(&mut n); n },
        ];
        let removed = collect_garbage(&mut nodes, 0.15);
        assert_eq!(removed.len(), 1);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn batch_decay_handles_10k_nodes() {
        let mut nodes: Vec<_> = (0..10_000).map(|_| node(Importance::Normal, 1.0)).collect();
        let now = Utc.with_ymd_and_hms(2026, 3, 11, 12, 0, 0).unwrap();
        let started = std::time::Instant::now();
        batch_decay(&mut nodes, now);
        assert!(started.elapsed().as_millis() < 50);
        assert!(nodes.iter().all(|n| n.strength < 1.0));
    }
}
