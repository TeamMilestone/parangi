# Iteration pdf04: 콘텐츠 스트림 파서 + 텍스트 오퍼레이터 디스패치

## 변경사항

### stream/operators.rs — 오퍼레이터 이름 상수
- PDF 스펙의 텍스트 관련 오퍼레이터 33종 정의
- BT/ET, Td/TD/Tm/T*, Tc/Tw/Tz/TL/Tf/Tr/Ts, Tj/TJ/'/", q/Q/cm, Do, gs, BMC/BDC/EMC

### stream/engine.rs — StreamEngine 핵심 구현
- `StreamEngine` 구조체: 콘텐츠 스트림 처리 엔진
- `process_content()`: lopdf Content 디코딩 → 오퍼레이터 디스패치
- `RawTextSegment`: 텍스트 바이트 + 폰트명 + 폰트크기 + TRM 정보

#### 구현된 오퍼레이터 핸들러 (22종):
| 카테고리 | 오퍼레이터 |
|---------|-----------|
| 그래픽스 상태 | q, Q, cm, gs |
| 텍스트 구분 | BT, ET |
| 텍스트 상태 | Tc, Tw, Tz, TL, Tf, Tr, Ts |
| 텍스트 위치 | Td, TD, Tm, T* |
| 텍스트 표시 | Tj, TJ, ', " |

#### 복합 오퍼레이터 (PDFBox과 동일한 위임 패턴):
- TD → TL + Td
- T* → Td(0, -leading)
- ' → T* + Tj
- " → Tw + Tc + T* + Tj

#### TRM (Text Rendering Matrix) 계산:
- `compute_text_rendering_matrix()`: [fontSize×Hs, 0, 0, fontSize, 0, rise] × Tm × CTM

### 통합 테스트 추가
- `test_stream_engine_real_pdf`: 실제 한국어 PDF에서 StreamEngine으로 세그먼트 추출 검증

## 테스트 결과
```
Unit tests: 39 passed
Integration tests: 8 passed
Total: 47 passed, 0 failed
```

## 다음 단계 (Iter pdf05)
- GlyphList + 사전정의 인코딩 (WinAnsi, Standard, Mac 등)
- DictionaryEncoding 구현
