# LivingBrain

**A memory system that forgets, learns, and evolves — like a biological brain.**

LivingBrain is a Rust library for building AI agents and knowledge systems with human-like memory. Instead of treating memory as a flat database, LivingBrain models memory as a living system where facts decay over time, frequently-used knowledge strengthens, contradictions are surfaced (not silently overwritten), and the system learns reusable skills from experience.

Built for agent frameworks, personal knowledge managers, and anyone who needs memory that's smarter than a vector database.

```
cargo add livingbrain
```

---

## Why This Exists

Every AI agent framework has the same memory problem: stuff goes in, stuff comes out, but nothing *ages*, nothing *fades*, nothing *conflicts*, and nothing *improves*. Your agent remembers a fact from 6 months ago with the same confidence as something the user said 5 minutes ago. It silently overwrites old facts with new ones without telling you they contradict. And it never learns from its own successes.

LivingBrain fixes this with six systems that work together:

| System | What It Does | Why It Matters |
|--------|-------------|----------------|
| **Ebbinghaus Decay** | Facts lose strength over time unless reinforced | Stale knowledge fades naturally — no manual cleanup |
| **Conceptual Inertia** | Frequently-accessed facts resist displacement | Important knowledge stays stable even under decay |
| **Contradiction Detection** | Surfaces conflicting facts instead of silent overwrite | Users see "old fact vs new fact" and decide |
| **Tiered Retrieval** | 5-layer cache from <1ms hot to <50ms cold | Right speed for every query type |
| **Skill Evolution** | Extracts reusable procedures from successful agent runs | The system gets better every time it's used |
| **Reasoning Metrics** | Measures agent trajectory quality (displacement, curvature, loops) | Know if your agent is thinking efficiently or going in circles |

---

## Quick Start

### Memory with Decay

```rust
use livingbrain::decay::{NodeStrength, Importance, decay, access, batch_decay};
use chrono::Utc;

// Create a fact with Normal importance
let mut fact = NodeStrength::new(Importance::Normal, 1.0, Utc::now());

// Simulate 30 days passing
let future = Utc::now() + chrono::Duration::days(30);
decay(&mut fact, future);
println!("Strength after 30 days: {:.3}", fact.strength);
// → ~0.223 (Normal decay rate: λ=0.05/day)

// Access the fact — strength resets to 1.0, access count increases
access(&mut fact);
println!("After access: strength={}, count={}", fact.strength, fact.access_count);
// → strength=1.0, count=1

// With CMS-X conceptual inertia: after 50 accesses, decay slows ~4x
fact.access_count = 50;
let effective_rate = fact.effective_decay_rate();
println!("Effective decay rate: {:.4} (base: {:.4})", effective_rate, fact.decay_rate);
// → 0.0119 vs 0.05 — frequently-used facts resist forgetting
```

### Importance Levels

| Level | Decay Rate (λ) | Half-Life | Use Case |
|-------|----------------|-----------|----------|
| Critical | 0.005/day | ~139 days | API keys, identity facts, safety rules |
| High | 0.01/day | ~69 days | Active projects, key decisions |
| Normal | 0.05/day | ~14 days | Regular notes, meeting summaries |
| Low | 0.1/day | ~7 days | Drafts, temporary thoughts |

Facts decay exponentially: `strength(t) = s₀ × e^(-λ_effective × days)`. When strength drops below 0.15, the fact is eligible for garbage collection.

### Contradiction Detection

```rust
use livingbrain::contradictions::{detect_contradiction, ConflictType};

let old_fact = "Claude Sonnet 4.6 input pricing is $3.00 per million tokens";
let new_fact = "Claude Sonnet 4.6 input pricing is $5.00 per million tokens";

let conflict = detect_contradiction(old_fact, new_fact);
// → Some(ConflictType::Numeric) — detects the $3.00 vs $5.00 discrepancy

// Instead of silently overwriting, surface both facts:
// "EXISTING (strength: 0.92): $3.00/MTok"
// "PROPOSED (new):            $5.00/MTok"
// → User decides which is correct
```

Conflict types: `Numeric`, `Boolean`, `Antonym`, `SemanticReversal`. Each comes with a confidence score.

### Tiered Retrieval (Neural Cache)

```rust
use livingbrain::cache::{NeuralCache, CacheLayer};

let mut cache = NeuralCache::new(vault_backend);

// Hot layer (<1ms): keyword match on top-K cached facts
let results = cache.instant_retrieve("MOHAWK training pipeline", 5).await;

// Temporal query: "what did I learn in the last hour?"
let recent = cache.temporal_retrieve(
    chrono::Utc::now() - chrono::Duration::hours(1),
    chrono::Utc::now(),
    10,
).await;

// Facts automatically warm up from Cold → Warm → Hot based on access
```

| Layer | Latency | Mechanism | Capacity |
|-------|---------|-----------|----------|
| L0 (Context) | 0ms | Current conversation window | ~128K tokens |
| L1 (Hot) | <1ms | In-memory keyword index + LRU | 1,000 facts |
| L2 (Warm) | <5ms | Tantivy FTS + SQLite vec0 | 100K facts |
| L3 (Cold) | <50ms | Filesystem scan | Unlimited |

### Reasoning Trajectory Metrics (TRACED)

```rust
use livingbrain::metrics::{compute_trajectory_metrics, TrajectoryClassification};

// After an agent session, evaluate the tool call sequence
let tool_calls = vec![
    ("vault_search".into(), "MOHAWK".into(), "found 8 files".into(), false),
    ("vault_read".into(), "MOHAWK/README.md".into(), "training pipeline docs".into(), false),
    ("vault_read".into(), "MOHAWK/eval.jsonl".into(), "92% accuracy".into(), false),
];

let metrics = compute_trajectory_metrics(&tool_calls);
println!("Classification: {:?}", metrics.classification);
// → Efficient (high displacement, low curvature, no loops)

println!("Displacement: {:.2}", metrics.displacement);    // semantic progress
println!("Curvature: {:.2}", metrics.curvature_ratio);     // path efficiency
println!("Loops: {}", metrics.loop_count);                  // repeated calls
```

| Classification | Meaning | Curvature | Loops |
|---------------|---------|-----------|-------|
| Efficient | Direct path to goal | < 2.0 | 0 |
| Exploratory | Broad search, making progress | 2.0–4.0 | 0–2 |
| Hesitating | Going in circles | > 4.0 | 3+ |
| Stuck | No progress despite effort | any | any (displacement < 0.1) |
| Failed | Mostly errors | any | any (errors > 50%) |

Inspired by [TRACED](https://arxiv.org/abs/2603.10384) — geometric metrics for reasoning evaluation.

### Skill Evolution (GEPA)

```rust
use livingbrain::evolution::{analyze_traces, propose_mutation, ImprovementSignal};

// Analyze agent execution traces for improvement patterns
let signals = analyze_traces(&session_traces);
// → [FrequentRetries("vault_search", 4), SlowExecution("web_fetch", 8.2s)]

// Propose a skill mutation based on the signals
if let Some(mutation) = propose_mutation(&current_skill, &signals) {
    println!("Proposed: {}", mutation.rationale);
    // → "Add retry logic with exponential backoff for vault_search calls"

    // Constraint gates prevent bad mutations:
    // - Size gate: skill must stay under 15KB
    // - Semantic preservation: cosine similarity > 0.80 vs original
    if mutation.passes_constraints() {
        apply_mutation(&mut skill, &mutation);
    }
}
```

Improvement signals detected:
- **FrequentRetries** — tool called 3+ times in a row (agent struggling)
- **SlowExecution** — consistently >5 seconds (bottleneck)
- **ConsistentFailure** — same error across 3+ sessions (systematic bug)
- **UnusedCapability** — skill defines a tool but never invokes it (dead code)

### Diff Engine

```rust
use livingbrain::diff::{generate_text_diff, apply_text_patch};

let old = "Claude input: $3.00/MTok\nClaude output: $15.00/MTok";
let new = "Claude input: $5.00/MTok\nClaude output: $15.00/MTok";

let diff = generate_text_diff(old, new);
// → 1 hunk: line 1 changed, line 2 unchanged

// Apply the patch with fuzzy matching (tolerates ±3 lines of drift)
let patched = apply_text_patch(old, &diff.hunks)?;
assert_eq!(patched, new);
```

Also supports JSON diffs with JSON Pointer paths for structured data.

### Hyperbolic Vault Topology

```rust
use livingbrain::topology::{scan_vault, should_pierce_blanket, topology_to_agent_context};

// Scan vault into hyperbolic (Poincare disk) coordinates
let topology = scan_vault("/path/to/vault");

// Each node gets 3 dimensional metrics:
// - Complexity Weight (Cw): token count + structural density (1.0–10.0)
// - Gravity (Gv): how many other facts reference this
// - Volatility (Vs): edit recency with exponential decay

// God Nodes: top 10 by gravity — the hub documents everything connects to
println!("God nodes: {:?}", topology.god_nodes);

// Markov Blanket piercing: should the agent explore inside this directory?
let (should_explore, confidence) = should_pierce_blanket("MOHAWK training", &directory_node);
// → (true, 0.73) — high relevance to query

// Generate compact spatial map for agent context injection
let context = topology_to_agent_context(&topology, 500); // 500 token budget
```

The hyperbolic embedding places nodes on a Poincare disk where depth maps to radius: `r = tanh(depth × 0.3)`. Deeper nodes are closer to the boundary, naturally clustering related content.

---

## Architecture

```
livingbrain/
├── src/
│   ├── decay.rs          — Ebbinghaus decay + CMS-X conceptual inertia
│   ├── cache.rs          — 5-layer tiered retrieval (Neural Cache)
│   ├── contradictions.rs — Conflict detection (Numeric/Boolean/Antonym/SemanticReversal)
│   ├── diff.rs           — Text + JSON diff engine with fuzzy patching
│   ├── classifier.rs     — Memory operation classification (Add/Update/Delete/Noop)
│   ├── metrics.rs        — TRACED reasoning trajectory metrics
│   ├── topology.rs       — Hyperbolic vault topology + Markov Blanket piercing
│   ├── evolution.rs      — GEPA skill evolution from execution traces
│   ├── vault_git.rs      — Git-backed vault mutations with structured commits
│   └── lib.rs
├── examples/
│   ├── basic_memory.rs
│   ├── agent_with_decay.rs
│   └── skill_evolution.rs
├── benchmarks/
│   └── retrieval_bench.rs
└── docs/
    ├── ARCHITECTURE.md
    ├── DECAY_MATH.md
    └── TRACED_METRICS.md
```

---

## Use Cases

### 1. Agent Memory That Actually Works

Every agent framework (LangChain, CrewAI, AutoGen, Hermes) stores conversation history but doesn't manage it intelligently. LivingBrain slots in as the memory backend:

```rust
// At session start: load identity + facts + relevant episodes
let soul = vault.read("SOUL.md");           // L4: never pruned
let facts = vault.read("knowledge.md");     // L3: always injected
let skills = skill_router.route(objective); // L2: relevance-ranked
let episodes = cache.temporal_retrieve(...); // L1: semantic match

// During session: facts decay, new facts classified, contradictions surfaced
// After session: skill evolution analyzes trace, distills reusable patterns
```

### 2. Personal Knowledge Management

Replace flat note-taking with a vault that understands itself:
- Notes you haven't opened in 3 months naturally fade from search results
- Notes you reference daily stay prominent regardless of age
- When you write something that contradicts an older note, you see both
- Your vault's structure is navigable as a hyperbolic graph with god nodes

### 3. Research Assistants

Build research agents that measure their own quality:
- TRACED metrics tell you if the agent is making progress or going in circles
- Skill evolution captures "how I successfully researched topic X" as a reusable procedure
- Tiered cache ensures recent research is instantly available, old research is still findable

### 4. Enterprise Knowledge Bases

For teams that need auditable, version-controlled knowledge:
- Every mutation is a git commit with structured messages
- Contradictions are tracked, not silently resolved
- Decay rates can be set per-document (legal docs = Critical, meeting notes = Low)
- Cross-file propagation ensures related documents stay consistent

---

## Benchmarks

Measured on Apple M2 Pro, 18GB unified memory:

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Hot cache lookup (L1) | 0.3ms | 3,300 queries/sec |
| Warm search (L2, Tantivy) | 2.1ms | 476 queries/sec |
| Cold filesystem scan (L3) | 31ms | 32 queries/sec |
| Decay pass (10K nodes) | 0.8ms | 12.5M nodes/sec |
| Contradiction detection | 0.1ms | 10,000 pairs/sec |
| Diff generation (1KB) | 0.05ms | 20,000 diffs/sec |
| Trajectory metrics (20 calls) | 0.02ms | 50,000 sessions/sec |

---

## Research Foundations

LivingBrain is grounded in published research:

- **Ebbinghaus Forgetting Curve** (1885) — exponential memory decay: `s(t) = s₀ × e^(-λt)`
- **CMS-X Conceptual Inertia** — frequently-accessed facts resist displacement via logarithmic damping
- **TRACED** (arXiv:2603.10384, 2026) — geometric reasoning metrics using displacement and curvature
- **BeliefShift** (arXiv:2603.23848, 2026) — no model achieves both drift resistance and evidence sensitivity
- **Poincare Disk Embeddings** — hyperbolic geometry for hierarchical structure (Nickel & Kiela, 2017)
- **Free Energy Principle** (Friston, 2010) — Markov Blanket boundaries for information-theoretic navigation
- **Voyager** (Wang et al., 2023) — skill library with hash-based dedup for LLM agents

---

## Comparison with Alternatives

| Feature | LivingBrain | LangChain Memory | Mem0 | MemGPT/Letta | ChromaDB |
|---------|------------|-----------------|------|-------------|----------|
| Time-based decay | **Yes** (Ebbinghaus + inertia) | No | No | Partial (archival) | No |
| Contradiction detection | **Yes** (4 types) | No | No | No | No |
| Skill evolution | **Yes** (GEPA) | No | No | No | No |
| Reasoning metrics | **Yes** (TRACED) | No | No | No | No |
| Tiered retrieval | **Yes** (5 layers) | Single layer | Single layer | 2 layers | Single layer |
| Hyperbolic topology | **Yes** (Poincare disk) | No | No | No | No |
| Git-backed mutations | **Yes** | No | No | No | No |
| Cross-file propagation | **Yes** | No | No | No | No |
| Pure Rust (no Python) | **Yes** | No (Python) | No (Python) | No (Python) | No (Python) |
| Sub-millisecond hot cache | **Yes** (0.3ms) | No | No | No | ~5ms |

---

## Installation

```toml
[dependencies]
livingbrain = "0.1"
```

Requires Rust 1.75+ (for async traits). Optional features:
- `tantivy` — enables Warm layer full-text search (on by default)
- `git` — enables git-backed vault mutations (requires `git2`)
- `metal` — enables Metal GPU acceleration for TurboQuant (macOS only)

---

## License

MIT OR Apache-2.0 (dual-licensed)

---

## Contributing

LivingBrain is extracted from [Epistemos](https://github.com/BlickandMorty/Epistemos), a native macOS cognitive operating system. Contributions welcome — see [CONTRIBUTING.md](docs/CONTRIBUTING.md).

**Areas where help is needed:**
- Python bindings (PyO3) for LangChain/CrewAI integration
- Wasm target for browser-based agents
- Benchmarks on Linux/Windows
- Additional conflict detection heuristics
- Visualization tools for decay curves and trajectory metrics
