# lopdf-iter02: 빈 텍스트 추출 6건 해결 (ASCIIHexDecode + font fallback)

## 날짜
2026-02-25

## 목표
lopdf-iter01에서 로드 성공했으나 텍스트 추출이 빈 6개 파일 해결

## 분석 — 3가지 독립적인 원인

### 원인 1: lopdf Content::decode()가 0 operations 반환 (1개 파일)
**파일**: 우유급식공고.pdf

Content stream이 `% CANON_PFINF_TYPE0_TEXTON\n` 주석으로 시작하는데,
lopdf의 `operation()` 파서에서 주석 파싱 후 남은 `\n`을 operator 앞에서 소비하지 못함.

```
operation = preceded(many0(comment), alt((inline_image, terminated(pair(many0(operand), operator), content_space))))
```

`many0(comment)`가 주석을 소비 → 남은 `\n` → `many0(operand)` 실패(whitespace 롤백) →
`operator`에서 `\n` 파싱 불가 → `many0(operation)` 0건 반환 → `strip_nom`이 나머지 폐기

**수정**: `operator` 앞에 `content_space` 추가
```
preceded(content_space, operator)
```

### 원인 2: ASCIIHexDecode 필터 미지원 (3개 파일)
**파일**: 설계도.pdf, 숙등역.pdf, 학생복지센타.pdf

Content stream의 `/Filter [/ASCIIHexDecode /FlateDecode]` 체인에서
lopdf가 ASCIIHexDecode를 지원하지 않아 `Unimplemented("decompression algorithms")` 에러.
pdfbox-text의 fallback이 raw hex-encoded 데이터를 그대로 사용하여 파싱 실패.

**수정**: lopdf `object.rs`에 `decode_ascii_hex()` 구현
- Hex 문자(0-9, A-F) → 바이너리 변환
- `>` EOD 마커 처리
- 홀수 자릿수 시 implicit 0 추가

### 원인 3: Font encoding fallback 미비 (5개 파일)

#### 3a. Identity-H/V CID 폰트 (3개 파일)
**파일**: 설계도.pdf, 숙등역.pdf, 학생복지센타.pdf (원인 2와 중복)

Type0 폰트가 Identity-H encoding + Adobe-Identity CIDSystemInfo를 사용.
ToUnicode CMap 없고, UCS2 CMap도 `Adobe-Identity-UCS2` (존재 안 함).
기존 Tier 3 fallback은 `to_unicode_cmap.is_some()` 조건이라 적용 안 됨.

**수정**: Tier 4 추가 — encoding CMap name이 "Identity"로 시작하면 CID를 직접 Unicode로 사용
```rust
if self.encoding_cmap.name.starts_with("Identity") {
    let cid = self.code_to_cid(code);
    char::from_u32(cid)
}
```

#### 3b. Type3 폰트 (2개 파일)
**파일**: 경영관 전광판 공문.pdf, 물품구매 규격서.pdf

Type3 폰트의 glyph name이 숫자("0", "1", "2" 등)로, Adobe Glyph List에 없음.
Java PDFBox도 의미 있는 텍스트를 추출하지 못함 (garbled output).

**수정**: Tier 3 fallback — code를 직접 Unicode code point로 사용 (Java 동작 매칭)

### 원인 4: Form XObject 해제 실패 (잠재적 영향)
`extract_form_data()`에서 `decompressed_content()` 에러를 `?`로 전파하여
Form XObject 텍스트가 통째로 사라지는 문제. `get_stream_data()`의 기존 fallback 패턴 적용.

## 수정 내용

### lopdf (../lopdf)
1. `src/object.rs` — `decode_ascii_hex()` 함수 추가, `decompressed_content()`에서 호출
2. `src/parser/mod.rs` — `operation()` 파서에 `preceded(content_space, operator)` 추가
3. `src/parser/mod.rs` — endstream fallback을 xref stream (/Type /XRef)에만 적용

### pdfbox-text
1. `font/type0_font.rs` — Identity-H/V encoding Tier 4 fallback 추가
2. `font/type3_font.rs` — code→Unicode Tier 3 fallback 추가
3. `stream/engine.rs` — Form XObject decompression fallback 추가

## 결과

### 6개 파일 텍스트 추출
| 파일 | Java | Rust | 비율 |
|------|------|------|------|
| 설계도.pdf | 13,011 | 12,582 | 96.7% |
| 숙등역.pdf | 27,164 | 26,029 | 95.8% |
| 학생복지센타.pdf | 8,356 | 7,594 | 90.9% |
| 경영관 전광판.pdf | 1,043 | 848 | 81.3% |
| 우유급식공고.pdf | 4,975 | 5,614 | 112.8% |
| 물품구매 규격서.pdf | 3,072 | 1,848 | 60.2% |

### Regression 테스트
- 500개 기존 PDF: **484/500 통과** (16개 "실패"는 Java 참조도 non-whitespace 0문자)
- lopdf 자체 테스트: all pass
- pdfbox-text 유닛 테스트: 12/12 pass
