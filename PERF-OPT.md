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

## Conclusion (pre-lopdf)
PGO provided a modest but measurable 4.4% wall-time improvement. The remaining bottleneck
is lopdf's document loading/parsing. Alternative parsers (pdf_oxide) are faster but have
significant quality gaps on Korean PDFs. Further gains would require:
- Replacing/optimizing lopdf internals (lazy object resolution, faster xref parsing)
- BOLT (post-link optimization) on top of PGO
- Workload-specific optimizations (e.g., skip non-text pages early)

---

## lopdf Optimization Session (2026-02-26)

### Benchmark Setup
- **Machine**: AMD Ryzen 5 6600H (12 cores, 1073% CPU util)
- **Test corpus**: 200 Korean government PDFs from /home/deploy/projects/superboard_raw_pdf/
- **Profiling**: PARANG_PROFILE=1 (CPU-time per phase) + LOPDF_PROFILE=1 (lopdf sub-phases)

### Baseline (before lopdf work, after PGO)
- Wall clock (200 files): 2.421s = 82.6 files/sec
- doc_open (lopdf parse): 17,132ms CPU (67.7% of total)
- parallel_parse: 21,912ms CPU (99.7% of lopdf time)

### Lopdf Optimization Summary

| Iteration | Change | parallel_parse | Wall (200 files) | Notes |
|-----------|--------|----------------|-----------------|-------|
| 0 | Baseline | 21,912ms | 2.421s | - |
| 1 | BTreeMap → HashMap (document.objects, xref.entries) | 18,290ms | - | 16.5% improvement |
| 2 | Lazy stream loading (skip non-ObjStm content, backing buffer) | 16,706ms | 2.421s | Fix zero_len bug |
| 3 | **LocatedSpan::take_from(0) optimization** | **731ms** | **0.916s** | **22.8x improvement!** |

### Iteration 3: Root Cause Discovery (CRITICAL)

**Root cause**: `nom_locate::LocatedSpan::take_from(offset)` inside `parser::indirect_object()` was
scanning ALL `offset` bytes for newline counting. Called once per xref entry (73,980 times across
200 files). With large PDFs (some 19MB), this created an O(N×files×avg_offset) scan = hundreds of GB.

**Fix**: In `lopdf/src/reader.rs`, `read_object()` now pre-slices the buffer:
```rust
// Before (O(offset) per call):
parser::indirect_object(ParserInput::new_extra(self.buffer, "indirect object"), offset, ...)

// After (O(1)):
let sliced = &self.buffer[offset..];
parser::indirect_object(ParserInput::new_extra(sliced, "indirect object"), 0, ...)
// then manually adjust stream.start_position += offset
```

### Final Profile (after all lopdf optimizations)
- Wall clock (200 files): **0.916s** (2.64x vs 2.421s baseline)
- doc_open: 1,015ms (was 17,132ms) — **17x improvement**
- parallel_parse: 731ms (was 21,912ms) — **30x improvement**
- New bottleneck: process_content 52.6%, content_bytes 23.3%

### Full Phase Breakdown (after)
```
doc_open (lopdf parse):      1,015ms  (11.1%)
content_bytes (decomp):      2,135ms  (23.3%)
load_resources (fonts):        285ms   (3.1%)
process_content (eng):       4,818ms  (52.6%)
assemble_text (strip):         913ms  (10.0%)
TOTAL:                        9,167ms
```

### Additional Changes Made
- **lopdf/src/xref.rs**: BTreeMap → HashMap for xref.entries
- **lopdf/src/document.rs**: BTreeMap → HashMap for document.objects; added backing_buffer field
- **lopdf/src/object.rs**: Added `raw_length: Option<usize>` to Stream; Stream::new_lazy(); decompressed_content_from()
- **lopdf/src/parser/mod.rs**: lazy stream creation in stream() function
- **lopdf/src/reader.rs**: load_from_arc(), profiling counters, zero_length_streams fix
- **crates/parang/src/document.rs**: should_skip_stream() (lazy all except ObjStm/XRef), open() via load_from_arc

---

## New Optimization Session (2026-02-26 afternoon)

### Benchmark machine
- AMD Ryzen 5 6600H (12 HT cores), Linux 6.17
- Test corpus: 200 Korean government PDFs (`/tmp/parang-bench-200.txt`)
- 100-PDF subset: `/tmp/parang-bench-100.txt`

### Starting point (iter04 + non-PGO)
| Phase | CPU time | % |
|-------|---------|---|
| doc_open | ~1499ms | 17.3% |
| content_bytes | ~2265ms | 26.1% |
| load_resources | ~342ms | 3.9% |
| process_content | ~3773ms | 43.6% |
| assemble_text | ~785ms | 9.1% |

- Wall clock 200 PDFs: ~0.86s (non-PGO), ~0.74s (PGO)
- Wall clock 100 PDFs: ~0.82s (non-PGO), ~0.54s (PGO)

### iter05: TextPosition struct size reduction
- **Change**: Removed 5 unused fields from TextPosition: `char_code`, `end_x`, `end_y`, `font_size_in_pt`, `page_rotation`
- **Struct size**: 104 → 84 bytes (19% smaller)
- **Dead code removed**: `end_y` computation per glyph in show_text, `font_size_in_pt` sqrt before loop
- **Result**: ~1% improvement (0.830 → 0.820s, within noise)
- **Files**: `text_position.rs`, `engine.rs`, test helpers in `stripper.rs`, `comparator.rs`

### iter06: PGO (Profile-Guided Optimization)
- **Change**: Used `cargo-pgo` (llvm-tools-preview) to collect runtime profiles on 200 PDF workload, then rebuild
- **Result**: 0.82s → 0.74s (100 PDFs), **10.4% speedup**
- **Profile wins**: process_content 5278ms → 3773ms (28.5% faster with PGO)
- **Script**: `scripts/build-pgo.sh`

### iter07: normalize_text NFC fast-path
- **Problem**: `normalize_text(&str) -> String` always allocated 2 Strings per call (one for ligature decomp, one for NFC). Called once per page.
- **Fix**: Changed to `normalize_text(String) -> String`. Added fast-path: scan for ligatures, then `is_nfc_quick()`. For Korean text (no ligatures, already NFC), returns String unchanged = zero allocation.
- **Result**: assemble_text CPU 785ms → 370ms (**53% faster**); wall clock 0.74 → 0.708s (200 PDFs, PGO)
- **Files**: `normalizer.rs` (signature + fast-path), `stripper.rs` (pass owned String)

### iter08: zlib-rs decompression backend
- **Change**: `flate2 = { version = "1.0", default-features = false, features = ["zlib-rs"] }` in `lopdf/Cargo.toml`
- **Previous attempt**: system zlib (`features = ["zlib"]`) was slower (C FFI overhead); zlib-rs is pure Rust with SIMD
- **Result**: content_bytes CPU 1679ms → 1337ms (**20% faster**); wall clock 0.708 → 0.633s (200 PDFs, PGO) = **10.6% speedup**
- **Files**: `lopdf/Cargo.toml`

### iter09: hashbrown HashMap across hot paths
- **Change**: Replaced `std::collections::HashMap` (SipHash) with `hashbrown::HashMap` (ahash) in all hot-path lookup tables:
  - `CidWidths::individual` (per-glyph width lookup in type0_font.rs)
  - `CMap::char_to_unicode` (per-glyph unicode lookup in cmap.rs)
  - `engine.rs` fonts HashMap, `lib.rs` FontCache
  - Also: `glyph_list.rs`, `encoding/mod.rs`, `cmap_manager.rs`, `stripper.rs`
- **Result**: wall clock 0.633 → 0.613s (200 PDFs, PGO) = **3.2% speedup**
- **Files**: Workspace `Cargo.toml` (add hashbrown), `crates/parang/Cargo.toml`, all above source files

### Final Profile (iter07+08+09 combined, PGO, 100 PDFs = 1929 pages)
| Phase | CPU time | % |
|-------|---------|---|
| doc_open | 519ms | 10.5% |
| content_bytes | 1241ms | 25.0% |
| load_resources | 141ms | 2.9% |
| process_content | 2688ms | 54.2% |
| assemble_text | 367ms | 7.4% |
| TOTAL | 4956ms | |

- **Wall clock (200 PDFs)**: ~0.613s (median of 10 runs)
- **Wall clock (100 PDFs)**: ~0.478s (median of 7 runs)
- **vs iter06+PGO baseline**: 200 PDFs 0.74→0.613 = **17.2% improvement**

### Rebuild PGO command
```bash
bash scripts/build-pgo.sh /tmp/parang-bench-200.txt
```

### Attempted but reverted
- `read_code` Identity-H fast path (`#[inline]` + skip codespace matching): actually caused 7.7% regression — PGO already optimizes this path well and `#[inline]` hurt instruction cache layout

---

### iter10: TextPosition Matrix → 4×f32 (88→64 bytes, 1 cache line)
- **Change**: Replaced `text_matrix: Matrix` (36 bytes) with `trm_a, trm_b, trm_tx, trm_ty: f32` (16 bytes)
- **Struct size**: 88 → 64 bytes (exactly 1 cache line — no cross-line access)
- **Engine bonus**: Eliminated per-glyph Matrix copy (`let mut trm = trm_template; trm.set(...)`) — saves 36-byte copy per glyph; hoisted `trm_const_a/b` outside loop
- **Removed**: `x_scale()`, `y_scale()` (unused in production; test coverage removed)
- **Result** (PGO, 100 PDFs): process_content 2688ms → 2547ms (5.3% faster), wall 0.478s → 0.47s (~1.7%)
- **Files**: `text_position.rs`, `engine.rs`, `stripper.rs`, `comparator.rs`
