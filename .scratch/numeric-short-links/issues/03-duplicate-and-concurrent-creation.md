# 03: 같은 원본 URL 재입력과 동시 생성

**What to build:** 이미 등록된 원본 URL을 다시 입력하면 새 번호를 만들지 않고 기존 단축 주소를 돌려주며, 화면은 "이미 만들어진 주소"라는 안내를 덧붙인다. 동시에 생성 요청이 몰려도 번호가 겹치거나 기존 링크가 덮어써지지 않는다.

**Blocked by:** 01 로컬에서 생성하고 이동하기

**Status:** ready-for-agent

- [x] 같은 원본 URL로 다시 생성하면 200과 기존 번호, 기존 링크라는 표시를 돌려준다
- [x] 삽입 충돌 시 기존 행을 바꾸지 않고 기존 번호를 조회해 돌려준다
- [x] 화면에서 기존 링크를 돌려받으면 "이미 만들어진 주소" 안내가 보인다
- [x] 서로 다른 원본 URL 여러 개를 동시에 생성하는 통합 테스트에서 번호가 모두 다르고, 각 번호가 자기 원본 URL로 이동한다
- [x] 같은 원본 URL을 동시에 생성하는 통합 테스트에서 번호가 하나만 생긴다
- [x] 번호 사이 빈자리는 허용된다(테스트가 연속 번호를 가정하지 않는다)

## Comments

- 구현: `INSERT ... ON CONFLICT (original_url) DO NOTHING RETURNING number`, 행이 안 돌아오면 `SELECT number`(기존 행은 UPDATE하지 않음). 신규는 201/`existing:false`, 기존은 200/`existing:true`. 동시 충돌은 ON CONFLICT가 상대 트랜잭션 종료를 기다린 뒤 READ COMMITTED SELECT가 커밋된 행을 보므로 오류 창이 없고, 방어용으로 최대 3회 재시도한다. 웹은 `existing`일 때 "이미 만들어진 주소예요"를 표시한다. 테스트: `backend/tests/duplicates.rs`.
