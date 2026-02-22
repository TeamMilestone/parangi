# Iteration pdf13: 기본 텍스트 조립 — 단어/줄 감지

## 변경사항

### text/comparator.rs (NEW)
- `sort_positions()`: TextPosition 배열을 읽기 순서로 정렬
- 정렬 기준: (1) text direction → (2) Y position (overlap 허용) → (3) X position
- PDFBox TextPositionComparator 포팅
- Y 좌표 0.1pt tolerance + 수직 범위 overlap 감지로 같은 줄 판별

### text/stripper.rs (NEW)
- `StripperConfig`: 텍스트 조립 설정
  - `spacing_tolerance`: 0.5 (공백 너비 기반 단어 경계)
  - `average_char_tolerance`: 0.3 (평균 문자 너비 기반 단어 경계)
  - `drop_threshold`: 2.5 (줄높이 배수 → 단락 감지)
  - `indent_threshold`: 2.0 (공백 너비 배수 → 들여쓰기 단락 감지)
  - `sort_by_position`, `word_separator`, `line_separator`, `paragraph_separator`, `page_separator`

- `assemble_text()`: 핵심 텍스트 조립 함수 (PDFTextStripper.writePage() 포팅)
  - 줄 감지: Y 좌표 overlap 체크 → 다른 줄이면 line break
  - 단어 감지: gap > min(deltaSpace, deltaCharWidth) → word separator 삽입
  - 단락 감지: 수직 gap > drop_threshold × 줄높이 → paragraph separator
  - 이중 threshold: 공백 너비 기반 + 평균 문자 너비 기반 → 작은 값 사용

### text/mod.rs (UPDATED)
- comparator, stripper 모듈 선언 및 re-export

### lib.rs (UPDATED)
- `extract_text()`: StreamEngine + assemble_text() 사용하는 전체 파이프라인
- `extract_text_with_config()`: 커스텀 StripperConfig 지원
- 기존 `extract_text_basic()` 대신 전체 엔진 사용

### 처리 흐름
```
PDF 파일
  ↓
PdfDocument::open()
  ↓
[for each page]:
  page.content_bytes() → StreamEngine.process_content()
    → Vec<TextPosition> (글리프별 위치/유니코드)
  ↓
  sort_positions() → 읽기 순서 정렬
  ↓
  assemble_text() → 단어/줄/단락 감지 → String
  ↓
  페이지 결과 병합
```

## 테스트 결과
```
Unit tests: 114 passed (12개 신규)
Integration tests: 12 passed (2개 신규)
Total: 126 passed, 0 failed
```

## 신규 테스트 (14개)
### comparator (3개)
- sort_same_line, sort_different_lines, sort_overlapping_y_same_line

### stripper (9개)
- overlap_function, simple_word_assembly, word_separation
- line_separation, paragraph_detection, empty_positions
- single_character, korean_text_no_spaces, unsorted_positions

### integration (2개)
- test_full_text_extraction: 전체 PDF 텍스트 추출 (8페이지, 35K자)
- test_text_assembly_with_real_pdf: 1페이지 텍스트 조립 (31줄)

## 실제 한국어 PDF 추출 결과
```
전체: 35,005자, 8페이지
1페이지: 2,931자, 31줄

샘플 출력:
  우편번호: 46508
  주소: 부산광역시  북구  금곡대로  506-17(금곡동  1010-6)
  홈페이지:  http://www.pps.go.kr
  전화번호: 1588-0800
  수요물자(용역)  조달  입찰  공고(긴급)
```

## 알려진 이슈
- 이중 공백: 일부 단어 사이에 double space 발생 (iter14에서 정규화 예정)
- 단락 감지 미세 조정 필요 (iter15)

## 다음 단계
- Iter pdf14: 공간 정렬 + 중복 텍스트 제거
