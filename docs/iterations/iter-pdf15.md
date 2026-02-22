# Iteration pdf15: 단락 감지 개선 + List Item 패턴

## 변경사항

### text/stripper.rs — 단락 감지 로직 강화

#### `is_paragraph_separation()` (NEW)
- PDFBox `isParagraphSeparation()` 포팅
- 4가지 규칙으로 단락 경계 감지:
  1. **DROP THRESHOLD**: 수직 gap > drop_threshold × 줄높이 → 단락 분리
  2. **INDENT THRESHOLD**: X 들여쓰기 > indent_threshold × 공백너비 → 단락 (hanging indent 제외)
  3. **Left-of-previous**: 새 줄이 이전 줄 시작보다 왼쪽 → 단락
  4. **List item pattern**: 번호 패턴 매칭 (미래 확장용 기반 마련)

#### `match_list_item_pattern()` (NEW)
- PDFBox `matchListItemPattern()` 포팅
- 첫 번째 단어에서 list item 패턴 감지

#### `matches_list_pattern()` (NEW)
- 10가지 list item 패턴 (regex 의존성 없이 직접 구현):
  - `"."` (bullet), `"1."`, `"[1]"`, `"1)"`, `"A."`, `"a."`, `"A)"`, `"a)"`, `"I."` (Roman), `"i."` (lowercase Roman)

#### `LIST_ITEM_PATTERNS` (NEW)
- 패턴 문자열 상수 배열

### assemble_text() 리팩토링
- `last_line_start_y/x` → `last_line_start_x` + `last_line_start_is_paragraph`
- `current_line_text` 추적 (list item 감지용)
- 단락 감지를 `is_paragraph_separation()` 함수로 분리

## 테스트 결과
```
Unit tests: 120 passed (3개 신규)
Integration tests: 12 passed
Total: 132 passed, 0 failed
```

## 신규 테스트 (3개)
- `test_list_item_patterns`: 10가지 list item 패턴 개별 매칭
- `test_match_list_item_pattern`: 문장에서 list item 감지
- `test_paragraph_with_drop_threshold`: 3줄 텍스트에서 단락 분리

## 다음 단계
- Iter pdf16: 유니코드 정규화 + BiDi 처리 + 분음부호 병합
