# Iterations 16-18: Build-Level Optimizations

## Iteration 16: Scaling Factor Precompute + Sparse TRM Multiply
**Attempted, Reverted** — Both optimizations were slower than baseline:
- Lazy caching of scaling_factor_x/y: branch overhead > sqrt savings
- Sparse TRM multiply (exploiting params zeros): compiler already optimizes this

## Iteration 17: PGO Build Script
Created `scripts/build-pgo.sh` for reproducible PGO builds.

## Iteration 18: target-cpu=native + PGO

| Build | Wall | User | vs Baseline |
|-------|------|------|-------------|
| Baseline (thin LTO) | 6.112s | 5.947s | — |
| target-cpu=native | 5.900s | 5.774s | -3.5% |
| PGO only | 5.615s | 5.485s | -8.1% |
| PGO + native | 5.836s | 5.642s | -4.5% (noisy) |

**Conclusion:** target-cpu=native provides marginal benefit on Apple Silicon
(already using good defaults). PGO alone gives the best ROI.

## Full 20K Benchmark (PGO build, multi-thread)
- **20,219 files in 3:20 (200.57s wall)**
- 616% CPU (6-core utilization)
- **9.9ms/file** (multi-thread wall time)
- **59.5ms/file** (single-thread user time equivalent)

## Dead-End Analysis
Optimizations attempted but not yielding measurable improvement:
- `panic = "abort"`: No measurable change
- `lto = "fat"` vs `lto = "thin"`: No measurable change
- Sparse matrix multiply: Slower (compiler auto-optimizes zeros)
- Scaling factor caching: Branch overhead outweighs sqrt savings
- SmallVec in content parser: Not applicable (lopdf Object requires Vec<u8>)
- to_unicode Cow<str>: Would require TextPosition refactoring (String → Cow)
