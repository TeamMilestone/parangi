# Iteration 7: Fast Number Parser

## Problem
Profiling revealed that `parse_number()` + `f32::from_str()` + `i64::from_str()`
consumed ~37% of total CPU time. The standard library's float parser uses the
Eisel-Lemire algorithm with Dragonbox fallback — overkill for simple PDF numbers
that are always `[+-]digits[.digits]` (no scientific notation).

## Solution
Replaced `s.parse::<f32>()` and `s.parse::<i64>()` with hand-written parsers
that work directly on byte slices:

- **Integer parsing**: Simple loop accumulating `result * 10 + digit`
- **Float parsing**: Parse integer and fractional parts separately as integers,
  then combine using precomputed `POW10[]` lookup table
- Eliminates: UTF-8 validation, Result creation, complex algorithm overhead

## Results

### 100-file benchmark (hyperfine, sequential)
| Metric | Baseline | Iter 7 | Change |
|--------|----------|--------|--------|
| User time | 14.545s | 13.252s | **-8.9%** |
| Wall time | 12.265s | 10.859s | **-11.5%** |

### Full 20K benchmark
| Metric | Baseline | Iter 7 | Change |
|--------|----------|--------|--------|
| User (CPU) time | 1,296s | 1,124s | **-13.3%** |
| Wall time (est.) | 3:49 | ~3:12 | **-16.2%** |
| Success/Fail | 20,198/21 | 20,198/21 | No regression |

### Quality
- 96/96 files byte-identical with baseline (100-file check)
- 20,198/21 success/fail ratio unchanged

## Files Changed
- `crates/parang/src/stream/content_parser.rs`: Replaced `parse_number()` function

## Profile Before (top functions by sample count)
```
parse_number + f32::from_str  ~37%  ← TARGETED
lopdf::reader::read_object    ~22%
show_text                      ~6%
load_fonts_from_resources      ~8%
sort_positions/assemble_text   ~8%
miniz_oxide decompress         ~6%
```
