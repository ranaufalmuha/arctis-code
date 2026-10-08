# Perf

Measure, don't assume (PRD V1 §7):

- `cargo run --release -- <repo>` → time to first frame (target < 800ms on M-series)
- Project open (100-file repo) → tree interactive (target < 300ms, no full scan)
- Idle RSS via Activity Monitor / `ps -o rss` (target < 150MB idle)
- Med-repo (5k files) RSS (target < 300MB, lazy tree)
- File open latency: `tests/perf.rs` asserts < 100ms for 50KB file

`cargo test --release` runs the smoke perf test.
