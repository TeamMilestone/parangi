# Iteration pdf21: Fix stream decompression + CLI output naming

## Changes

### 1. Stream decompression fallback (`cos_helpers.rs`)
- **Root cause**: `lopdf::Stream::decompressed_content()` fails with `DictKey("Filter")` error on uncompressed streams (no Filter dictionary entry)
- **Fix**: Added fallback to raw `stream.content.clone()` when `decompressed_content()` fails
- This resolves ToUnicode CMap loading failures where the CMap stream is stored uncompressed

### 2. CLI output file naming (`main.rs`)
- Changed from `file_stem()` (drops `.pdf` extension) to `file_name()` (keeps full name)
- Output now uses `filename.pdf.txt` format, consistent with Java PDFBox CLI behavior

## E2E Results (500 PDF sample)

### Before fix (v1)
| Metric | Value |
|--------|-------|
| Total files | 495 |
| Empty (extraction failed) | 50 |
| Compared (both non-empty) | 445 |
| Char frequency similarity | 99.4% |

### After fix (v2)
| Metric | Value |
|--------|-------|
| Total files | 495 |
| Empty (extraction failed) | **33** (-17) |
| Compared (both non-empty) | **462** (+17) |
| Char frequency similarity | **99.5%** |
| Line similarity (Jaccard) | 64.9% |
| Length ratio | 99.3% |
| Char >=90% files | 457/462 (98.9%) |
| Char <70% files | 1/462 (0.2%) |

### Remaining 33 empty files analysis
- 32 files: Java also produces near-empty output (<100 bytes) — image-only PDFs, scan documents
- 1 file: HWP→PDF conversion with custom font mapping (11KB Java output, 0 Rust) — tracked for future fix

## Test Results
- 128 unit tests + 12 integration tests = **140 tests pass**
- `cargo build --release` success

## Next Steps
- Full corpus (20,219 PDFs) E2E test
- Investigate HWP→PDF font mapping edge case
- Line similarity improvement (whitespace/linebreak normalization)
