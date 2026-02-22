# Iteration pdf11: Form XObject (Do 오퍼레이터) + 중첩 스트림

## 변경사항

### stream/engine.rs — Form XObject 처리
- `op_do()`: Do 오퍼레이터 핸들러 — XObject 이름 → Form XObject 처리
- `extract_form_data()`: Form 스트림 디코딩 + 매트릭스 + 리소스 사전 추출 (불변 보로우)
- `process_form()`: 그래픽 상태 저장 → Form 매트릭스 적용 → 콘텐츠 스트림 처리 → 복원
- `load_resources()`: 리소스 딕셔너리에서 폰트 + XObject 참조 로드
- `load_fonts_from_resources()`: Resources/Font 딕셔너리에서 PdfFont 생성 (정적 메서드)
- `load_xobject_refs()`: Resources/XObject 딕셔너리에서 ObjectId 맵 생성
- `load_form_resources()`: Form 고유 리소스 로드 (없으면 부모 상속)
- `read_form_matrix()`: Form /Matrix 배열 → Matrix 변환
- 재귀 깊이 제한: 50 레벨 (무한 재귀 방지)

### 리소스 스코핑 전략
- Form에 자체 Resources가 있으면: swap-in (부모 리소스 저장)
- Form에 Resources가 없으면: 부모 리소스 상속 (swap 불필요)
- Clone 불필요: `std::mem::replace`로 효율적 스왑

### stream/graphics_state.rs — 전체 스택 저장/복원
- `save_full()` → `SavedGraphicsStack`: 전체 스택 + 현재 상태 스냅샷
- `restore_full(saved)`: 완전 복원 (q/Q 쌍이 아닌 전체 교체)
- Form XObject 내 모든 그래픽 상태 변경이 완전 격리됨

### Form XObject 처리 흐름
```
Do /Fm0
  ↓
xobject_refs["Fm0"] → ObjectId
  ↓
extract_form_data(oid)
  → Stream 디코딩
  → /Matrix 읽기
  → /Resources 로드 (폰트 + XObject)
  ↓
process_form(FormData)
  → 리소스 swap-in (자체 리소스가 있을 때)
  → 그래픽 스택 전체 저장
  → CTM.concatenate(Form.Matrix)
  → process_content(form_bytes) ← 재귀
  → 그래픽 스택 복원
  → 리소스 복원
```

## 테스트 결과
```
Unit tests: 95 passed
Integration tests: 10 passed
Total: 105 passed, 0 failed
```

## 통합 테스트 개선
- `test_stream_engine_real_pdf`: `set_fonts()` → `load_resources()` 변경
  - 폰트와 XObject 참조를 한번에 로드

## 다음 단계
- Iter pdf12: Marked Content (BMC/BDC/EMC) + ActualText
