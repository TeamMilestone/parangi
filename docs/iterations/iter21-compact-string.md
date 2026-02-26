# Iteration 21: CompactString for TextPosition.unicode

## Problem
Every glyph extracted from a PDF creates a `TextPosition` with a `String` field
for its Unicode representation. Most glyphs are 1-4 bytes (single characters),
but `String` always heap-allocates (24 bytes on stack + heap pointer). With
thousands of glyphs per page, this creates significant allocation pressure.

## Changes

### CompactString for unicode field
- Replaced `TextPosition.unicode: String` with `CompactString` from the
  `compact_str` crate (v0.8)
- CompactString stores strings up to 24 bytes inline (no heap allocation),
  which covers virtually all single-glyph Unicode representations
- Updated engine.rs `show_text()` to construct CompactString directly
- Updated normalizer.rs diacritics merging to convert back via `.into()`
- Updated all test helpers

### Technical notes
- `CompactString::from(char)` is not implemented; used `ch.encode_utf8(&mut buf)`
  followed by `CompactString::from(s as &str)` instead
- NFC normalization in diacritics merging produces a `String` which is converted
  back to `CompactString` via `.into()`

## Results

A/B comparison (hyperfine, 10 runs each, same system, interleaved):

| Build | Wall (100 files, 1 thread) | Std Dev | vs Baseline |
|-------|---------------------------|---------|-------------|
| iter20 (String) | 6.477s | 0.386s | -- |
| **iter21 (CompactString)** | **6.074s** | **0.016s** | **-6.2%** |

Key observation: standard deviation dropped **24x** (0.386s -> 0.016s), indicating
much more consistent performance due to reduced heap allocation pressure.

## Output Verification
100/100 files MD5-identical with baseline (`aa8c2d04536ab08250d8eedbc5caffc5`).
