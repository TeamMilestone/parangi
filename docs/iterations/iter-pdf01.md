# Iteration 1: 프로젝트 스캐폴드 + lopdf + memmap I/O

## 변경사항

### 프로젝트 구조
- Cargo workspace 설정 (`Cargo.toml`)
- `crates/pdfbox-text/` — 라이브러리 크레이트
- `crates/pdfbox-text-cli/` — CLI 바이너리 크레이트
- `.claude/commands/iterate.md` — 이터레이션 워크플로우 설정
- `docs/iterations/` — 이터레이션 문서 디렉토리
- `/Volumes/ssd1tb/pdfbox-test/` — E2E 테스트 작업 디렉토리 (외장 SSD)

### 핵심 모듈
| 파일 | 설명 |
|------|------|
| `lib.rs` | 공개 API: `extract_text()` |
| `error.rs` | `PdfError` enum, `Result` type alias |
| `document.rs` | `PdfDocument` — lopdf 래퍼, memmap I/O, 페이지 열거 |
| `page.rs` | `PdfPage` — 콘텐츠 스트림 추출, 미디어/크롭 박스, 회전 |
| `resources.rs` | `PdfResources` — 폰트/XObject/ExtGState 리소스 조회 |
| `cos_helpers.rs` | `DocumentExt`, `ObjectExt` — lopdf 확장 트레이트 |
| `main.rs` (CLI) | clap 기반 CLI, 다중 파일 입력, 스레드 수 지정 |

### 스텁 모듈 (향후 구현)
- `font/mod.rs`, `encoding/mod.rs`, `stream/mod.rs`, `text/mod.rs`

### 의존성
- `lopdf 0.35` — PDF 파싱
- `memmap2 0.9` — 메모리 매핑 I/O
- `rayon 1.10` — 병렬 처리
- `thiserror 2` — 에러 derive
- `clap 4` — CLI 파싱

## 테스트 결과
```
running 4 tests
test document::tests::test_from_bytes_invalid ... ok
test document::tests::test_open_nonexistent_file ... ok
test document::tests::test_invalid_page_index ... ok
test document::tests::test_from_bytes_minimal_pdf ... ok

test result: ok. 4 passed; 0 failed
```

## 실제 PDF 테스트
- 한국어 PDF로 CLI 실행 확인 — 텍스트가 추출되지만 CJK 인코딩 미구현으로 글자 깨짐 (예상대로)
- 폰트 디코딩은 Iter 5-8에서 구현 예정

## 다음 단계 (Iter 2)
- COS 헬퍼 레이어 강화
- PdfResources 테스트 추가
- 스트림 디코딩 검증 (다양한 압축 방식)
