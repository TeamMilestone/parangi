# Iteration pdf12: Marked Content (BMC/BDC/EMC) + ActualText

## 변경사항

### stream/operators.rs — 오퍼레이터 상수 추가
- `MP`, `DP`: Marked Content Point 오퍼레이터 (no-op 처리)

### stream/engine.rs — Marked Content 처리

#### 새 구조체
- `MarkedContentEntry`: 마크드 콘텐츠 스택 항목 (tag + actual_text)

#### 새 필드 (StreamEngine)
- `marked_content_stack: Vec<MarkedContentEntry>` — BMC/BDC push, EMC pop
- `actual_text: Option<String>` — 활성 ActualText 값
- `first_actual_text_position: bool` — ActualText 스팬 내 첫 번째 글리프 플래그

#### 새 메서드
- `op_bmc()`: BMC 오퍼레이터 — 태그만 스택에 push (ActualText 없음)
- `op_bdc()`: BDC 오퍼레이터 — properties에서 ActualText 추출 → 스택 push → ActualText 활성화
- `op_emc()`: EMC 오퍼레이터 — 스택 pop → ActualText 해제
- `extract_actual_text()`: BDC operands에서 ActualText 값 추출
  - inline Dictionary 지원
  - indirect Reference 해석
  - UTF-16BE (BOM FE FF) 디코딩
  - PDFDocEncoding (Latin-1) 폴백

#### ActualText 텍스트 대체 로직 (show_text 수정)
- ActualText 활성 시:
  - 첫 번째 글리프 → ActualText 전체 값으로 unicode 대체
  - 이후 글리프 → empty string (억제)
- ActualText 비활성 시: 기존 unicode 매핑 사용
- Soft hyphen (U+00AD) 자동 제거 (PDFBox 동작 일치)

### 처리 흐름
```
BDC /Span << /ActualText (fi) >>
  ↓
marked_content_stack.push({ tag: "Span", actual_text: Some("fi") })
actual_text = Some("fi"), first_actual_text_position = true
  ↓
Tj <0001>   → TextPosition { unicode: "fi" }  (첫 번째)
Tj <0002>   → 억제 (empty string → 생성 안 됨)
  ↓
EMC
  ↓
marked_content_stack.pop()
actual_text = None
```

### 중첩 지원
- Marked Content 스택 기반으로 중첩 BMC/BDC 블록 정확 처리
- 내부 BDC의 ActualText만 활성 → EMC로 해제 → 외부 레벨 복원
- PDFBox와 동일한 스택 기반 동작

## 테스트 결과
```
Unit tests: 102 passed (7개 신규)
Integration tests: 10 passed
Total: 112 passed, 0 failed
```

## 신규 테스트 (7개)
- `test_bmc_emc_no_effect_on_text`: BMC/EMC만으로는 텍스트 영향 없음
- `test_bdc_actual_text_replaces_glyphs`: ActualText로 글리프 유니코드 대체
- `test_bdc_actual_text_utf16be`: UTF-16BE 인코딩 ActualText (한국어 "가")
- `test_bdc_actual_text_soft_hyphen_removed`: Soft hyphen 자동 제거
- `test_nested_marked_content`: 중첩 BMC/BDC 정확 처리
- `test_emc_without_bmc_is_safe`: 매칭 없는 EMC 안전 처리
- `test_actual_text_with_no_glyphs`: 글리프 없는 ActualText 블록

## 다음 단계
- Phase 4 시작: Iter pdf13 — 기본 텍스트 조립 (단어/줄 감지)
