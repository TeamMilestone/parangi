# Iteration pdf18: 파일별 병렬 처리 + 배치 API + comparator 수정

## 변경사항

### lib.rs — 배치 처리 API

#### `BatchResult` 구조체
- `path: PathBuf` — 처리된 파일 경로
- `result: Result<String>` — 추출 결과 또는 에러

#### `extract_text_batch()`
- 여러 PDF 파일을 rayon `par_iter`로 병렬 처리
- 중첩 병렬성: 파일 수준(par_iter) × 페이지 수준(내부 par_iter)
- rayon의 work-stealing 스케줄러가 자동 최적화

### text/comparator.rs — Total order 수정 (버그 수정)

#### 문제
- 기존 `same_line` 판정이 비전이적(non-transitive): A≈B, B≈C이지만 A≉C인 경우 발생
- `partial_cmp` 사용으로 NaN 비교 시 undefined behavior 가능
- Rust 1.81+ sort에서 total order 위반 시 panic 발생

#### 해결
- `f32::total_cmp()` 사용으로 NaN 안전한 비교 보장
- Y 거리 tolerance를 평균 높이의 50%로 단순화하여 전이성 유지
- 비전이적 overlap 방식 → 대칭적 distance 방식으로 변경

### CLI (pdfbox-text-cli) — 배치 모드 개선

#### 단일 파일 모드
- `--output-file/-f` 옵션으로 출력 파일 지정 가능
- `--sequential` 플래그로 단일 스레드 모드 지원
- 출력 파일 미지정 시 stdout

#### 배치 모드 (다중 파일)
- `--output-dir/-o` 옵션으로 출력 디렉토리 지정
- 파일별 병렬 처리 (extract_text_batch 사용)
- 각 PDF → 동명의 .txt 파일로 출력
- 처리 결과 요약 (성공/실패 건수)

## 테스트 결과
```
Unit tests: 128 passed
Integration tests: 12 passed
Total: 140 passed, 0 failed
```

### 실제 PDF 배치 테스트
- 5개 한국어 PDF 파일 동시 처리 성공
- 총 처리 시간: 1.1초 (5개 파일, 155KB 총 출력)
- 0 실패

## 다음 단계
- Iter pdf19: 프로파일링 + 최적화 (flamegraph, SmallVec, 제로카피)
