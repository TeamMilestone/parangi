# Iteration pdf06: CMap 파서

## 변경사항

### encoding/cmap.rs — CMap 데이터 구조
- `CMap` 구조체: 코드→유니코드, 코드→CID 매핑 저장
  - 코드 길이별(1~4바이트) 분리된 HashMap으로 효율적 조회
  - `char_to_unicode[4]`: 코드→유니코드 매핑
  - `code_to_cid[4]`: 코드→CID 직접 매핑
  - `cid_ranges: Vec<CidRange>`: 범위 기반 CID 매핑
  - `unicode_to_code`: 유니코드→코드 역방향 매핑
- `CodespaceRange`: 유효 코드 범위 정의 (다중 바이트 지원)
- `CidRange`: 연속 CID 매핑 범위
- `merge()`: 다른 CMap 매핑 병합 (usecmap 지원)

### encoding/cmap_parser.rs — CMap 파서
- `parse_cmap(data) -> CMap`: CMap 바이트 스트림 파싱
- 토크나이저: HexBytes(`<AABB>`), Integer, Name(`/name`), Operator(keyword), LiteralString(`(text)`)
- 섹션 파서:
  - `begincodespacerange`: 코드스페이스 범위 파싱
  - `beginbfchar`: 코드→유니코드 개별 매핑
  - `beginbfrange`: 코드→유니코드 범위 매핑 (순차 + 배열 형식)
  - `begincidchar`: 코드→CID 개별 매핑
  - `begincidrange`: 코드→CID 범위 매핑
- 메타데이터: `/CMapName`, `/WMode` 파싱
- UTF-16BE 디코딩: hex bytes → Unicode 문자열 변환

### 통합 테스트 추가
- `test_tounicode_cmap_parsing`: 실제 한국어 PDF에서 ToUnicode 스트림 추출 → CMap 파싱 검증

## 테스트 결과
```
Unit tests: 73 passed
Integration tests: 9 passed
Total: 82 passed, 0 failed
```

## 신규 테스트 (17개)
- CMap: new_cmap, unicode_mapping, cid_direct, cid_range, codespace_range, reverse_lookup, merge
- CMap Parser: simple_bfchar, bfrange_sequential, bfrange_array, cidrange, cidchar, codespace_range, wmode, hex_bytes_to_unicode, bytes_to_u32, comments
- Integration: tounicode_cmap_parsing (실제 PDF)

## 다음 단계 (Iter pdf07)
- Simple 폰트 (Type1, TrueType) 구현
- toUnicode 폴백 체인: ToUnicode CMap → Encoding + GlyphList
