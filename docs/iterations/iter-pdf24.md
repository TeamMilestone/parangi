# Iteration pdf24: lopdf 0.39 upgrade + inline image & NUL byte handling

## Changes

### 1. Upgrade lopdf 0.35 → 0.39
- Fixes critical Stream-as-Dictionary parsing bug
  - In 0.35, certain PDF stream objects (esp. ToUnicode CMaps) were parsed as plain Dictionaries, losing stream content
  - This caused `to_unicode()` to return None → empty text extraction for ~18 files
  - v0.36 fixed: "Avoid parsing encrypted object streams early and correctly parse object streams upon decryption"
  - v0.39 added: "Handle Byte Order Marks in CMaps" + "Fixed parsing of empty bfrange/bfchar sections in ToUnicode CMaps"
- All ToUnicode CMap streams now correctly parsed as Stream objects (verified via diagnostic tool)

### 2. Inline image stripping in content stream parser (`engine.rs`)
- lopdf's `Content::decode` fails on inline images (BI...ID...EI operators) containing binary data
- Added `strip_unparseable_content()` that removes inline image blocks before parsing
- Properly tracks parenthesized string literal depth to avoid corrupting string data
- NUL bytes inside string literals `(\x00\x14)` are preserved (valid CID character codes)
- NUL bytes outside strings are stripped (HWP→PDF converter padding)

### 3. Robust content stream decoding strategy (`engine.rs`)
Three-tier fallback:
1. **Fast path**: Direct `Content::decode` for clean streams (no NUL bytes)
2. **Clean path**: Strip inline images + external NUL bytes, then decode
3. **Chunk path**: Split on NUL byte runs, decode each chunk independently

### 4. Sequential mode error resilience (`lib.rs`)
- `extract_text_sequential()` now skips pages with decode errors instead of failing the entire file
- Matches parallel mode behavior (which already used `is_err()` to skip)

## Impact on Previously-Failing Files

| File | Before (0.35) | After (0.39) | Java | Improvement |
|------|--------------|-------------|------|-------------|
| 단샘유치원 공고 | 45 | **19,155** | 8,088 | 42,567% (>Java) |
| 규격서 PA2026 | 11 | **3,568** | 2,439 | 32,336% (>Java) |
| 지능정보사회 제안요청서 | 42 | **17,585** | 7,285 | 41,769% (>Java) |
| 방송사업자 제안요청서 | 41 | **14,226** | 5,735 | 34,578% (>Java) |
| 기상탑 타워 도면 | 24 | 24 | 14,671 | No change (CAD) |

Root cause for NUL-padded PDFs: HWP→PDF converters insert NUL byte padding between PDF operators, causing `Content::decode` to stop early or fail entirely.

## Root Cause Analysis: Remaining Low-Quality Files

| Category | Count | Cause |
|----------|-------|-------|
| CAD drawings | ~20 | Coordinate-heavy data with different rendering paths |
| HWP→PDF with binary inline images | ~5 | Complex inline image + NUL patterns partially recovered |
| Image-only/minimal text | ~5 | No text to extract |
| Font encoding edge cases | ~5 | Specific font types not handled |

## Test Results
- 140 tests pass (128 unit + 12 integration)
- `cargo build --release` success
- No regressions on existing test suite

## Next Steps
- Re-run full corpus E2E with directory structure preservation for accurate comparison
- Profile inline image stripping performance impact
- Investigate remaining CAD drawing extraction differences
