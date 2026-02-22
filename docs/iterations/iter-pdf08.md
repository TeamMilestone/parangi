# Iteration pdf08: Type0 + CIDFont (CJK 핵심)

## 변경사항

### encoding/cmap_manager.rs — 사전정의 CMap 매니저
- `get_predefined_cmap(name)`: 92개 사전정의 CMap 로드 + 캐싱
- 빌드 스크립트 (`build.rs`): `src/data/cmap/` 디렉토리의 모든 파일을 `include_str!`로 컴파일타임 임베딩
- `Mutex<HashMap>` 기반 lazy 캐시: 첫 접근 시 파싱, 이후 캐시에서 반환
- `has_predefined_cmap()`, `identity_h()` 헬퍼

### font/type0_font.rs — Type0 합성 폰트
- `Type0Font`: CJK 텍스트 지원의 핵심 구조체
- **toUnicode 폴백 체인** (PDFBox와 동일):
  1. **ToUnicode CMap**: 코드→유니코드 직접 변환 (1~4바이트 시도)
  2. **Encoding CMap + UCS2 CMap**: 코드→CID (Encoding), CID→유니코드 (UCS2)
  3. **Identity fallback**: 코드를 유니코드 코드포인트로 직접 해석

#### 초기화 과정:
- `read_encoding_cmap()`: /Encoding 항목에서 CMap 읽기 (Name→사전정의, Stream→임베디드)
- `get_descendant_dict()`: /DescendantFonts 배열에서 CIDFont 딕셔너리 추출
- `read_cid_system_info()`: Registry/Ordering으로 CJK 여부 판별
  - Adobe-Korea1, Adobe-GB1, Adobe-CNS1, Adobe-Japan1 감지
- `fetch_ucs2_cmap()`: `{Registry}-{Ordering}-UCS2` CMap 자동 로드
- `read_widths()`: W 배열 파싱 (두 가지 형식)
  - `[cid [w1 w2 ...]]`: CID별 개별 너비
  - `[cid_start cid_end width]`: CID 범위 너비
- `read_default_width()`: DW (기본 너비, 없으면 1000)

### font/mod.rs — PdfFont 업데이트
- `Type0Stub` → `Type0(Type0Font)` 변경
- `is_stub()`: Type3Stub만 스텁으로 남음

### data/cmap/ — 사전정의 CMap 92개
- Identity-H/V, Adobe-Korea1-UCS2, UniKS-UCS2-H/V
- Adobe-GB1-UCS2, Adobe-CNS1-UCS2, Adobe-Japan1-UCS2
- KSCms-UHC-H/V, KSC-EUC-H/V 등 한국어 관련 CMap
- 총 3.3MB

## CMap 지원 현황
| CMap 그룹 | 용도 | 파일 수 |
|-----------|------|---------|
| Identity-H/V | Identity 매핑 | 2 |
| Adobe-Korea1-* | 한국어 CID↔Unicode | 4 |
| KSC-*, UniKS-* | 한국어 인코딩 | 10 |
| Adobe-Japan1-* | 일본어 | 10 |
| Adobe-GB1-*, UniGB-* | 중국어 간체 | 12 |
| Adobe-CNS1-*, UniCNS-* | 중국어 번체 | 12 |
| 기타 | RKSJ, EUC, B5 등 | 42 |

## 테스트 결과
```
Unit tests: 87 passed
Integration tests: 10 passed
Total: 97 passed, 0 failed
```

## 신규 테스트 (8개)
- CMap Manager: identity_h, korea_ucs2, nonexistent, cache_works, has_predefined
- Type0Font: code_to_cid_identity, to_unicode_via_tounicode_cmap, width_lookup

## 다음 단계 (Iter pdf09)
- Type3 폰트 + 엣지 케이스 (missing font, subset prefix 등)
- 그 후 E2E 테스트 50개 PDF로 1차 검증
