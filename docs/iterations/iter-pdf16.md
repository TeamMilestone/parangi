# Iteration pdf16: 유니코드 정규화 + 분음부호 병합

## 변경사항

### text/normalizer.rs (NEW)

#### `normalize_text()`
- 리거처 분해 + Unicode NFC 정규화
- `unicode-normalization` 크레이트 사용

#### `decompose_ligatures()`
- 7가지 일반 리거처 분해:
  - U+FB00 → ff, U+FB01 → fi, U+FB02 → fl
  - U+FB03 → ffi, U+FB04 → ffl
  - U+FB05/FB06 → st

#### `merge_diacritics()`
- 결합 분음부호(combining marks)가 별도 글리프로 나온 경우 이전 문자와 병합
- 병합 후 NFC 정규화 적용 (e + \u0301 → é)
- 5가지 Unicode 결합 문자 범위 지원:
  - U+0300-036F (Combining Diacritical Marks)
  - U+1AB0-1AFF (Extended)
  - U+1DC0-1DFF (Supplement)
  - U+FE20-FE2F (Half Marks)
  - U+20D0-20FF (For Symbols)

### text/stripper.rs — 정규화 통합

#### assemble_text() 파이프라인 업데이트
```
TextPositions
  → sort → suppress_duplicates → remove_contained_spaces
  → merge_diacritics (NEW)     ← 결합 분음부호 병합
  → 단어/줄/단락 감지
  → normalize_text (NEW)       ← NFC + 리거처 분해
  → String
```

### text/mod.rs
- normalizer 모듈 추가 및 normalize_text re-export

## 테스트 결과
```
Unit tests: 128 passed (8개 신규)
Integration tests: 12 passed
Total: 140 passed, 0 failed
```

## 신규 테스트 (8개)
- `test_normalize_basic`: 기본 텍스트 변환 없음
- `test_ligature_decomposition`: fi/fl/ff/ffi/ffl 리거처 분해
- `test_nfc_normalization`: e + 결합 악센트 → é
- `test_merge_diacritics`: 별도 글리프 결합 분음부호 병합
- `test_merge_diacritics_no_combining`: 결합 없는 경우 변경 없음
- `test_is_combining_mark`: 결합 문자 판별
- `test_korean_text_unchanged`: 한국어 텍스트 통과
- `test_merge_multiple_diacritics`: 여러 분음부호 동시 병합

## Phase 4 완료!
Phase 4 (Iter 13-16: 텍스트 조립)의 모든 이터레이션이 완료되었습니다.

### Phase 4 요약
- **Iter 13**: TextPosition 정렬 + 단어/줄 감지 + 기본 텍스트 조립
- **Iter 14**: 중복 텍스트 제거 + 포함된 공백 제거
- **Iter 15**: 단락 감지 (drop/indent threshold + list item 패턴)
- **Iter 16**: 유니코드 정규화 (NFC + 리거처) + 분음부호 병합

## 다음 단계
- Phase 5 시작: Iter pdf17 — 페이지별 병렬 처리 (rayon)
