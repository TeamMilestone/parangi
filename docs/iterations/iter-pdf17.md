# Iteration pdf17: 페이지별 병렬 처리 (rayon)

## 변경사항

### lib.rs — rayon 기반 병렬 파이프라인

#### `extract_text_with_config()` 재구현
- **순차 수집 단계**: lopdf 페이지 트리 탐색은 스레드 안전하지 않으므로 순차적으로 페이지 데이터 수집
  - content_bytes, resources_dict, rotation, media_box를 페이지별로 미리 추출
- **병렬 처리 단계**: `rayon::par_iter()`로 페이지별 독립 처리
  - 각 스레드에서 StreamEngine 인스턴스 생성 (doc_arc.clone() 공유)
  - 콘텐츠 스트림 파싱 → TextPosition 생성 → 텍스트 조립
- **결과 병합**: 페이지 번호 기준 정렬 후 순차 결합

#### `extract_text_sequential()` 추가
- 디버깅 및 단일 스레드 필요 시 사용 가능한 순차 처리 함수

### 아키텍처
```
PdfDocument (Arc<lopdf::Document>)
    │
    ├─ Sequential: 페이지별 데이터 수집
    │   └─ (page_num, content_bytes, resources_dict, rotation, media_box)
    │
    ├─ Parallel (rayon par_iter):
    │   ├─ Page 0 → StreamEngine → TextPositions → assemble_text
    │   ├─ Page 1 → StreamEngine → TextPositions → assemble_text
    │   └─ Page N → StreamEngine → TextPositions → assemble_text
    │
    └─ Sequential: 페이지 번호 순 정렬 → 최종 문자열 결합
```

## 테스트 결과
```
Unit tests: 128 passed
Integration tests: 12 passed
Total: 140 passed, 0 failed
```

기존 테스트 모두 통과. 병렬 처리가 기존 순차 처리와 동일한 결과를 생산함을 확인.

## Phase 5 진행 현황
- **Iter 17** ✅: 페이지별 병렬 처리 (rayon)
- **Iter 18**: 파일별 병렬 처리 — 중첩 병렬성 (파일 × 페이지) + memmap
- **Iter 19**: 프로파일링 + 최적화

## 다음 단계
- Iter pdf18: 파일별 병렬 처리 (중첩 병렬성)
