# Iteration pdf19: 성능 최적화

## 변경사항

### stream/engine.rs — show_text() 핫루프 최적화

#### space_width 사전 캐싱
- 루프 진입 전에 space_width를 한 번만 계산
- 기존: 매 글리프마다 font.get_width(32) + fallback 계산 반복
- 변경: 루프 밖에서 cached_space_width로 1회 계산

#### actual_text clone 제거
- 기존: 매 글리프에서 `self.actual_text.clone()` 호출
- 변경: 루프 진입 전 `actual_text_value`로 1회 clone

#### state_stack 접근 최소화
- 기존: current() 5회 별도 호출로 text state 값 읽기
- 변경: current() 1회로 통합

### font/type0_font.rs — CID Width 범위 압축

#### CidWidths 구조체 신규
- `individual: HashMap<u32, f32>` — 개별 CID 폭 (≤32개 범위)
- `ranges: Vec<WidthRange>` — 대규모 범위 (>32개 CID)
- `finalize()`: 범위를 start 기준 정렬 (binary search용)

#### 성능 개선
- 기존: `for cid in 0..=65535 { widths.insert(cid, w); }` → 65K HashMap 항목
- 변경: 1개 WidthRange 구조체로 대체
- 조회: 개별 HashMap 먼저 → 미스 시 binary search

### text/comparator.rs — 엄격한 total order (pdf18에서 적용)
- 비전이적 tolerance 제거, (direction, Y, X) strict sort
- 실제 PDF에서 sort panic 완전 해결

## 성능 측정 (50개 한국어 PDF)
```
최적화 전: 4.28초 (CPU 148%)
최적화 후: 2.84초 (CPU 241%)
개선: 34% 시간 단축, 63% CPU 활용도 증가
```

## 테스트 결과
```
Unit tests: 128 passed
Integration tests: 12 passed
Total: 140 passed, 0 failed
```

## Phase 5 완료!
- **Iter 17** ✅: 페이지별 병렬 처리 (rayon)
- **Iter 18** ✅: 파일별 병렬 처리 + 배치 API + comparator 수정
- **Iter 19** ✅: 성능 최적화 (캐싱, 범위 압축)

## 다음 단계
- Phase 6: Iter pdf20 — CLI 완성 + E2E 테스트 (500개 PDF 비교)
