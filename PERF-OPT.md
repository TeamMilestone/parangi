# Performance Optimization Tracker

## Baseline
- **Corpus**: 20,219 Korean government procurement PDFs
- **PDF location**: `/Users/wonsup-mini/projects/superboard/data/raw/files/`
- **File list**: `find /Users/wonsup-mini/projects/superboard/data/raw/files -name '*.pdf' | sort > /tmp/parang-bench-full.txt`
- **Machine**: Mac (multi-core, 584% CPU utilization)
- **Baseline (v4)**: 3:49 wall / 1,296s CPU / 20,198 success / 21 fail

## Optimizations

| # | Optimization | Status | Wall | CPU | Delta |
|---|-------------|--------|------|-----|-------|
| 0 | Baseline (fast parser + mimalloc + font cache) | done | 3:49 | 1296s | — |
| 1 | CMap range: linear scan → binary search | skipped | — | — | semantics issue: range order matters for overlapping CID ranges |
| 2 | Duplicate suppression: O(n²) → retain | done | 3:53 | 1320s | ~0% (within noise) |
| 3 | Contained spaces: O(n²) → retain | done | (same run) | (same) | ~0% (within noise) |
| 4 | TRM per-glyph → incremental | skipped | — | — | correctness issue: params×CTM scaling factors ≠ params×Tm×CTM |
| 5 | Font cache Mutex → RwLock | skipped | 4:05 | 1335s | -7% (worse, RwLock overhead > contention savings) |
| 6 | PGO (Profile-Guided Optimization) | done | 3:39 | 1253s | **+4.4%** (wall), **+3.3%** (CPU) |

## Alternative Parser Evaluation

| Parser | 100-file time | Avg char similarity | Verdict |
|--------|--------------|-------------------|---------|
| parang (ours) | 1.32s | 99.36% | production |
| pdf_oxide 0.3 | 0.96s | 90.58% | quality gap too large |

## PGO (Profile-Guided Optimization)

### Steps to reproduce
```bash
# 1. Instrumented build
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release

# 2. Collect profiles on representative workload (~100 PDFs)
target/release/parang -L /tmp/parang-bench-100.txt -o /tmp/pgo-train-out

# 3. Merge profiles
xcrun llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data/*.profraw

# 4. Optimized build
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" cargo build --release
```

### Results
- **100-file benchmark**: 0.96s (baseline 1.32s, **27% faster**)
- **Full 20K benchmark**: 3:39 wall / 1,253s CPU / 593% CPU (baseline 3:49 / 1,296s / 584%)
- **Quality**: 198/200 byte-identical with v4, 20,198 success / 21 fail (no regression)

## Results Summary
- **#2 + #3 applied**: retain-based batch removal replaces O(n²) Vec::remove loops (~0% change)
- **#6 PGO**: 4.4% wall-time improvement, 3.3% CPU reduction
- **Quality**: 198/200 byte-identical with v4 (2 diffs = filename collision, not code change)

## Performance History

| Version | Wall time | Improvement vs v1 |
|---------|-----------|-------------------|
| v1 (xargs batches) | 21:23 | baseline |
| v2 (--file-list) | 7:10 | 3.0x |
| v3 (mimalloc + font cache) | 6:24 | 3.3x |
| v4 (fast content parser) | 3:49 | 5.6x |
| **v5 (PGO)** | **3:39** | **5.9x** |

## Quality Gate
- avg char similarity vs Java PDFBox >= 99.3% ✓ (99.36%)
- >= 98% files with >= 99% similarity ✓ (98.5%)
- 20,198 success / 21 fail (no regression) ✓

## Conclusion
PGO provided a modest but measurable 4.4% wall-time improvement. The remaining bottleneck
is lopdf's document loading/parsing. Alternative parsers (pdf_oxide) are faster but have
significant quality gaps on Korean PDFs. Further gains would require:
- Replacing/optimizing lopdf internals (lazy object resolution, faster xref parsing)
- BOLT (post-link optimization) on top of PGO
- Workload-specific optimizations (e.g., skip non-text pages early)
