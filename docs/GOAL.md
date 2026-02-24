# PDFBox Rust Port — Goals

## Goal 1: lopdf xref 파싱 수정으로 실패 파일 0건 달성
- **상태**: 완료 (lopdf 로드 기준) / 진행 중 (텍스트 추출 6건 미해결)
- **배경**: lopdf 0.39의 xref 파싱 한계로 ~300개 PDF 로드 실패
- **결과**: /Users/wonsup-mini/projects/lopdf 수정으로 **300/300 전부 로드 성공**
  - stream() endstream fallback 추가 (잘못된 /Length 처리)
  - xref_and_trailer() lenient fallback 추가 (malformed dict 처리)
  - get_xref_start() %%EOF 검색 범위 512 → 2048 확대
- **텍스트 추출**: 115/121개 성공 (avg 98.2% similarity)
  - 6개 파일: lopdf 로드 성공이지만 텍스트 빈 출력 (pdfbox-text 레벨 이슈)
- **상세**: docs/iterations/lopdf-iter01.md

## Goal 2 (완료): 99.7% 문자 유사도 달성
- 20,217개 PDF 대상, Java PDFBox 대비 평균 99.7% char similarity
- 96.8%가 99% 이상 일치
