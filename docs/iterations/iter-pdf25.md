# Iteration pdf25: Word spacing + duplicate suppression — UTF-8 len() bug fix

## Changes

### 1. Fix double-space insertion at word boundaries (`stripper.rs`)
- **Root cause**: When a PDF contains explicit space characters (TextPositions with `unicode=" "`),
  the word boundary gap detection in `flush_line()` would insert an additional word separator BEFORE
  the space TextPosition, producing `" " + " "` = double space at every word boundary.
- **Fix**: Added `!last_unicode.ends_with(&*config.word_separator)` check in `flush_line()`,
  matching Java PDFBox's `!lastProcessedCharacter.getUnicode().endsWith(" ")` behavior.
  This prevents inserting a word separator when the previous character already ends with one.

### 2. Fix UTF-8 byte count vs character count (`stripper.rs` — 3 locations)
- **Root cause**: `pos.unicode.len()` returns UTF-8 byte count, but Java's
  `text.getCharacter().length()` returns UTF-16 code unit count.
  - CJK characters: 3 bytes UTF-8 vs 1 code unit UTF-16
  - `·` (U+00B7): 2 bytes UTF-8 vs 1 code unit UTF-16
- **Impact on word boundary detection**: `aveCharWidth = width/3` instead of `width/1`,
  making `delta_char` tolerance 1/3 of correct value → overly sensitive gap detection
- **Impact on duplicate suppression**: tolerance = `width/2/3 = width/6` instead of
  `width/1/3 = width/3` → failed to suppress overlapping `·` dot leaders
  (PDF renders dot leaders with bold/shadow overlay, producing 2x characters)
- **Fix**: Changed all 3 occurrences of `pos.unicode.len()` to `pos.unicode.chars().count()`:
  1. `assemble_text()` word boundary detection
  2. `flush_line()` word boundary detection
  3. `suppress_duplicate_positions()` tolerance calculation

### 3. Use config tolerances in `flush_line()` instead of hardcoded values
- Previously `flush_line()` used hardcoded `0.5` and `0.3` for spacing/char tolerance.
- Changed to use `config.spacing_tolerance` and `config.average_char_tolerance` for consistency.

## Impact

### Sample file comparison (character count)
| File | Before | After | Java | Match |
|------|--------|-------|------|-------|
| 공고서_전문인력양성 | 17,281 | **15,199** | 15,196 | 99.98% |
| 산출내역서 | 1,189 | **992** | 1,009 | 98.3% |
| 과업지시서 | 3,019 | **2,711** | 2,723 | 99.6% |

### Middle dot duplicate suppression
| File | Before `·` count | After | Java | Match |
|------|-------------------|-------|------|-------|
| 시방서(해체계획서) | 27,745 | **13,917** | 13,936 | 99.9% |
| 과업지시서(모듈러교사) | 3,662 | **1,870** | 1,870 | 100% |

### Full corpus (20,217 PDF files)
| Version | Avg Sim | >= 99% | >= 95% | < 70% |
|---------|---------|--------|--------|-------|
| **v6 (this)** | **99.7%** | **18,339 (96.8%)** | **18,786 (99.2%)** | **21 (0.1%)** |
| v4 (before) | 98.9% | 11,907 (62.9%) | 18,308 (96.7%) | 30 (0.2%) |

Delta from v4:
- Avg similarity: +0.8%p
- >= 99% files: +6,432 files (+33.9%p)
- >= 95% files: +478 files (+2.5%p)
- < 70% files: -9 files

## Test Results
- 140 tests pass (128 unit + 12 integration)
- `cargo build --release` success
- No regressions on existing test suite

## Remaining 95-99% Analysis (601 files → 447 files)
After fixes, files in the 95-99% range are dominated by:
- **CAD/drawing PDFs (63%)**: Different text block ordering vs Java due to complex page layouts
- **Minor spacing differences (23%)**: Edge cases in word/line boundary detection
- **Font encoding edge cases (10%)**: Character mapping differences

## Next Steps
- Investigate 1,215 Rust-empty files (lopdf xref parsing limitation)
- Consider alternative PDF parser for xref-problematic files
- Remaining 21 worst-case files are all CAD/design drawings — fundamentally different text ordering
