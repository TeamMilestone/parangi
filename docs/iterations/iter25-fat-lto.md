# Iteration 25: Fat LTO + Parse Hex-String Allocation Reduction

## Changes

### 1. Fat LTO (whole-program optimization)

`Cargo.toml` [profile.release]:
```toml
# Before:
lto = "thin"

# After:
lto = true  # fat LTO
```

Fat LTO (full LLVM bitcode LTO) enables whole-program optimization across **all
crates** at link time. Compared to thin LTO:

- Inlining of larger cross-crate functions (e.g., lopdf → parang boundaries)
- Global dead code elimination (DCE) of unused lopdf features (PDF writing,
  encryption, etc.) → smaller I-cache footprint
- Better whole-program constant propagation and value numbering
- More aggressive instruction scheduling across function call boundaries

With `codegen-units = 1` already set, fat LTO adds cross-crate optimization that
thin LTO's summary-based approach can miss for larger functions.

### 2. parse_hex_string allocation reduction

```rust
// Before: allocates 64 bytes even for 2-byte Korean glyph codes
let mut result = Vec::with_capacity(64);

// After: allocates 4 bytes (covers 1–2 glyph codes without realloc)
let mut result = Vec::with_capacity(4);
```

Korean CJK fonts use Identity-H encoding: each glyph code is 2 bytes in hex
format (`<4F2A>`). The old capacity of 64 bytes wasted 62 bytes per allocation.
Capacity 4 covers:
- 2-byte strings (1 glyph): allocate 4, use 2 — no realloc
- 4-byte strings (2 glyphs): allocate 4, use 4 — perfect
- 6-byte strings: one realloc to 8 — rare

With ~200 TJ string allocations per page × 1929 pages per 100 files, this
reduces total heap allocation per 100-file batch by ~(64−4) × 400K =
~24MB → less allocator cache pressure.

### 3. Unused variable cleanup

Removed `has_dot` variable in `parse_number` (was set but never read).

## Results

A/B comparison (wall clock, 200 PDFs, 8 runs excluding first cold run):

| Build | Wall (200 PDFs) | vs Baseline |
|-------|-----------------|-------------|
| iter24 + PGO + thin LTO | ~0.560s median | — |
| **iter25 + PGO + fat LTO** | **~0.547s median** | **-2.3%** |

Run distribution (iter25): 0.532, 0.541, 0.546, 0.547, 0.553, 0.555, 0.566s
→ Median: 0.547s

## Notes

- Fat LTO compile time: ~2x longer than thin LTO (~55s vs 33s for instrumented
  build), but the PGO binary build itself takes ~31s — acceptable.
- Profile breakdown (accumulated) is noisy between runs due to Rayon work-stealing
  scheduling variability. Wall clock measurements (8 runs) are reliable.
- The 2.3% improvement reflects the combined effect of both changes.
  Attribution between fat LTO and the allocation reduction is not separately
  measured but fat LTO likely dominates.
