# lopdf Iteration 3: LocatedSpan take_from(0) 최적화

## 날짜
2026-02-26

## 문제 발견 과정

### 출발점
lopdf-iter02에서 lazy stream loading을 구현했지만 parallel_parse가 16,706ms로 여전히 높음.
ObjStm 처리 측정 결과: decomp 32ms + parse 188ms = 220ms에 불과.
하지만 ALL-SKIP baseline은 3,431ms.

### 세밀한 프로파일링 추가
`read_object()` 호출 시간을 별도 측정:
```
parallel_parse:   18,290ms
  read_object:    20,302ms  (73,980 Normal entries)
  objstm_block:     193ms
```
read_object가 병목의 99%!

### 근본 원인 발견
`nom_locate::LocatedSpan::slice_by()` 구현:
```rust
let consumed = self.fragment.take(consumed_len);
let consumed_as_bytes = consumed.as_bytes();
let iter = Memchr::new(b'\n', consumed_as_bytes);  // ← newline 스캔!
let number_of_lines = iter.count() as u32;
```

`indirect_object()` 함수에서:
```rust
let (id, mut object) = _indirect_object(
    input.take_from(offset),  // ← 여기서 offset 바이트를 모두 스캔!
    ...
);
```

`input`은 전체 파일 버퍼의 LocatedSpan. `take_from(offset)`은 0..offset까지
newline을 세기 위해 `offset` 바이트를 전부 스캔.

73,980 entries × 평균 파일 오프셋 수 MB = **수백 GB 스캔** = O(N²) 패턴!

## 수정

### lopdf/src/reader.rs: read_object() 최적화

Before:
```rust
fn read_object(&self, offset: usize, ...) {
    parser::indirect_object(
        ParserInput::new_extra(self.buffer, "indirect object"),
        offset,  // take_from(offset) = O(offset) scan
        ...
    )
}
```

After:
```rust
fn read_object(&self, offset: usize, ...) {
    // 미리 슬라이스 → take_from(0) = O(1) (consumed_len=0, 즉시 반환)
    let sliced = &self.buffer[offset..];
    let (object_id, mut object) = parser::indirect_object(
        ParserInput::new_extra(sliced, "indirect object"),
        0,  // take_from(0) is O(1)
        ...
    )?;
    // stream.start_position을 파일 절대 위치로 보정
    if let Object::Stream(ref mut stream) = object {
        stream.start_position = stream.start_position
            .and_then(|sp| sp.checked_add(offset));
    }
    Ok((object_id, object))
}
```

핵심: `take_from(0)` → `consumed_len = self.fragment.offset(&next_fragment) = 0` → `slice_by()` 즉시 반환 (newline 스캔 없음)

## 결과

### 성능 비교

| 지표 | Before (iter02) | After (iter03) | 개선 |
|------|----------------|----------------|------|
| Wall clock (200 PDF) | 2.421s | **0.916s** | **2.64배** |
| read_object CPU | 20,302ms | 552ms | **36.8배** |
| parallel_parse CPU | 16,706ms | 731ms | **22.8배** |
| doc_open CPU | 17,205ms | 1,015ms | **17배** |
| 총 CPU | 26,135ms | 9,167ms | **2.85배** |

### 새로운 병목 (lopdf 최적화 후)
```
doc_open (lopdf parse):      1,015ms  (11.1%)
content_bytes (decomp):      2,135ms  (23.3%)
load_resources (fonts):        285ms   (3.1%)
process_content (eng):       4,818ms  (52.6%)  ← 새 병목
assemble_text (strip):         913ms  (10.0%)
```

lopdf의 비중이 67% → 11%로 감소.

## 교훈

- `nom_locate::LocatedSpan`은 위치 추적을 위해 슬라이싱 시 newline 스캔 수행
- `take_from(offset)`은 O(offset), `take_from(0)`은 O(1)
- 전체 파일 버퍼 span을 매번 생성하고 `take_from(offset)`하는 패턴은 O(N×avg_offset)
- 해결책: 파서에 전달하기 전 버퍼를 미리 슬라이싱
- 스트림 start_position은 절대 오프셋으로 보정 필요 (backing buffer 참조 시 사용)
