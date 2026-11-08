# 05: 불변성

**What to build:** 한 번 발급한 링크는 어떤 경로로도 바뀌거나 사라지지 않는다. 서버는 변경 요청을 거부하고, 코드에 실수가 있어도 DB가 마지막으로 막으며, 재시작 후에도 링크가 그대로 이동한다.

**Blocked by:** 01 로컬에서 생성하고 이동하기

**Status:** ready-for-agent

- [x] `/<번호>`와 링크 하나를 가리키는 API 경로에 GET이 아닌 메서드(PUT, PATCH, DELETE 등)가 오면 405
- [x] 생성 요청 본문에 `url` 외의 필드(예: `number`)가 있으면 400
- [x] DB 트리거가 링크 테이블의 UPDATE, DELETE, TRUNCATE를 거부한다
- [x] 통합 테스트가 SQL로 UPDATE, DELETE, TRUNCATE를 직접 시도해 모두 거부되는지 확인한다
- [x] 통합 테스트가 같은 DB에 라우터를 새로 만들어도 이전 번호가 같은 원본 URL로 이동하는지 확인한다(재시작 시뮬레이션)
- [x] 405, 400 경우마다 HTTP 경계 통합 테스트가 있다

## Comments

- 구현: `0002_links_immutable.sql`(행 단위 BEFORE UPDATE/DELETE 트리거, 문 단위 BEFORE TRUNCATE 트리거), `CreateLinkRequest`에 `deny_unknown_fields`, 생성 핸들러가 모든 JSON 거절(미지의 필드 422 포함, 형식 오류, `url` 누락)을 한국어 메시지의 400 ErrorResponse로 변환.
- `GET /api/links/{번호}`를 추가해 링크 JSON(number, short_url, original_url) 또는 404 JSON을 돌려준다. 이 경로의 PUT/PATCH/DELETE는 axum이 `Allow` 헤더와 함께 405로 응답한다.
- 테스트: `backend/tests/immutability.rs` (6개). 재시작 시뮬레이션은 풀을 닫고 새 풀과 새 라우터를 같은 DB에 연결해 확인. 프로세스 재시작 자체는 실행하지 않았다.
