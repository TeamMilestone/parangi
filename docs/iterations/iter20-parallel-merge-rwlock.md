# Iteration 20: Merged Parallel Phases + RwLock Font Cache

## Problem
The text extraction pipeline had two separate phases:
1. **Sequential**: Collect page data (content bytes, resources, rotation, media box)
2. **Parallel**: Process pages (engine, text assembly)

This forced all stream decompression and dictionary lookups to run sequentially
before any parallel processing could begin. Additionally, the font cache used
`Mutex` which serializes all access even for read-only lookups.

## Changes

### Merged sequential/parallel phases
- Combined page data collection and page processing into a single `par_iter` pass
- All Document operations are read-only via `Arc<Document>`, so this is thread-safe
- Eliminates intermediate `Vec<(page_num, content_bytes, ...)>` allocation
- Better cache locality: content bytes used immediately after decompression

### RwLock font cache
- Changed `FontCache` from `Arc<Mutex<HashMap<...>>>` to `Arc<RwLock<HashMap<...>>>`
- Cache read (font lookup): uses `read()` for concurrent access
- Cache write (font insert): uses `write()` only when adding new fonts
- Benefits multi-threaded workloads where pages share fonts

## Results

| Build | Wall (100 files, 1 thread) | User | vs Baseline |
|-------|---------------------------|------|-------------|
| Baseline | 6.112s | 5.947s | — |
| iter19 skip streams | 5.839s | 5.726s | -4.5% |
| **iter20 merged+RwLock** | **5.799s** | **5.717s** | **-5.1%** |
| PGO + iter19 | 5.456s | 5.343s | -10.7% |
| **PGO + iter20** | **5.410s** | **5.338s** | **-11.5%** |

Multi-thread (100 files, default threads):
| Build | Wall | User |
|-------|------|------|
| PGO + iter20 | 2.127s | 14.407s |

## Output Verification
100/100 files MD5-identical with baseline (`aa8c2d04536ab08250d8eedbc5caffc5`).
