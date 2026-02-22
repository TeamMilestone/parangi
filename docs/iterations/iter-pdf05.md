# Iteration pdf05: GlyphList + 사전정의 인코딩 + DictionaryEncoding

## 변경사항

### encoding/mod.rs — Encoding 기본 타입
- `Encoding` 구조체: `code_to_name: HashMap<u16, String>`, `name_to_code: HashMap<String, u16>` 양방향 매핑
- 주요 메서드: `new()`, `add()`, `has_code()`, `get_name()`, `get_code()`, `name()`, `len()`, `iter()`
- Clone 가능 (DictionaryEncoding에서 base encoding 복사 시 필요)

### encoding/glyph_list.rs — Adobe Glyph List
- `GlyphList` 구조체: glyph name ↔ Unicode string 양방향 매핑
- `DEFAULT` static: glyphlist.txt (4300+ entries) + additional.txt (TeX 관련 130+ entries)
- `ZAPF_DINGBATS` static: zapfdingbats.txt (200+ entries)
- `to_unicode()` 변환 체인:
  1. 직접 조회
  2. `uniXXXX` 포맷 (정확히 7글자)
  3. `uniXXXXXXXX` 포맷 (4의 배수 hex digits, 복수 코드포인트)
  4. `uXXXX`~`uXXXXX` 포맷 (단일 코드포인트, 4-6 hex digits)
  5. 도트 접미사 제거 후 재조회 (예: "A.swash" → "A")

### encoding/predefined.rs — 사전정의 인코딩 6종
| 인코딩 | 엔트리 수 | 비고 |
|--------|-----------|------|
| WinAnsiEncoding | 172+ | 미매핑 코드(33-255)는 bullet |
| StandardEncoding | 149 | PDF 기본 인코딩 |
| MacRomanEncoding | 172 | |
| MacExpertEncoding | 34 (subset) | |
| SymbolEncoding | 113 (subset) | 그리스 문자 + 수학 기호 |
| ZapfDingbatsEncoding | 174 | |

- `get_encoding(name)`: 이름으로 사전정의 인코딩 조회 팩토리 함수

### encoding/dictionary.rs — DictionaryEncoding
- `build_encoding(doc, encoding_obj, is_symbolic)`: PDF /Encoding 항목으로 Encoding 구축
  - Name 객체 → 사전정의 인코딩 직접 반환
  - Reference 객체 → 역참조 후 재귀
  - Dictionary 객체 → BaseEncoding + Differences 처리
- `apply_differences()`: `/Differences` 배열 파싱 (정수=코드시작, Name=glyph 매핑, 자동 증가)
- 비심볼릭 폰트: 기본 base = StandardEncoding
- 심볼릭 폰트: 기본 base = 빈 인코딩 (BuiltIn)

### 데이터 파일
- `src/data/glyphlist.txt`: Adobe Glyph List (4327줄)
- `src/data/additional.txt`: PDFBox 추가 매핑 (TeX 관련, 153줄)
- `src/data/zapfdingbats.txt`: ITC Zapf Dingbats 전용 (249줄)

## 테스트 결과
```
Unit tests: 56 passed
Integration tests: 8 passed
Total: 64 passed, 0 failed
```

## 신규 테스트 (17개)
- GlyphList: default loaded, basic lookup, .notdef, uni format, u format, dot suffix, reverse lookup, zapf dingbats
- Predefined: win_ansi, standard, mac_roman, get_encoding
- DictionaryEncoding: name encoding, dict with differences, empty differences, no base (nonsymbolic), symbolic no base

## 다음 단계 (Iter pdf06)
- CMap 파서 (fontbox CMapParser 포팅)
- CMap → Unicode 매핑 (ToUnicodeCMap)
