# Iteration pdf07: Simple 폰트 (Type1, TrueType)

## 변경사항

### font/mod.rs — PdfFont 열거형
- `PdfFont` enum: `Simple(SimpleFont)`, `Type0Stub`, `Type3Stub`
- `from_dict(doc, font_dict, oid)`: 폰트 딕셔너리에서 PdfFont 생성
  - Type1/TrueType/MMType1 → SimpleFont
  - Type0 → Type0Stub (pdf08에서 구현)
  - Type3 → Type3Stub (pdf09에서 구현)
- `to_unicode(code)`: 문자 코드 → 유니코드 변환 위임
- `get_width(code)`: 문자 너비 조회 위임

### font/simple_font.rs — SimpleFont 구조체
- `SimpleFont`: 단일 바이트 폰트 (Type1, TrueType, MMType1)
- **toUnicode 폴백 체인** (PDFBox와 동일):
  1. **ToUnicode CMap** (최우선): 1바이트 + 2바이트 코드 모두 시도
  2. **Encoding → GlyphList**: 인코딩에서 glyph name 조회 → GlyphList로 유니코드 변환

#### 인코딩 해석 (`read_encoding`):
- ZapfDingbats/Symbol: 전용 사전정의 인코딩 사용
- 명시적 /Encoding 항목: `build_encoding()` 호출 (Name 또는 Dictionary)
- 없으면: StandardEncoding 기본값

#### 추가 기능:
- `read_is_symbolic()`: FontDescriptor Flags 비트 3으로 심볼릭 여부 판별
- `read_to_unicode()`: ToUnicode 스트림 → CMap 파싱
- `read_widths()`: /Widths 배열 + /FirstChar 기반 너비 조회
- `read_missing_width()`: FontDescriptor의 /MissingWidth
- ZapfDingbats 전용 GlyphList 자동 선택

## 테스트 결과
```
Unit tests: 79 passed
Integration tests: 10 passed
Total: 89 passed, 0 failed
```

## 신규 테스트 (7개)
- SimpleFont: creation, to_unicode_via_encoding, get_width, missing_width, notdef, standard_encoding_default
- Integration: font_creation_and_decode (실제 PDF에서 폰트 생성 + 문자 디코딩)

## 다음 단계 (Iter pdf08)
- **Type0 + CIDFont (CJK 핵심)**: 한국어 PDF 지원의 핵심
- DescendantFonts → CIDFont 해석
- ToUnicode CMap 2바이트 코드 디코딩
