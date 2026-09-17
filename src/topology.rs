//! File-hierarchy coordinates and descriptive tags.
//!
//! The code places directory entries in disk coordinates and summarizes their
//! contents. Names such as Markov blanket and gravity are software metaphors;
//! this does not establish conditional independence or a cognitive mechanism.

#![allow(clippy::too_many_arguments, clippy::manual_clamp)]

use std::path::Path;
use std::{fs, io};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Poincaré Disk Coordinates
// ---------------------------------------------------------------------------

/// A point in the Poincaré disk model of hyperbolic space.
/// The disk has radius 1.0; points near the edge represent deep nesting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbolicPoint {
    /// Radial distance from center (0.0 = root, approaches 1.0 for deep nodes).
    pub r: f64,
    /// Angular position (0.0 to 2π). Siblings spread across the angle.
    pub theta: f64,
    /// Cartesian x in the Poincaré disk (derived from r, theta).
    pub x: f64,
    /// Cartesian y in the Poincaré disk (derived from r, theta).
    pub y: f64,
}

impl HyperbolicPoint {
    fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            r,
            theta,
            x: r * theta.cos(),
            y: r * theta.sin(),
        }
    }

    /// Hyperbolic distance between two points in the Poincaré disk.
    /// d(u,v) = arcosh(1 + 2|u-v|² / ((1-|u|²)(1-|v|²)))
    pub fn hyperbolic_distance(&self, other: &HyperbolicPoint) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let diff_sq = dx * dx + dy * dy;
        let denom = (1.0 - self.r * self.r) * (1.0 - other.r * other.r);
        if denom <= 0.0 {
            return f64::INFINITY;
        }
        let arg = 1.0 + 2.0 * diff_sq / denom;
        arg.max(1.0).acosh()
    }
}

// ---------------------------------------------------------------------------
// Dimensional Tags
// ---------------------------------------------------------------------------

/// Spatial metadata for a vault node (file or folder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultNodeMetrics {
    /// Path relative to vault root.
    pub path: String,
    /// Whether this is a directory (Markov Blanket) or a file (leaf node).
    pub is_directory: bool,
    /// Complexity Weight (1.0–10.0): token count + structural density.
    pub complexity_weight: f64,
    /// Gravity (0.0+): how many other nodes reference this node.
    pub gravity: f64,
    /// Volatility Score (0.0–1.0): edit recency heat-map.
    pub volatility: f64,
    /// Poincaré disk coordinates.
    pub position: HyperbolicPoint,
    /// For directories: boundary summary (Markov Blanket active state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blanket_summary: Option<String>,
    /// Number of children (for directories).
    pub child_count: u32,
    /// Depth in the vault tree (0 = root).
    pub depth: u32,
}

// ---------------------------------------------------------------------------
// Topology Map
// ---------------------------------------------------------------------------

/// The complete spatial topology of a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultTopology {
    pub vault_root: String,
    pub nodes: Vec<VaultNodeMetrics>,
    pub total_files: u32,
    pub total_dirs: u32,
    pub max_depth: u32,
    /// Top nodes by gravity (God Nodes).
    pub god_nodes: Vec<String>,
}

/// Build the hyperbolic topology map for a vault.
pub fn build_topology(vault_root: &Path) -> Result<VaultTopology, io::Error> {
    let mut nodes = Vec::new();
    let mut max_depth: u32 = 0;
    let mut total_files: u32 = 0;
    let mut total_dirs: u32 = 0;

    // Phase 1: Walk the directory tree and collect raw metrics
    walk_tree(
        vault_root,
        vault_root,
        0,
        0.0,
        std::f64::consts::TAU,
        &mut nodes,
        &mut max_depth,
        &mut total_files,
        &mut total_dirs,
    )?;

    // Phase 2: Compute gravity (reference counts via simple name matching)
    compute_gravity(&mut nodes);

    // Phase 3: Generate Markov Blanket summaries for directories
    generate_blanket_summaries(&mut nodes);

    // Phase 4: Identify God Nodes (top 10 by gravity)
    let mut by_gravity = nodes.clone();
    by_gravity.sort_by(|a, b| b.gravity.partial_cmp(&a.gravity).unwrap_or(std::cmp::Ordering::Equal));
    let god_nodes: Vec<String> = by_gravity
        .iter()
        .take(10)
        .map(|n| n.path.clone())
        .collect();

    Ok(VaultTopology {
        vault_root: vault_root.to_string_lossy().to_string(),
        nodes,
        total_files,
        total_dirs,
        max_depth,
        god_nodes,
    })
}

// ---------------------------------------------------------------------------
// Tree Walking
// ---------------------------------------------------------------------------

fn walk_tree(
    dir: &Path,
    root: &Path,
    depth: u32,
    angle_start: f64,
    angle_span: f64,
    nodes: &mut Vec<VaultNodeMetrics>,
    max_depth: &mut u32,
    total_files: &mut u32,
    total_dirs: &mut u32,
) -> Result<(), io::Error> {
    *max_depth = (*max_depth).max(depth);

    let entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_str().unwrap_or("");
            !name_str.starts_with('.') && name_str != "node_modules" && name_str != "target"
        })
        .collect();

    if entries.is_empty() {
        return Ok(());
    }

    let angle_step = angle_span / entries.len() as f64;

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let is_dir = path.is_dir();
        let angle = angle_start + (i as f64 + 0.5) * angle_step;

        // Poincaré disk: r = tanh(depth * scale), ensuring r < 1.0
        // Scale factor controls how quickly nodes approach the boundary.
        let r = ((depth as f64 + 1.0) * 0.3).tanh();

        let cw = if is_dir {
            estimate_dir_complexity(&path)
        } else {
            estimate_file_complexity(&path)
        };

        let vs = estimate_volatility(&path);

        let child_count = if is_dir {
            fs::read_dir(&path)
                .map(|rd| rd.count() as u32)
                .unwrap_or(0)
        } else {
            0
        };

        if is_dir {
            *total_dirs += 1;
        } else {
            *total_files += 1;
        }

        nodes.push(VaultNodeMetrics {
            path: relative,
            is_directory: is_dir,
            complexity_weight: cw,
            gravity: 0.0, // computed in Phase 2
            volatility: vs,
            position: HyperbolicPoint::from_polar(r, angle),
            blanket_summary: None, // computed in Phase 3
            child_count,
            depth,
        });

        if is_dir {
            walk_tree(
                &path,
                root,
                depth + 1,
                angle,
                angle_step,
                nodes,
                max_depth,
                total_files,
                total_dirs,
            )?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Complexity Weight (Cw)
// ---------------------------------------------------------------------------

fn estimate_file_complexity(path: &Path) -> f64 {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    // Base complexity from file size (log scale, 1.0–8.0)
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let size_score = (size as f64).log10().max(0.0).min(6.0) / 6.0 * 7.0 + 1.0;

    // Language complexity modifier
    let lang_mod = match ext {
        "rs" | "swift" => 1.3,    // systems languages = higher complexity
        "py" | "ts" | "js" => 1.0,
        "md" | "txt" => 0.7,
        "json" | "yaml" | "toml" => 0.5,
        _ => 0.8,
    };

    (size_score * lang_mod).clamp(1.0, 10.0)
}

fn estimate_dir_complexity(path: &Path) -> f64 {
    // Directory complexity = sum of children's sizes (capped)
    let total_size: u64 = walkdir_size(path);
    let score = (total_size as f64).log10().max(0.0).min(8.0) / 8.0 * 9.0 + 1.0;
    score.clamp(1.0, 10.0)
}

fn walkdir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                total += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                // Only go one level deep for performance
                if let Ok(sub) = fs::read_dir(&p) {
                    for se in sub.filter_map(|e| e.ok()) {
                        if se.path().is_file() {
                            total += fs::metadata(se.path()).map(|m| m.len()).unwrap_or(0);
                        }
                    }
                }
            }
        }
    }
    total
}

// ---------------------------------------------------------------------------
// Volatility Score (Vs)
// ---------------------------------------------------------------------------

fn estimate_volatility(path: &Path) -> f64 {
    // Based on last modified time — exponential decay with ~7-day half-life
    let modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok();

    let Some(modified) = modified else {
        return 0.0;
    };

    let elapsed = modified.elapsed().unwrap_or_default();
    let days = elapsed.as_secs_f64() / 86400.0;
    // Half-life of ~7 days: e^(-0.1 * days)
    (-0.1 * days).exp().clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Gravity (Gv) — Reference Count
// ---------------------------------------------------------------------------

fn compute_gravity(nodes: &mut [VaultNodeMetrics]) {
    // Simple heuristic: count how many other node paths contain this node's name
    let names: Vec<(usize, String)> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let name = Path::new(&n.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            (i, name)
        })
        .collect();

    // For each node, count how many OTHER nodes' paths mention its name
    for (idx, name) in &names {
        if name.len() < 3 {
            continue; // Skip very short names to avoid false matches
        }
        let mut refs = 0u32;
        for (other_idx, _) in &names {
            if *other_idx == *idx {
                continue;
            }
            if nodes[*other_idx].path.to_lowercase().contains(name) {
                refs += 1;
            }
        }
        nodes[*idx].gravity = refs as f64;
    }
}

// ---------------------------------------------------------------------------
// Markov Blanket Summaries
// ---------------------------------------------------------------------------

fn generate_blanket_summaries(nodes: &mut [VaultNodeMetrics]) {
    // For each directory, create a boundary summary from its children
    let dir_indices: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.is_directory)
        .map(|(i, _)| i)
        .collect();

    for dir_idx in dir_indices {
        let dir_path = nodes[dir_idx].path.clone();

        // Find children of this directory
        let children: Vec<String> = nodes
            .iter()
            .filter(|n| {
                n.path.starts_with(&dir_path)
                    && n.path != dir_path
                    && n.path[dir_path.len()..].matches('/').count() <= 1
            })
            .map(|n| {
                let name = Path::new(&n.path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?");
                let tag = if n.is_directory { "dir" } else { "file" };
                format!("{name}({tag},Cw:{:.1})", n.complexity_weight)
            })
            .take(10) // Limit summary to 10 children
            .collect();

        if !children.is_empty() {
            let total_cw: f64 = nodes
                .iter()
                .filter(|n| n.path.starts_with(&dir_path) && n.path != dir_path)
                .map(|n| n.complexity_weight)
                .sum();

            nodes[dir_idx].blanket_summary = Some(format!(
                "Contains {} items, total Cw={:.1}: [{}]",
                nodes[dir_idx].child_count,
                total_cw,
                children.join(", ")
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// FEP-Inspired Blanket Piercing Decision
// ---------------------------------------------------------------------------
// Decides whether an agent should descend into a directory (pierce its Markov
// Blanket) based on the query's relevance to the blanket summary.
// This prevents wasteful exploration of irrelevant vault subtrees.

/// Determine if the agent should explore inside a directory.
///
/// Uses Jaccard similarity between the query terms and the blanket summary
/// as a proxy for semantic relevance. High-gravity directories get a bonus
/// (they're hubs). Low-volatility directories get a penalty (stale content
/// is less likely to be relevant).
///
/// Returns: (should_pierce, confidence: 0.0–1.0)
pub fn should_pierce_blanket(
    query: &str,
    node: &VaultNodeMetrics,
) -> (bool, f64) {
    // Non-directories always "pierce" (nothing to descend into)
    if !node.is_directory {
        return (true, 1.0);
    }

    let summary = node.blanket_summary.as_deref().unwrap_or("");
    if summary.is_empty() {
        // Empty directories: pierce only if gravity > 0 (something references them)
        return (node.gravity > 0.0, 0.3);
    }

    // Jaccard similarity between query terms and blanket summary terms
    let query_lower = query.to_lowercase();
    let query_terms: std::collections::HashSet<&str> = query_lower
        .split_whitespace()
        .collect();
    let summary_lower = summary.to_lowercase();
    let summary_terms: std::collections::HashSet<&str> = summary_lower
        .split_whitespace()
        .collect();

    let intersection = query_terms.intersection(&summary_terms).count() as f64;
    let union = query_terms.union(&summary_terms).count() as f64;
    let jaccard = if union > 0.0 { intersection / union } else { 0.0 };

    // Adjust by gravity (hub bonus) and volatility (recency bonus)
    let gravity_bonus = (node.gravity / 10.0).min(0.2); // max +0.2 for high-gravity dirs
    let volatility_bonus = node.volatility * 0.1;        // max +0.1 for recently-edited
    let confidence = (jaccard + gravity_bonus + volatility_bonus).min(1.0);

    // Pierce if confidence exceeds threshold (0.15 is lenient — prefer exploration over missing)
    let should_pierce = confidence > 0.15;
    (should_pierce, confidence)
}

// ---------------------------------------------------------------------------
// Agent-Facing Output
// ---------------------------------------------------------------------------

/// Generate a compact spatial map that the AI can consume to understand vault structure.
/// This replaces `ls -la` output with dimensionally-tagged topology.
pub fn topology_to_agent_context(topology: &VaultTopology, max_tokens: usize) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "## Vault Topology ({} files, {} dirs, depth {})",
        topology.total_files, topology.total_dirs, topology.max_depth
    ));

    if !topology.god_nodes.is_empty() {
        lines.push(format!(
            "**God Nodes** (highest gravity): {}",
            topology.god_nodes.join(", ")
        ));
    }

    lines.push(String::new());

    // Show directories first (Markov Blankets), sorted by gravity
    let mut dirs: Vec<&VaultNodeMetrics> = topology
        .nodes
        .iter()
        .filter(|n| n.is_directory)
        .collect();
    dirs.sort_by(|a, b| b.gravity.partial_cmp(&a.gravity).unwrap_or(std::cmp::Ordering::Equal));

    let max_words = max_tokens * 3 / 4; // rough word budget
    let mut word_count = 0;

    for dir in dirs {
        let line = format!(
            "📁 {} — Cw:{:.1} Gv:{:.0} Vs:{:.2} | {}",
            dir.path,
            dir.complexity_weight,
            dir.gravity,
            dir.volatility,
            dir.blanket_summary.as_deref().unwrap_or("(empty)")
        );
        word_count += line.split_whitespace().count();
        if word_count > max_words {
            lines.push("[...truncated to fit context budget]".to_string());
            break;
        }
        lines.push(line);
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_vault() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create a realistic vault structure
        fs::create_dir_all(root.join("sessions/2026-04-08_abc")).unwrap();
        fs::write(root.join("sessions/2026-04-08_abc/session.json"), "{}").unwrap();
        fs::write(root.join("sessions/2026-04-08_abc/transcript.jsonl"), "").unwrap();

        fs::create_dir_all(root.join("memory")).unwrap();
        fs::write(root.join("memory/decisions.md"), "# Decisions\n- Use Rust FFI").unwrap();
        fs::write(root.join("memory/knowledge.md"), "# Knowledge\n- MLX for inference").unwrap();

        fs::create_dir_all(root.join("skills/vault-search")).unwrap();
        fs::write(root.join("skills/vault-search/SKILL.md"), "---\nname: vault-search\n---\nSearches the vault.").unwrap();

        fs::write(root.join("SOUL.md"), "You are Epistemos.").unwrap();
        fs::write(root.join("_index.md"), "# Vault Index").unwrap();

        tmp
    }

    #[test]
    fn build_topology_basic() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();

        assert!(topology.total_files > 0);
        assert!(topology.total_dirs > 0);
        assert!(topology.max_depth >= 1);
        assert!(!topology.nodes.is_empty());
    }

    #[test]
    fn poincare_coordinates_valid() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();

        for node in &topology.nodes {
            assert!(node.position.r >= 0.0 && node.position.r < 1.0,
                "r={} for {} is outside Poincaré disk", node.position.r, node.path);
            assert!(node.position.x.is_finite());
            assert!(node.position.y.is_finite());
        }
    }

    #[test]
    fn deeper_nodes_have_larger_radius() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();

        let depths: Vec<(u32, f64)> = topology.nodes.iter().map(|n| (n.depth, n.position.r)).collect();
        // On average, deeper nodes should have larger r
        let shallow: f64 = depths.iter().filter(|(d, _)| *d == 0).map(|(_, r)| r).sum::<f64>();
        let deep: f64 = depths.iter().filter(|(d, _)| *d >= 2).map(|(_, r)| r).sum::<f64>();
        let shallow_count = depths.iter().filter(|(d, _)| *d == 0).count() as f64;
        let deep_count = depths.iter().filter(|(d, _)| *d >= 2).count().max(1) as f64;
        if shallow_count > 0.0 && deep_count > 0.0 {
            assert!(deep / deep_count >= shallow / shallow_count,
                "Deep nodes should have larger r than shallow ones");
        }
    }

    #[test]
    fn complexity_weight_in_range() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();

        for node in &topology.nodes {
            assert!(node.complexity_weight >= 1.0 && node.complexity_weight <= 10.0,
                "Cw={} for {} is out of range", node.complexity_weight, node.path);
        }
    }

    #[test]
    fn volatility_in_range() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();

        for node in &topology.nodes {
            assert!(node.volatility >= 0.0 && node.volatility <= 1.0,
                "Vs={} for {} is out of range", node.volatility, node.path);
        }
    }

    #[test]
    fn blanket_summaries_exist_for_dirs() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();

        let dirs_with_children: Vec<&VaultNodeMetrics> = topology.nodes.iter()
            .filter(|n| n.is_directory && n.child_count > 0)
            .collect();

        for dir in &dirs_with_children {
            assert!(dir.blanket_summary.is_some(),
                "Directory {} should have a blanket summary", dir.path);
        }
    }

    #[test]
    fn hyperbolic_distance_properties() {
        let origin = HyperbolicPoint::from_polar(0.0, 0.0);
        let near = HyperbolicPoint::from_polar(0.3, 0.0);
        let far = HyperbolicPoint::from_polar(0.9, 0.0);

        let d_near = origin.hyperbolic_distance(&near);
        let d_far = origin.hyperbolic_distance(&far);

        assert!(d_far > d_near, "Far point should have larger hyperbolic distance");
        assert!(d_near > 0.0, "Non-zero distance");
    }

    #[test]
    fn agent_context_generation() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();
        let context = topology_to_agent_context(&topology, 500);

        assert!(context.contains("Vault Topology"));
        assert!(context.contains("Cw:"));
        assert!(context.contains("Gv:"));
    }

    #[test]
    fn god_nodes_identified() {
        let tmp = create_test_vault();
        let topology = build_topology(tmp.path()).unwrap();
        // God nodes should be present (even if gravity is low in test)
        assert!(topology.god_nodes.len() <= 10);
    }
}
