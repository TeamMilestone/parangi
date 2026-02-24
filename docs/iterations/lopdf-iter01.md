# lopdf-iter01: xref 파싱 수정으로 PDF 로드 실패 해결

## 날짜
2026-02-25

## 목표
lopdf 0.39의 xref 파싱 한계로 실패하던 300개 PDF 파일 로드 성공

## 분석

### 실패 원인 분류 (lopdf 0.39 기준)
| 에러 | 파일 수 | 원인 |
|------|---------|------|
| InvalidXref | 100 | XRef stream + Linearized/incremental PDF |
| InvalidTrailer | 7 | 동일 (에러 분류만 다름) |
| Xref(Start) | 2 | 파일 끝에서 %%EOF 검색 범위 부족 |
| **이미 성공** | **191** | lopdf 0.39에서 이미 로드 가능 |

### 근본 원인
1. **잘못된 /Length 값**: Linearized PDF의 xref stream object에서 /Length가 실제 스트림 데이터보다 큼
   - 예: Length=455, 실제=21 (incremental update에서 이전 length를 복사한 것으로 추정)
   - `stream()` 함수가 `take(length)` 실행 시 바이트 부족으로 실패

2. **Malformed literal string**: `/ID` 배열의 binary string에 이스케이프 안된 `)` 포함
   - 예: `(j\xab\xf5TH\x83J{\x95*\x1e\xce\xd3)\xa5[\xa5[)` — `\xd3)` 에서 string 조기 종료
   - dictionary 파서가 실패하여 xref stream을 아예 읽지 못함

3. **%%EOF 검색 범위 부족**: 파일 끝에 garbage data가 있을 때 %%EOF가 512바이트 범위 밖에 위치
   - 예: 965,632바이트 파일에서 %%EOF가 끝에서 553바이트 전

## 수정 내용 (/Users/wonsup-mini/projects/lopdf)

### 1. `src/parser/mod.rs` — stream() 함수에 endstream fallback 추가
- Length로 `take(n)` 실패 시 `endstream` 마커를 직접 검색하여 스트림 데이터 추출
- `find_endstream_fallback()` 함수 추가: `\r\nendstream`, `\rendstream`, `\nendstream` 순서로 검색

### 2. `src/parser/mod.rs` — xref_and_trailer() 함수에 lenient fallback 추가
- `_indirect_object` 파싱 실패 시 `parse_xref_stream_object_lenient()` 호출
- 바이트 스캔으로 `>>` + `stream` 키워드를 찾아 dict와 stream data를 분리
- dict 파싱 실패 시 `parse_xref_dict_from_bytes()`로 필수 필드만 추출
  - /Size, /Root, /Info, /Prev, /W, /Index, /Filter, /DecodeParms, /Type

### 3. `src/parser/mod.rs` — parse_xref_dict_from_bytes() 참조 파싱 수정
- `/Root 2 0 R/Size` 같은 공백 없는 패턴 처리
- `"R"` 뒤에 `/`, `>`, `)` 가 바로 올 수 있도록 매칭 로직 수정

### 4. `src/reader.rs` — get_xref_start() 검색 범위 확대
- %%EOF 검색 범위: 512바이트 → 2048바이트

## 결과

### lopdf 로드 성공률
| 항목 | Before | After |
|------|--------|-------|
| 300개 실패 파일 | 191/300 | **300/300** |
| 500개 정상 파일 (regression) | 500/500 | **500/500** |
| lopdf 자체 테스트 | all pass | **all pass** |

### 텍스트 추출 결과 (300개 파일)
- **115개에서 텍스트 추출 성공** (Java 대비 avg 98.2% 유사도)
- 185개는 빈 출력 (도면/이미지 PDF — Java도 빈 출력인 것 179개)
- **6개는 Rust 빈 / Java 텍스트 있음** — pdfbox-text 레벨에서 추가 조사 필요

## 남은 이슈
6개 파일에서 lopdf 로드는 성공하지만 텍스트 추출이 빈 결과:
- 3.영진글로벌캠퍼스_파크골프실습장 (13K chars Java)
- 숙등역(F1~F26)수정 (27K chars Java)
- 학생복지센타5층 강의실 (8K chars Java)
- 1. 경영관 Center for AI Finance 전광판 (1K chars Java)
- 우유급식공고 (5K chars Java)
- 02.-(게시용)_물품구매 규격서 (3K chars Java)
