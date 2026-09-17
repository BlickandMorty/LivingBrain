//! # LivingBrain
//!
//! Experimental software-memory utilities with substantial AI assistance.
//!
//! Modules cover decay, an in-memory cache, text-conflict heuristics,
//! memory-operation classification, text differences, tool-call summaries,
//! and file-hierarchy coordinates. These mechanisms are not validated models
//! of human memory, reasoning, or consciousness. See the README for scope.

pub mod cache;
pub mod classifier;
pub mod contradictions;
pub mod decay;
pub mod diff;
pub mod metrics;
pub mod topology;
