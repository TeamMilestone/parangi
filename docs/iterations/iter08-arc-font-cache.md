# Iteration 8: Arc<PdfFont> + Font Cache Loop Optimization

## Problem
Profiling (post iter7) revealed:
- `PdfFont::clone` and `CMap::clone` consuming ~2% of CPU
- Font mutex contention at ~3%
- `show_text` inner loop doing 3 HashMap lookups per glyph

## Solution

### Part A: Arc<PdfFont>
Replaced `HashMap<Vec<u8>, PdfFont>` with `HashMap<Vec<u8>, Arc<PdfFont>>`
throughout the engine. Font cache now stores `Arc<PdfFont>` instead of
cloned values. Arc::clone is a cheap atomic increment vs deep cloning
CMap HashMaps.

### Part B: Font Loop Cache
Cached `Arc<PdfFont>` reference once before the glyph loop in `show_text()`.
Previously, each glyph did 3 separate HashMap lookups:
- `self.fonts[name].read_code()`
- `self.fonts[name].get_width()`
- `self.fonts[name].to_unicode()`

Now a single `Arc::clone` before the loop serves all three.

Also simplified `dy_display` calculation: removed `font_height_text * ...`
where `font_height_text` was always 1.0.

## Results

### 100-file benchmark (single-thread, RAYON_NUM_THREADS=1)
| Metric | Baseline (iter7) | Iter 8 | Change |
|--------|------------------|--------|--------|
| User time | 6.25s | 6.13s | **-1.9%** (Arc only) |
| User time | 6.13s | 5.92s | **-3.4%** (+ loop cache) |
| **Combined** | **6.25s** | **5.92s** | **-5.3%** |

### Full 20K benchmark (with PGO)
| Metric | v4 Baseline | Iter 8+PGO | Change |
|--------|-------------|------------|--------|
| User time | 1,296s | 1,209s | **-6.7%** |
| Wall time | 3:49 | 3:37 | **-5.3%** |
| Success/Fail | 20,198/21 | 20,198/21 | No regression |

### Quality
- 100/100 files byte-identical with baseline

## Files Changed
- `crates/parang/src/stream/engine.rs`:
  - `FontCache` type: `Arc<Mutex<HashMap<ObjectId, Arc<PdfFont>>>>`
  - `fonts` field: `HashMap<Vec<u8>, Arc<PdfFont>>`
  - `show_text()`: cached font ref, simplified height calc
