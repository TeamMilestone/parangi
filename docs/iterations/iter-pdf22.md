# Iteration pdf22: Full corpus E2E test (20,219 PDFs)

## Full Corpus Results

### Extraction Summary
| Metric | Value |
|--------|-------|
| Input PDFs | 20,219 |
| Output files | 20,089 (130 filename collisions — flat dir) |
| Non-empty output | 18,940 (94.3%) |
| Empty output | 1,149 (5.7%) |
| - Rust empty, Java has content | **19** (0.09%) |
| - Both empty (image-only PDFs) | 1,130 (5.6%) |

### Quality Comparison with Java PDFBox
| Metric | Value |
|--------|-------|
| Files compared (both non-empty) | 18,934 |
| **Character frequency similarity** | **99.4%** |
| Length ratio | 99.1% |
| >= 99% similarity | 17,573 (92.8%) |
| >= 95% similarity | 18,349 (96.9%) |
| >= 90% similarity | 18,735 (98.9%) |
| >= 80% similarity | 18,889 (99.8%) |
| < 70% similarity | 28 (0.15%) |

### Performance
| Metric | Value |
|--------|-------|
| Total time | **9 min 22 sec** |
| Parallelism | 10 processes × rayon page-level |
| CPU utilization | 540% |
| Throughput | ~36 PDFs/sec |

### Worst Cases Analysis (< 70% similarity)
- 28 files total, mostly **CAD drawings** (도면 PDF) with coordinate-heavy text
- 3 files with near-zero length ratio — Rust produces tiny output while Java extracts full text
  - Likely caused by missing font or encoding handling for specific font types
- Only 19 Rust-only failures in entire corpus (Java succeeds, Rust empty)

## Key Takeaways

1. **99.4% quality parity** with Java PDFBox on 20K Korean government PDFs
2. **9.5 minutes** for full corpus (vs estimated 30+ min with Java 8-process setup)
3. Remaining failures are mostly image-only PDFs (1,130) and edge cases (19 Rust-only + 28 low quality)
4. Low-quality cases are predominantly CAD drawing PDFs — acceptable for bid document text extraction use case

## Next Steps
- Investigate 19 Rust-only empty files for fixable issues
- Investigate 3 near-zero length ratio files for potential font/encoding bugs
- Consider OCR integration for image-only PDFs (out of scope for this port)
