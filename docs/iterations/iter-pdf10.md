# Iteration pdf10: showText/showGlyph → TextPosition 생성

## 변경사항

### text/text_position.rs — TextPosition 구조체 (NEW)
- `TextPosition`: 단일 글리프/문자의 위치·크기·유니코드 정보
- 필드: unicode, char_codes, text_matrix(TRM), end_x/y, max_height, individual_width, space_width, font_size, font_size_in_pt, page_rotation/width/height
- 접근자: `x()`, `y()`, `x_scale()`, `y_scale()`, `direction()` (0/90/180/270)
- 방향 보정 좌표: `x_dir_adj()`, `y_dir_adj()`, `width_dir_adj()`, `height_dir_adj()`

### font/mod.rs — PdfFont::read_code
- `read_code(data, offset) -> (code, bytes_consumed)`: 가변 길이 문자 코드 읽기
- Simple/Type3: 항상 1바이트
- Type0: 인코딩 CMap codespace range에 따라 1-4바이트

### font/type0_font.rs — Type0Font::read_code
- 인코딩 CMap의 codespace range를 순차 체크 (min → max byte length)
- 매칭되는 codespace가 없으면 1바이트 폴백

### stream/engine.rs — StreamEngine 대규모 리팩토링
- `RawTextSegment` 제거 → `TextPosition` 교체
- 새 필드: `fonts: HashMap<Vec<u8>, PdfFont>`, `text_positions: Vec<TextPosition>`, 페이지 정보
- `set_fonts()`: 사전 로드된 폰트 설정
- `set_page_info()`: 페이지 rotation/width/height 설정
- `show_text()`: PDFStreamEngine.showText() + LegacyPDFStreamEngine.showGlyph() 통합 포팅
  - 바이트 → 문자 코드 → 유니코드 디코딩 (글리프 단위)
  - TRM(Text Rendering Matrix) 계산: [fs×Hs 0 0; 0 fs 0; 0 rise 1] × Tm × CTM
  - 변위 계산: width/1000 × fontSize × Hs → endX/endY
  - 텍스트 매트릭스 진행: (displacement × fontSize + charSpacing + wordSpacing) × Hs
  - Space width 계산: font.get_width(32), 폴백 체인
  - 워드 스페이싱: code 32 (single-byte)일 때만 적용

### stream/matrix.rs
- `Matrix`에 `Copy` derive 추가 (9×f32 = 36바이트, Copy 적합)

### text/mod.rs
- text_position 모듈 선언 및 TextPosition re-export

## 처리 흐름

```
Content Stream Operator (Tj/TJ/'/"")
    ↓
show_text(bytes)
    ↓
[for each character]:
    ↓
  font.read_code(bytes, offset) → (code, code_length)
    ↓
  compute_text_rendering_matrix() → TRM
    ↓
  font.get_width(code) / 1000.0 → displacement
    ↓
  displacement × fontSize × Hs → endX/endY (visual extent)
    ↓
  font.to_unicode(code) → unicode string
    ↓
  TextPosition { unicode, text_matrix, end_x/y, ... }
    ↓
  text_matrix.translate(total_tx, 0.0) → advance to next glyph
```

## 테스트 결과
```
Unit tests: 95 passed
Integration tests: 10 passed
Total: 105 passed, 0 failed
```

## 신규 테스트 (7개)
- TextPosition: position_accessors, direction_horizontal, direction_rotated, scaling_factors
- StreamEngine: show_text_without_font, show_text_with_font, tj_array_with_font, text_matrix_advances, word_spacing

## 실제 한국어 PDF 추출 결과
```
Text positions extracted: 1180
Positions with unicode: 1180/1180 (100%)
샘플: '우편번호:46508', '주소:부산...'
```

## 다음 단계
- Iter pdf11: Form XObject (Do 오퍼레이터) + 중첩 스트림 처리
- Iter pdf12: Marked Content (BMC/BDC/EMC) + ActualText
