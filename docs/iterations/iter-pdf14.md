# Iteration pdf14: 중복 텍스트 제거 + 포함된 공백 제거

## 변경사항

### text/stripper.rs — 전처리 함수 추가

#### `suppress_duplicate_positions()`
- PDFBox processTextPosition() 중복 억제 로직 포팅
- 같은 문자가 같은 위치(tolerance: 문자 너비 / 글자수 / 3)에 나타나면 중복 제거
- HashMap<String, Vec<(f32, f32)>>로 문자별 위치 추적
- 볼드/그림자 효과로 겹쳐진 텍스트 처리

#### `remove_contained_spaces()`
- PDFBox removeContainedSpaces() 포팅
- 공백 문자의 X 범위가 인접 문자의 X 범위에 완전히 포함되면 제거
- 인접 ±5개 문자만 검사 (성능 최적화)
- 같은 줄(Y overlap)인 경우만 처리

#### StripperConfig 확장
- `suppress_duplicates: bool` (기본: true) — 중복 텍스트 억제 활성화

### 처리 파이프라인
```
TextPositions
  ↓
sort_positions()          — 읽기 순서 정렬
  ↓
suppress_duplicate_positions()  — 중복 제거 (NEW)
  ↓
remove_contained_spaces()       — 포함된 공백 제거 (NEW)
  ↓
assemble_text()           — 단어/줄/단락 감지
  ↓
String
```

## 테스트 결과
```
Unit tests: 117 passed (3개 신규)
Integration tests: 12 passed
Total: 129 passed, 0 failed
```

## 신규 테스트 (3개)
- `test_duplicate_suppression`: 동일 위치 동일 문자 중복 제거
- `test_duplicate_suppression_different_chars`: 다른 문자는 유지
- `test_contained_space_removal`: 넓은 문자 내부 공백 제거

## 다음 단계
- Iter pdf15: Article bead 분리 + 단락 감지 개선
