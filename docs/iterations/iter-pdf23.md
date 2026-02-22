# Iteration pdf23: Decrypt encrypted PDFs + content_bytes fallback

## Changes

### 1. Auto-decrypt encrypted PDFs (`document.rs`)
- Added automatic decryption with empty password on document load
- Most "encrypted" Korean government PDFs use empty password (owner-only restriction)
- Successfully extracts previously-failing encrypted PDF (물품구매, 20,049 chars)

### 2. Unify content_bytes() with get_stream_data() fallback (`page.rs`)
- `content_bytes()` now uses `get_stream_data()` which has fallback to raw content
- Handles Array contents (multiple content streams) and single stream/reference

### Investigation: Remaining 18 failures (lopdf limitation)
Root cause identified: **lopdf parses certain PDF stream objects as plain dictionaries**
- Objects like ToUnicode CMap streams lose their stream content during parsing
- `get_object(id)` returns a Dictionary with only Length/Filter keys instead of a Stream
- This causes `to_unicode()` to return None for all character codes → empty extraction
- Affects PDFs with certain xref table structures (not necessarily linearized)
- This is a lopdf parser limitation, not fixable in our layer

### Analysis of 19 Rust-only failures
| Category | Count | Fix |
|----------|-------|-----|
| Encrypted (empty pwd) | 1 | Fixed in this iteration |
| lopdf stream parse bug | ~14 | Requires lopdf fix or alternative parser |
| HWP→PDF conversion | ~3 | Custom font mapping needed |
| Image-only scan | ~1 | No text to extract |

## Test Results
- 140 tests pass (128 unit + 12 integration)
- `cargo build --release` success

## Next Steps
- Consider upgrading lopdf or using pdf-rs as fallback for problematic PDFs
- Full corpus re-extraction with these fixes
