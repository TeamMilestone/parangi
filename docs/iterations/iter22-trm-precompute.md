# Iteration 22: TRM Precomputation in show_text()

## Problem
In `show_text()`, every glyph triggered:
1. Full TRM computation: 2 matrix multiplies (params × Tm × CTM) = ~90 FP ops
2. `scaling_factor_x/y()` calls: 2 `sqrt()` per glyph
3. `font_size_in_pt`: another `sqrt()` via `scaling_factor_x()`
4. 3 separate `state_stack.current()` calls per glyph

Within the show_text loop, only Tm's translation changes (via `translate(tx, 0)`).
TRM's rotation/scale components (a,b,c,d) are constant, so scaling factors and
font_size_in_pt are also constant.

## Changes

### Hoisted constants out of loop
- TRM computed once before loop; scaling factors (sqrt) computed once
- `font_size_in_pt` computed once
- `dy_display`, `space_width_display` computed once
- Eliminated `compute_text_rendering_matrix()` method (now unused)

### TRM translation re-derived per glyph
- Track Tm translation locally (`tm_tx`, `tm_ty`) with same arithmetic as
  `Matrix::translate()` to ensure bit-identical FP results
- Derive TRM translation using same operation order as full matrix multiply:
  `row2 = [rise*tm[3]+tm_tx, rise*tm[4]+tm_ty, 1]`, then `× CTM`
- End position computed using same FP ops as original inline calculation

### Eliminated redundant state accesses
- Reduced from 4 `state_stack.current()` calls per glyph to 1 (mutable, at end)
- All read-only data extracted once before loop

## Results

A/B comparison (hyperfine, 10 runs each):

| Build | Wall (100 files, 1 thread) | Std Dev |
|-------|---------------------------|---------|
| iter21 (CompactString only) | 6.166s | 0.050s |
| **iter22 (+ TRM precompute)** | **6.092s** | **0.036s** |

~1.2% improvement over iter21. Within noise for some runs but theoretically
eliminates 2 matrix multiplies + 3 sqrt per glyph.

## Output Verification
100/100 files MD5-identical with baseline (`aa8c2d04536ab08250d8eedbc5caffc5`).
FP bit-identical: same operation order as original full matrix multiply path.
