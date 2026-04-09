//! Basic LivingBrain usage — create facts, decay them, detect contradictions.
//!
//! Run: cargo run --example basic_memory

use chrono::{Duration, Utc};
use livingbrain::decay::{self, Importance, NodeStrength};
use livingbrain::classifier;
use livingbrain::contradictions;
use livingbrain::metrics;

fn main() {
    println!("=== LivingBrain Basic Example ===\n");

    // --- 1. Create facts with different importance levels ---
    let mut api_key = NodeStrength::new(Importance::Critical, 1.0, Utc::now());
    let mut project_note = NodeStrength::new(Importance::High, 1.0, Utc::now());
    let mut meeting_note = NodeStrength::new(Importance::Normal, 1.0, Utc::now());
    let mut draft = NodeStrength::new(Importance::Low, 1.0, Utc::now());

    // Simulate 30 days passing
    let future = Utc::now() + Duration::days(30);
    decay::decay(&mut api_key, future);
    decay::decay(&mut project_note, future);
    decay::decay(&mut meeting_note, future);
    decay::decay(&mut draft, future);

    println!("After 30 days:");
    println!("  Critical (API key):    {:.3}", api_key.strength);
    println!("  High (project note):   {:.3}", project_note.strength);
    println!("  Normal (meeting note): {:.3}", meeting_note.strength);
    println!("  Low (draft):           {:.3}", draft.strength);

    // --- 2. Conceptual inertia — frequently accessed facts resist decay ---
    let mut rarely_used = NodeStrength::new(Importance::Normal, 1.0, Utc::now());
    rarely_used.access_count = 1;

    let mut heavily_used = NodeStrength::new(Importance::Normal, 1.0, Utc::now());
    heavily_used.access_count = 100;

    let future = Utc::now() + Duration::days(30);
    decay::decay(&mut rarely_used, future);
    decay::decay(&mut heavily_used, future);

    println!("\nConceptual inertia (same importance, different access count):");
    println!("  Rarely used (1 access):    {:.3}", rarely_used.strength);
    println!("  Heavily used (100 access): {:.3}", heavily_used.strength);

    // --- 3. Contradiction detection ---
    println!("\nContradiction detection:");

    let c1 = contradictions::detect(
        "Claude Sonnet input pricing is $3.00 per million tokens",
        "Claude Sonnet input pricing is $5.00 per million tokens",
    );
    println!("  Pricing change: {:?}", c1.as_ref().map(|c| &c.conflict_type));

    let c2 = contradictions::detect(
        "Feature flag is enabled",
        "Feature flag is disabled",
    );
    println!("  Boolean flip:   {:?}", c2.as_ref().map(|c| &c.conflict_type));

    let c3 = contradictions::detect(
        "The sky is blue",
        "Rust is a programming language",
    );
    println!("  Unrelated:      {:?}", c3); // None

    // --- 4. Memory classification ---
    println!("\nMemory classification:");

    let existing = vec![
        "Claude Sonnet costs $3 per million input tokens",
        "MLX supports Apple Silicon natively",
    ];

    let (op1, _) = classifier::classify("Claude Sonnet costs $5 per million input tokens", &existing, 0.70);
    println!("  Price update: {:?}", op1); // Update

    let (op2, _) = classifier::classify("Rust is memory safe", &existing, 0.85);
    println!("  New fact:     {:?}", op2); // Add

    let (op3, _) = classifier::classify("MLX supports Apple Silicon natively", &existing, 0.85);
    println!("  Duplicate:    {:?}", op3); // Noop

    // --- 5. Reasoning metrics ---
    println!("\nReasoning trajectory metrics:");

    let efficient_calls = vec![
        ("search".into(), "MOHAWK".into(), "found training pipeline docs".into(), false),
        ("read".into(), "README.md".into(), "full content with 15 categories".into(), false),
        ("read".into(), "eval.jsonl".into(), "92% accuracy on benchmarks".into(), false),
    ];
    let m = metrics::compute(&efficient_calls);
    println!("  Efficient agent: {:?} (curvature: {:.2})", m.classification, m.curvature_ratio);

    let looping_calls: Vec<_> = (0..5)
        .map(|_| ("search".into(), "same query".into(), "same result".into(), false))
        .collect();
    let m2 = metrics::compute(&looping_calls);
    println!("  Looping agent:   {:?} (loops: {})", m2.classification, m2.loop_count);

    println!("\n=== Done ===");
}
