# Iteration 19: Skip Stream Content for Non-Text Objects

## Problem
lopdf eagerly loads ALL object stream content (via `to_vec()`) when parsing PDFs.
For text extraction, image data, font programs, and metadata streams are never accessed
but still incur allocation and copy costs.

## Analysis (200-file sample from 20K corpus)

| Stream Type | Objects | % of Total | Stream Bytes |
|---|---|---|---|
| Image (Subtype=Image) | 3,315 | 4.2% | ~29 MB |
| Embedded font data | 1,489 | 1.9% | ~32.5 MB |
| Metadata (Type=Metadata) | 1,338 | 1.7% | minor |
| **Skippable total** | **~6,142** | **~7.8%** | **~62.7 MB (73.4% of stream bytes)** |
| Content/ToUnicode (needed) | 17,758 | 22.7% | ~22.8 MB |

## Changes

### lopdf: `skip_stream_content` predicate (parser level)
- Added `skip_stream_content: Option<fn(&Dictionary) -> bool>` field to `Reader`
- In parser `stream()`, checks predicate BEFORE `data.to_vec()`:
  ```rust
  let should_skip = reader.skip_stream_content
      .map(|pred| pred(&dict))
      .unwrap_or(false);
  let content = if should_skip { vec![] } else { data.to_vec() };
  ```
- Parser still advances past stream data (zero-cost `take()`), only skips the copy
- Added `Document::load_mem_skip_streams(buffer, predicate)` public API
- Refactored Reader initialization with `Reader::new()` constructor

### parang: `should_skip_stream` predicate
Skips stream content for:
1. **Image XObjects**: `Subtype = Image` — never accessed by text extraction
2. **Metadata streams**: `Type = Metadata` — never accessed
3. **CFF/OpenType fonts (FontFile3)**: `Subtype = CIDFontType0C | Type1C | OpenType`
4. **TrueType fonts (FontFile2)**: heuristic — has `/Length1` but no `/Type` or `/Subtype`

Parang only reads: content streams, ToUnicode CMaps, encoding CMaps, font dictionaries.
It never accesses actual font program data (glyph outlines).

## Results

| Build | Wall (100 files, 1 thread) | User | vs Baseline |
|-------|---------------------------|------|-------------|
| Baseline | 6.112s | 5.947s | — |
| Skip images only | 5.922s | 5.797s | -3.1% |
| Skip all (images+fonts+meta) | 5.839s | 5.726s | -4.5% |
| PGO only | 5.615s | 5.485s | -8.1% |
| Skip images + PGO | 5.461s | 5.344s | -10.6% |
| **Skip all + PGO** | **5.456s** | **5.343s** | **-10.7%** |

## Output Verification
100/100 files MD5-identical with baseline (`aa8c2d04536ab08250d8eedbc5caffc5`).
