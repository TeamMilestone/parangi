# Iteration pdf09: Type3 폰트 + 엣지 케이스

## 변경사항

### font/type3_font.rs — Type3 폰트
- `Type3Font`: 사용자 정의 폰트 (글리프가 PDF 콘텐츠 스트림으로 정의됨)
- toUnicode 폴백 체인:
  1. ToUnicode CMap (최우선)
  2. Encoding → glyph name → GlyphList → Unicode
- Widths 배열 + FirstChar 기반 너비 조회
- 텍스트 추출 관점에서 SimpleFont과 유사하게 동작

### font/mod.rs — PdfFont 최종 구조
- `PdfFont` enum: `Simple`, `Type0`, `Type3` — 모든 폰트 타입 지원 완료
- `Type3Stub` 제거 → `Type3(Type3Font)` 교체
- `is_stub()`: 항상 false (모든 폰트가 실제 구현)
- `base_font_name()`: 모든 폰트 타입에서 이름 조회
- Unknown subtype → SimpleFont으로 폴백 처리

### 엣지 케이스 처리
| 케이스 | 처리 |
|--------|------|
| Unknown font subtype | SimpleFont으로 폴백 |
| Missing BaseFont | 빈 문자열 |
| Missing Encoding | StandardEncoding 기본값 |
| Missing ToUnicode | 인코딩 기반 디코딩 폴백 |
| Missing Widths | 빈 배열, MissingWidth 사용 |
| Missing FontDescriptor | is_symbolic=false 기본값 |

## Phase 2 완료 요약

Phase 2 (Iter 5-9)에서 구현된 전체 폰트 & 인코딩 스택:

```
PDF Font Dictionary
    ├── Type1/TrueType/MMType1 → SimpleFont
    │     └── toUnicode: ToUnicode CMap → Encoding + GlyphList
    ├── Type0 → Type0Font
    │     └── toUnicode: ToUnicode CMap → Encoding CMap + UCS2 CMap → Identity
    │     └── CMap Manager: 92개 사전정의 CMap (한국어/일본어/중국어)
    └── Type3 → Type3Font
          └── toUnicode: ToUnicode CMap → Encoding + GlyphList
```

## 테스트 결과
```
Unit tests: 88 passed
Integration tests: 10 passed
Total: 98 passed, 0 failed
```

## 신규 테스트 (1개)
- Type3Font: type3_font_creation (생성 + 인코딩 디코딩 + 너비 조회)

## 다음 단계
- E2E 테스트: 샘플 50개 PDF로 1차 추출 테스트 시작
- Iter pdf10: showText/showGlyph 포팅 → TextPosition 생성
