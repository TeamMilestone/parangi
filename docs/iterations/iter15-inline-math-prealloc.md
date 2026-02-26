# Iteration 15: Inline Math + Pre-allocation + ActualText Clone Removal

## Problem
Post-iteration 12 profile is very flat (no hotspot > 2%). Need micro-optimizations.

## Changes

### 15a: Inline end-position computation
- Replaced `Matrix::translate_instance(tx_visual, 0.0).multiply(tm).multiply(ctm)`
  (1 Matrix ctor + 2 full 3x3 multiplies = ~54 FLOPs + 1 alloc) with direct arithmetic
  (6 multiplies + 4 adds).
- Key insight: translate(tx,0) × Tm only changes Tm's translation components.

### 15b: Remove actual_text clone
- Removed `self.actual_text.clone()` before the glyph loop.
- Instead, use `if let Some(ref at) = self.actual_text` directly.
- Saves 1 String clone per `show_text()` call.

### 15c: Pre-allocate output String
- `String::with_capacity(total_len)` in page merging, computed from page text lengths.

### 15d: Pre-allocate text_positions Vec
- `self.text_positions.reserve(content_bytes.len() / 10)` heuristic on first call.

## Results

| Build | Wall (100 files, 1 thread) | User |
|-------|---------------------------|------|
| Baseline | 6.112s | 5.947s |
| iter15 (code only) | 5.994s | 5.825s |
| PGO only | 5.640s | 5.509s |
| **iter15 + PGO** | **5.615s** | **5.485s** |

Combined improvement: **8.1% wall time** reduction.

## Attempted & Reverted
- **Sparse TRM multiply**: Exploiting params matrix zeros — compiler already optimizes this.
- **Lazy caching of scaling_factor_x/y**: Branch overhead outweighed sqrt savings.
- Both were reverted for being slower than baseline.

## Output Verification
100/100 files byte-identical with baseline (0 diffs).
