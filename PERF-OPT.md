# Performance Optimization Tracker

## Baseline
- **Corpus**: 20,219 Korean government procurement PDFs
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

## Results
- **#2 + #3 applied**: retain-based batch removal replaces O(n²) Vec::remove loops
- **Net performance**: ~0% change (duplicate suppression rarely triggers on Korean PDFs)
- **Quality**: 198/200 byte-identical with v4 (2 diffs = filename collision, not code change)

## Quality Gate
- avg char similarity vs Java PDFBox >= 99.3% ✓ (99.36%)
- >= 98% files with >= 99% similarity ✓ (98.5%)
- 20,198 success / 21 fail (no regression) ✓

## Conclusion
The v4 baseline (custom content parser + mimalloc + font cache) already captured most
extractable performance gains. The remaining bottleneck is lopdf's document loading/parsing,
which accounts for the majority of CPU time. Further gains require either:
- Replacing lopdf with a faster PDF parser
- Reducing per-document overhead (e.g., lazy object resolution)
- Profile-guided optimization (PGO)
