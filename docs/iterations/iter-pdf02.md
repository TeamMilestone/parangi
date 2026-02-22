# Iteration pdf02: COS 헬퍼 레이어 + PdfResources + 스트림 디코딩 검증

## 변경사항

### cos_helpers.rs 확장
- `dict_get_number_array()` — 배열에서 f32 벡터 추출
- `get_stream_data()` — 스트림 객체 디코딩 (자동 deref + 해제 압축)
- `obj_to_f32()` — Integer/Real → f32 변환 (독립 함수)
- `name_to_string()` — PDF 이름 #XX 디코딩
- `decode_text_string()` — UTF-16BE/UTF-8 BOM/PDFDocEncoding 감지 및 디코딩
- `ObjectExt::is_null()` — Null 체크
- COS 헬퍼 단위 테스트 9개 추가

### resources.rs 확장
- `color_space_dict()` — ColorSpace 서브 딕셔너리 조회
- `has_font()` — 폰트 존재 여부 확인

### page.rs 수정
- `get_box()`: lopdf `as_f32()` → `obj_to_f32()` 사용 (Integer → f32 변환 버그 수정)

### 통합 테스트 추가
- `tests/integration_test.rs` — 실제 한국어 PDF로 7개 통합 테스트
  - PDF 열기, 페이지 속성, 리소스, 콘텐츠 스트림, 기본 텍스트 추출, 폰트 딕셔너리 접근

### 테스트 Fixture
- `tests/fixtures/sample_korean.pdf` — 나라장터 입찰공고 PDF (212KB)

### 파일명 규칙 변경
- 이터레이션 문서: `iter-pdf01.md`, `iter-pdf02.md` ... 형식으로 통일

## 테스트 결과
```
Unit tests: 13 passed
Integration tests: 7 passed
Total: 20 passed, 0 failed
```

## 발견된 이슈 및 수정
- lopdf의 `as_f32()`는 Integer→f32 변환을 지원하지 않음 → 자체 `obj_to_f32()` 함수 사용으로 수정

## 다음 단계 (Iter pdf03)
- Matrix(3x3 아핀 변환) 구현
- GraphicsState 스택 구현
- TextState (폰트, 크기, 문자/단어 간격 등) 구현
