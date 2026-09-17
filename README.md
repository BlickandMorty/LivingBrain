# LivingBrain

A Rust experiment in software memory: how stored information changes with time, access, and conflicting text.

**Status:** exploratory, substantially AI-assisted implementation. I chose the project direction, but I am currently learning Python and do not yet understand this Rust code well enough to claim independent authorship or Rust proficiency. The documentation is also AI-assisted.

The name is a metaphor. This repository does not recreate a biological brain, measure consciousness, or establish that its rules describe human memory.

## Start here

Read [the decay model](src/decay.rs), then [the basic example](examples/basic_memory.rs). These are the smallest entry points into the question that interests me: what happens when information fades unless it is reinforced?

For biological teaching simulations, see the separate [Neuroscience Modeling Workbench](https://github.com/BlickandMorty/neuron-modeling-workbench).

## What the code contains

| Module | Implemented scope | Limitation |
| --- | --- | --- |
| [Decay](src/decay.rs) | Exponential strength decay, access-based rate adjustment, reinforcement, pinning, and threshold-based removal. | Rates and rules are software choices, not parameters fitted to human observations. |
| [Cache](src/cache.rs) | An in-memory hot cache with text matching, eviction, and time-window retrieval. | Warm/cold layer labels do not establish a complete multi-tier retrieval backend. |
| [Contradictions](src/contradictions.rs) | Text rules for selected numeric, Boolean, and antonym conflicts. | Heuristics can miss conflicts or flag unrelated statements; they do not establish factual truth. |
| [Classifier](src/classifier.rs) | Heuristics for proposing memory operations from text comparisons. | Suggested operations are not evidence of semantic understanding. |
| [Diff](src/diff.rs) | Text differences and patch application. | A software utility, not a cognitive mechanism. |
| [Metrics](src/metrics.rs) | Tool-call repetition, error counts, and text-overlap trajectory summaries. | Labels such as `Efficient` are heuristic categories, not validated measures of reasoning quality. |
| [Topology](src/topology.rs) | File-hierarchy coordinates and descriptive tags. | A representation of a directory tree, not a demonstrated neural or cognitive architecture. |

The earlier README described skill evolution, a git-backed vault module, extra examples, and benchmark artifacts that are not present in this repository. Those descriptions and unsupported performance comparisons have been removed. Historical text remains in Git history; this README describes the current source.

## One model, made explicit

For elapsed time in days, the decay code uses:

```text
effective_rate = base_rate / (1 + ln(max(access_count, 1)))
new_strength = old_strength * exp(-effective_rate * elapsed_days)
```

An access resets strength to 1 and increments the access count. Pinned entries are exempt from decay. The implementation updates its time reference after applying decay.

**Question:** under these rules, does greater prior access preserve more strength over an equal time interval?

**Method:** compare entries with the same starting strength and base rate while varying access count.

**Expected result from the equation:** higher access counts reduce the effective decay rate. The source includes a unit test for this comparison.

**Boundary:** this consequence follows from the chosen rule. Agreement with that rule does not validate a theory of human forgetting, rehearsal, or learning. No human-memory dataset is supplied here.

## Run the existing Rust experiment

From a Rust development environment:

```sh
git clone https://github.com/BlickandMorty/LivingBrain.git
cd LivingBrain
cargo test --no-default-features
cargo run --no-default-features --example basic_memory
```

These commands use the local repository rather than assuming a published crate. `--no-default-features` avoids the optional Tantivy dependency for this small example. See [Cargo.toml](Cargo.toml) for dependencies and declared features. Test success checks software behavior; it does not establish biological accuracy or benchmark superiority.

## Learning direction

I want to learn the small decay-and-rehearsal idea in Python before attempting a larger system. That would begin with one equation, one plot, predictions in my own words, and a comparison between assumptions. It is a proposed exercise; a Python implementation is not included here.

## Provenance and licensing

This experiment originated in the AI-assisted Epistemos project, now kept private. The repository includes [LICENSE-MIT](LICENSE-MIT). The package manifest declares `MIT OR Apache-2.0`, but an Apache license file is not included; this documentation update does not change the existing license declaration.
