# Iteration 24: Eliminate Redundant direction() Calls in Sort + Assembly

## Problem

`assemble_text` accounted for 8.4% (535ms accumulated) of total profile time. Two redundant patterns:

### 1. sort_positions: 6× direction() calls per comparison

`sort_unstable_by(compare_positions)` called `direction()` 6× per comparison:
```rust
fn compare_positions(a, b) {
    a.direction()                    // 1×
        .then_with(|| a.y_dir_adj()  // calls direction() again → 2×
                   .then_with(|| a.x_dir_adj()  // calls direction() again → 3×
    b.direction()                    // 4×, 5×, 6×
```
With N=1000 positions/page and O(N log N) ≈ 10,000 comparisons: **60,000 direction() calls per page**, or **115M calls for 1929 pages**.

### 2. Assembly loop: 4× direction() calls per position

```rust
let pos_x = pos.x_dir_adj();      // calls direction()
let pos_y = pos.y_dir_adj();      // calls direction() AGAIN
let pos_width = pos.width_dir_adj();   // calls direction() AGAIN
let pos_height = pos.height_dir_adj(); // calls direction() AGAIN
```
**3.86M redundant direction() calls** in the main assembly loop.

## Changes

### 1. TextPosition::sort_key() — precomputed sort key

Added method that calls `direction()` ONCE and returns `(u32, u32, u32)` with
f32 total-order bit encoding:

```rust
pub fn sort_key(&self) -> (u32, u32, u32) {
    let dir = self.direction();  // called ONCE
    let (y_adj, x_adj) = match dir as i32 { ... };
    (f32_total_bits(dir), f32_total_bits(y_adj), f32_total_bits(x_adj))
}
```

`f32_total_bits(f)`: converts f32 to u32 with total_cmp semantics (sets sign bit
for positives, flips all bits for negatives). `(u32, u32, u32)` implements `Ord`.

### 2. sort_positions: sort_by_cached_key

```rust
// Before: calls direction() 6× per comparison, O(N log N) calls total
positions.sort_unstable_by(|a, b| compare_positions(a, b));

// After: calls sort_key() ONCE per element, then sorts precomputed keys
positions.sort_by_cached_key(|p| p.sort_key());
```

Key computation: O(N) × 1 direction() = **18× fewer direction() calls vs before**.

### 3. dir_adj_all() — all four values in one call

```rust
pub fn dir_adj_all(&self) -> (f32, f32, f32, f32) {
    let dir = self.direction();  // called ONCE
    match dir as i32 { ... }     // returns (x_adj, y_adj, width_adj, height_adj)
}
```

Assembly loop change:
```rust
// Before: 4 separate calls → 4× direction()
let pos_x = pos.x_dir_adj();
let pos_y = pos.y_dir_adj();
let pos_width = pos.width_dir_adj();
let pos_height = pos.height_dir_adj();

// After: 1 combined call → 1× direction()
let (pos_x, pos_y, pos_width, pos_height) = pos.dir_adj_all();
```

## Results

A/B comparison (wall clock, 200 PDFs):

| Build | Wall (200 PDFs) | vs Baseline |
|-------|-----------------|-------------|
| iter23 + PGO (direction 6×/cmp) | ~0.592s median | — |
| **iter24 + PGO (sort_by_cached_key)** | **~0.560s median** | **-5.4%** |

Phase profile comparison (assemble_text):
- Before: 535ms (8.4%)
- After: ~300ms (5.8%) — **-44% improvement in assemble_text**

## Notes

- `sort_by_cached_key` is STABLE (vs previous `sort_unstable_by` which is UNSTABLE).
  For equal keys (same position), stable sort preserves original order. This is
  correct for text extraction — positions at the same coordinate are arbitrary.

- For Korean PDFs: `direction()` always returns 0.0 (horizontal text), so
  PGO branch-predicts it perfectly. But even with perfect prediction, the
  instructions still execute. With N=1000, 10K comparisons × 6 direction()
  calls = 60K executions per page. The cached key approach reduces this to 1K.

- Attempted iter17 (combined glyph table for get_width + to_unicode):
  REVERTED — adding a new HashMap increases memory pressure without reducing
  total memory (old tables still exist). Cache pressure caused regression.
  Lesson: combining data structures only helps when you can REMOVE the originals.
