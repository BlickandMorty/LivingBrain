//! # LivingBrain
//!
//! A memory system that forgets, learns, and evolves — like a biological brain.
//!
//! LivingBrain provides six core systems for building AI agents and knowledge
//! managers with human-like memory:
//!
//! - **Decay** — Ebbinghaus forgetting curve with CMS-X conceptual inertia
//! - **Cache** — 5-layer tiered retrieval from <1ms to <50ms
//! - **Contradictions** — Detect conflicting facts instead of silent overwrite
//! - **Diff** — Text + JSON diff engine with fuzzy patching
//! - **Metrics** — TRACED reasoning trajectory quality measurement
//! - **Evolution** — GEPA skill extraction from successful agent traces

pub mod decay;
pub mod diff;
pub mod metrics;
pub mod classifier;
pub mod contradictions;
