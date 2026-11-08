# 01: 로컬에서 생성하고 이동하기 (최소 경로)

**What to build:** 로컬에서 생성 화면을 열고 원본 URL을 입력해 생성 버튼을 누르면 단축 주소와 복사 버튼이 보이고, 그 단축 주소를 열면 302로 원본 URL에 이동한다. 이후 모든 티켓이 올라탈 워크스페이스(`shared`/`backend`/`web`, ADR 0001), 링크 테이블, HTTP 통합 테스트 틀을 함께 만든다. 이 티켓의 검증은 `http`/`https` 스킴 확인 정도로 최소화한다. 전체 규칙은 02에서 다룬다.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] Rust 툴체인과 Dioxus CLI(`dx`)가 설치되어 있고, 워크스페이스 전체가 빌드된다
- [x] Docker로 Postgres를 띄우는 로컬 개발 방법이 있다
- [x] 백엔드가 시작할 때 마이그레이션으로 링크 테이블을 만든다. 번호는 `BIGINT GENERATED ALWAYS AS IDENTITY` 기본키, 원본 URL은 UNIQUE, 생성 시각 포함
- [x] 생성 API에 원본 URL을 POST하면 201과 함께 번호와 단축 주소를 돌려준다. 요청·응답 타입은 `shared`에 있다
- [x] `/<번호>` GET은 302와 저장된 원본 URL을 그대로 담은 `Location`을 돌려준다(경로·쿼리·프래그먼트 유지)
- [x] Dioxus 화면에서 원본 URL을 입력하고 생성하면 입력창 아래에 단축 주소가 보인다
- [x] 복사 버튼을 누르면 단축 주소가 클립보드에 복사되고 복사 완료 표시가 보인다
- [x] 로컬 개발 시 화면이 `/api/*` 호출을 백엔드로 넘기도록 설정되어 있다
- [x] axum 라우터에 직접 요청을 보내고 Docker Postgres를 쓰는 통합 테스트 틀이 있고, 생성→이동 경로를 검증하는 테스트가 통과한다

## Comments

- Versions: dioxus 0.7.10, dx 0.7.10, axum 0.8, sqlx 0.8, Rust 1.99.1 (was already installed; added wasm32 target).
- Tests use testcontainers (one Postgres container per test binary, fresh database per test); the Docker daemon must be running.
- Local dev DB: `docker compose up -d` exposes Postgres on host port 5434 (5432/5433 were reported in use by Docker Desktop on this machine).
- `dx serve` proxies only `/api/` to `localhost:8080`; numeric paths are not proxied (dev proxy is prefix-based). Locally `short_url` points to the backend (`http://localhost:8080/<n>`), so the redirect works directly.
- Non-http(s) rejection is a simple prefix check; unknown numbers return a bare 404 and non-numeric numbers also 404 (pages/strict parsing left for 04). Duplicate URLs currently hit the UNIQUE constraint and return 500 (03).
- Copy button verified in the embedded browser only with a stubbed `navigator.clipboard.writeText` (native write permission was denied there); the 복사됨 indicator appeared.
- Tests were written together with the implementation rather than strictly red-first.
