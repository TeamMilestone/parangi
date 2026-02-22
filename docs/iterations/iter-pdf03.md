# Iteration pdf03: Matrix, GraphicsState 스택, TextState

## 변경사항

### stream/matrix.rs — 3x3 아핀 변환 행렬
- PDFBox `Matrix` 클래스 포팅
- Row-major 저장: `[a, b, 0, c, d, 0, tx, ty, 1]`
- 핵심 연산: multiply, concatenate, translate, scale, transform_point
- 팩토리: scale_instance, translate_instance, rotate_instance
- scaling_factor_x/y (열 벡터 크기)
- 9개 단위 테스트

### stream/text_state.rs — 텍스트 상태 파라미터
- PDFBox `PDTextState` 클래스 포팅
- 필드: character_spacing, word_spacing, horizontal_scaling (0-100%), leading, font_name, font_size, rendering_mode, rise, knockout
- `RenderingMode` enum (8종) + is_fill/is_stroke/is_clip 판별
- 4개 단위 테스트

### stream/graphics_state.rs — 그래픽스 상태 스택
- PDFBox `PDGraphicsState` 클래스 포팅 (텍스트 추출에 필요한 필드만)
- `GraphicsState`: ctm, text_state, text_matrix, text_line_matrix
- `GraphicsStateStack`: save (q) / restore (Q) 연산
- begin_text (BT) / end_text (ET) 지원
- 5개 단위 테스트

### stream/mod.rs
- matrix, text_state, graphics_state 모듈 등록

## 테스트 결과
```
Unit tests: 31 passed
Integration tests: 7 passed
Total: 38 passed, 0 failed
```

## 발견된 이슈
- PDF 행렬은 row-vector 규약 사용. `multiply(A, B)`에서 점 p는 `p × A × B` 순서로 변환됨.
- `concatenate(M)` = `M × self` (pre-multiplication)

## 다음 단계 (Iter pdf04)
- 콘텐츠 스트림 파서 구현
- 텍스트 오퍼레이터 디스패치 (BT/ET/Tf/Td/Tm/Tj/TJ 등)
