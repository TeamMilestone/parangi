이터레이션 $ARGUMENTS 실행:

1. 구현 작업을 수행한다
2. `/Users/wonsup-mini/.cargo/bin/cargo build --release` 빌드 확인
3. `/Users/wonsup-mini/.cargo/bin/cargo test` 테스트 통과 확인
4. `docs/iterations/iter-$ARGUMENTS.md` 이터레이션 문서 작성 (변경사항, 테스트 결과, 다음 단계)
5. git add → commit → push (conventional commits 스타일)
6. `python3 ~/projects/kakamogo/send_msg.py --chat 460332433113926 "Iter $ARGUMENTS: 작업 요약"` 알림 발송

빌드 실패 시 수정 후 재빌드. 테스트 실패 시 수정 후 재실행.
