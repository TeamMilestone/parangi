# PDFBox Rust Port — Goals

## Goal 1: lopdf 수정으로 실패 파일 0건 달성
- **상태**: 완료
- **배경**: lopdf 0.39의 xref 파싱 한계로 ~300개 PDF 로드 실패, 6개 텍스트 빈 출력
- **결과**:
  - /Users/wonsup-mini/projects/lopdf 수정으로 **300/300 전부 로드 성공**
  - **6/6 빈 텍스트 파일 전부 해결** (lopdf + pdfbox-text 수정)
- **lopdf 수정 요약**:
  - stream() endstream fallback (xref stream 전용)
  - xref_and_trailer() lenient fallback (malformed dict 처리)
  - get_xref_start() %%EOF 검색 범위 512 → 2048 확대
  - ASCIIHexDecode 필터 지원 추가
  - Content::decode() 주석 후 공백 처리 수정
- **pdfbox-text 수정 요약**:
  - Form XObject decompression fallback
  - Identity-H/V CID 폰트 Unicode fallback
  - Type3 폰트 code→Unicode fallback
- **텍스트 추출**: 121/121개 성공 (Java 텍스트 있는 파일 기준)
- **상세**: docs/iterations/lopdf-iter01.md, docs/iterations/lopdf-iter02.md

## Goal 2 (완료): 99.7% 문자 유사도 달성
- 20,217개 PDF 대상, Java PDFBox 대비 평균 99.7% char similarity
- 96.8%가 99% 이상 일치
