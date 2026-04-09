//! Neural Cache — Tiered instant retrieval across memory layers.
//!
//! 5-layer architecture:
//! - Layer 0: Working context (current conversation) — 0ms
//! - Layer 1: Hot facts (in-memory LRU) — <1ms
//! - Layer 2: Warm search (user-provided backend) — <5ms
//! - Layer 3: Cold vault (filesystem fallback) — <50ms
//!
//! Facts automatically warm from Cold → Warm → Hot based on access patterns.
//! Extracted from Epistemos agent_core.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Which cache layer a result was retrieved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheLayer {
    /// Layer 1: Hot facts (pre-warmed, <1ms).
    Hot,
    /// Layer 2: Warm search (backend-provided, <5ms).
    Warm,
    /// Layer 3: Cold vault (filesystem, <50ms).
    Cold,
}

/// Result from a tiered cache lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub path: String,
    pub content: String,
    pub score: f64,
    pub layer: CacheLayer,
    /// Retrieval latency in microseconds.
    pub latency_us: u64,
}

/// A cached fact in the hot layer.
#[derive(Debug, Clone)]
struct HotFact {
    content: String,
    path: String,
    score: f64,
    last_accessed: Instant,
    access_count: u32,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Tiered retrieval cache with hot-warm-cold layers.
///
/// ```
/// use livingbrain::cache::NeuralCache;
///
/// let cache = NeuralCache::new(1000); // 1000 hot entries max
/// cache.warm("notes/rust.md", "Rust FFI uses UniFFI", 0.9);
///
/// let results = cache.search_hot("Rust FFI", 5);
/// assert!(!results.is_empty());
/// ```
pub struct NeuralCache {
    hot: Mutex<HotLayer>,
    max_hot_entries: usize,
}

struct HotLayer {
    facts: HashMap<String, HotFact>,
}

impl NeuralCache {
    /// Create a new cache with the given hot layer capacity.
    pub fn new(max_hot_entries: usize) -> Self {
        Self {
            hot: Mutex::new(HotLayer {
                facts: HashMap::with_capacity(max_hot_entries),
            }),
            max_hot_entries,
        }
    }

    /// Warm a fact into the hot layer. Called after retrieval from deeper layers.
    pub fn warm(&self, path: &str, content: &str, score: f64) {
        let mut hot = match self.hot.lock() {
            Ok(h) => h,
            Err(_) => return,
        };

        if let Some(existing) = hot.facts.get_mut(path) {
            existing.access_count += 1;
            existing.last_accessed = Instant::now();
            existing.score = existing.score.max(score);
            return;
        }

        // LRU eviction at capacity
        if hot.facts.len() >= self.max_hot_entries {
            let oldest_key = hot.facts.iter()
                .min_by_key(|(_, f)| f.last_accessed)
                .map(|(k, _)| k.clone());
            if let Some(key) = oldest_key {
                hot.facts.remove(&key);
            }
        }

        hot.facts.insert(path.to_string(), HotFact {
            content: content.to_string(),
            path: path.to_string(),
            score,
            last_accessed: Instant::now(),
            access_count: 1,
            created_at: chrono::Utc::now(),
        });
    }

    /// Layer 1 lookup: instant keyword matching against cached content (<1ms).
    pub fn search_hot(&self, query: &str, limit: usize) -> Vec<CachedResult> {
        let start = Instant::now();
        let hot = match self.hot.lock() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        if query_words.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(f64, &HotFact)> = hot.facts.values()
            .filter_map(|fact| {
                let content_lower = fact.content.to_lowercase();
                let overlap = query_words.iter().filter(|w| content_lower.contains(*w)).count();
                if overlap > 0 {
                    let relevance = overlap as f64 / query_words.len() as f64;
                    let recency_boost = 1.0 / (fact.last_accessed.elapsed().as_secs_f64() + 1.0);
                    Some((relevance * 0.7 + fact.score * 0.2 + recency_boost * 0.1, fact))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let latency = start.elapsed().as_micros() as u64;

        scored.into_iter().take(limit).map(|(score, fact)| CachedResult {
            path: fact.path.clone(),
            content: fact.content.clone(),
            score,
            layer: CacheLayer::Hot,
            latency_us: latency,
        }).collect()
    }

    /// Temporal query: retrieve facts created within a time window.
    ///
    /// `minutes_ago`: how far back to start the window.
    /// `window_minutes`: size of the window.
    pub fn temporal_retrieve(&self, minutes_ago: u64, window_minutes: u64) -> Vec<CachedResult> {
        let start = Instant::now();
        let hot = match self.hot.lock() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let now = chrono::Utc::now();
        let window_start = now - chrono::Duration::minutes((minutes_ago + window_minutes) as i64);
        let window_end = now - chrono::Duration::minutes(minutes_ago as i64);

        let mut results: Vec<CachedResult> = hot.facts.values()
            .filter(|f| f.created_at >= window_start && f.created_at <= window_end)
            .map(|f| CachedResult {
                path: f.path.clone(),
                content: f.content.clone(),
                score: f.score,
                layer: CacheLayer::Hot,
                latency_us: start.elapsed().as_micros() as u64,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Cache statistics for diagnostics.
    pub fn stats(&self) -> CacheStats {
        let hot = self.hot.lock().map(|h| h.facts.len()).unwrap_or(0);
        CacheStats { hot_entries: hot, max_hot_entries: self.max_hot_entries }
    }

    /// Clear the hot cache entirely.
    pub fn clear_hot(&self) {
        if let Ok(mut hot) = self.hot.lock() {
            hot.facts.clear();
        }
    }
}

/// Diagnostic statistics for the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hot_entries: usize,
    pub max_hot_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warm_and_retrieve() {
        let cache = NeuralCache::new(100);
        cache.warm("notes/rust.md", "Rust FFI bridge uses UniFFI for Swift interop", 0.9);
        cache.warm("notes/swift.md", "Swift actors provide data isolation", 0.8);

        let results = cache.search_hot("Rust FFI bridge", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].layer, CacheLayer::Hot);
        assert!(results[0].latency_us < 1000);
    }

    #[test]
    fn lru_eviction() {
        let cache = NeuralCache::new(2);
        cache.warm("a.md", "content a", 0.9);
        cache.warm("b.md", "content b", 0.8);
        std::thread::sleep(std::time::Duration::from_millis(1));
        cache.warm("c.md", "content c", 0.7);
        assert_eq!(cache.stats().hot_entries, 2);
    }

    #[test]
    fn access_count_increments() {
        let cache = NeuralCache::new(100);
        cache.warm("t.md", "content", 0.5);
        cache.warm("t.md", "content", 0.6);
        cache.warm("t.md", "content", 0.7);
        let hot = cache.hot.lock().unwrap();
        let fact = hot.facts.get("t.md").unwrap();
        assert_eq!(fact.access_count, 3);
    }

    #[test]
    fn clear_works() {
        let cache = NeuralCache::new(100);
        cache.warm("a.md", "content", 0.5);
        assert_eq!(cache.stats().hot_entries, 1);
        cache.clear_hot();
        assert_eq!(cache.stats().hot_entries, 0);
    }

    #[test]
    fn empty_query_returns_empty() {
        let cache = NeuralCache::new(100);
        cache.warm("a.md", "content", 0.5);
        assert!(cache.search_hot("", 5).is_empty());
    }
}
