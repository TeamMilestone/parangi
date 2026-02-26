# Iteration 14: Profile-Guided Optimization (PGO)

## Problem
Profile is very flat after iterations 7-12. No single hotspot above 2%.
Build-level optimizations can improve branch prediction and code layout.

## Solution
PGO build process:
1. Build instrumented binary: `RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release`
2. Run training workload: `target/release/parang -L /tmp/parang-bench-100.txt` (×3)
3. Merge profiles: `llvm-profdata merge -o /tmp/pgo-merged.profdata /tmp/pgo-data/`
4. Build optimized: `RUSTFLAGS="-Cprofile-use=/tmp/pgo-merged.profdata" cargo build --release`

## Results

**Benchmark: 100 files, RAYON_NUM_THREADS=1**

| Build | Wall (mean) | User | Improvement |
|-------|-------------|------|-------------|
| Baseline (thin LTO) | 6.112s | 5.947s | — |
| PGO + thin LTO | 5.640s | 5.509s | **-7.7% wall, -7.4% user** |

Output: 100% byte-identical (0 diffs across 96 output files).

## Notes
- `lto = "fat"` showed no benefit over `lto = "thin"` (6.138s vs 6.112s)
- `panic = "abort"` showed no measurable benefit (6.365s, within noise)
- PGO is a build process change, not a code change. No commit needed.
- PGO profiles are workload-specific — should be regenerated for production.

## Corrected Baseline
Previous sessions' 1.3s baseline was incorrect: xargs split space-containing filenames,
processing only ~25 of 100 files. True baseline with `-L` flag: 6.112s.
