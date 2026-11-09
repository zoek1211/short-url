# 숫자형 단축 URL 서비스

## 배포 URL

- 서비스 URL: https://s.zoek.workers.dev
- 테스트 단축 URL: https://s.zoek.workers.dev/4
- 테스트 원본 URL: https://github.com/zoek1211/short-url

## 소개

로그인 없이 원본 URL을 입력하면 `/123`처럼 숫자로만 된 단축 주소를 만들어 주는 서비스입니다. 한 번 발급한 링크는 수정·삭제·만료되지 않고, 번호는 다른 원본 URL에 다시 쓰이지 않습니다. 화면 오른쪽 위 버튼으로 한국어와 영어를 바꿀 수 있습니다.

English version: [English](#english)

용어는 [GLOSSARY.md](GLOSSARY.md)를 따릅니다: **원본 URL**(줄이려는 주소), **번호**(링크 하나를 가리키는 양의 정수), **단축 주소**(공개 주소 + 번호), **링크**(원본 URL과 번호의 영구적인 짝).

구성 (Cargo 워크스페이스 + Worker):

- `shared`: 화면과 백엔드가 주고받는 요청·응답 타입
- `backend`: axum + sqlx(Postgres) JSON API와 리디렉션 처리 (Railway)
- `web`: Dioxus 웹(WASM) 생성 화면 (Dioxus fullstack 서버 함수는 쓰지 않음, [ADR 0001](docs/adr/0001-dioxus-web-only-with-axum-backend.md))
- `worker`: Cloudflare Worker. 정적 화면을 내주고 숫자 경로와 `/api/*`를 백엔드로 프록시

## 로컬 실행

필요한 도구: Rust(+ `wasm32-unknown-unknown` 타겟), Dioxus CLI `dx` 0.7, Docker.

1. DB 준비: docker compose로 Postgres를 띄웁니다. 호스트 포트는 **5434**입니다.

   ```bash
   docker compose up -d
   ```

2. 백엔드 실행: 시작할 때 `sqlx` 마이그레이션이 자동으로 실행되어 테이블과 불변성 트리거가 만들어집니다.

   ```bash
   cp .env.example .env   # 값 확인 후
   set -a; source .env; set +a
   cargo run -p backend
   ```

3. 다른 터미널에서 화면을 띄웁니다. `http://localhost:8081`에서 열리고, `/api/` 요청은 `localhost:8080` 백엔드로 넘어갑니다.

   ```bash
   cd web && dx serve --port 8081
   ```

### 환경변수

| 이름 | 설명 | 기본값 |
| --- | --- | --- |
| `DATABASE_URL` | Postgres 접속 주소 | 없음(필수) |
| `PUBLIC_BASE_URL` | 단축 주소를 만들 때 쓰는 공개 기본 주소 | `http://localhost:8080` |
| `BACKEND_BASE_URL` | 백엔드 자체 주소. 이 호스트를 가리키는 원본 URL을 거부하는 데 씀(선택) | 없음 |
| `PORT` | 백엔드 수신 포트 | `8080` |

실제 비밀 값은 저장소에 넣지 않습니다. 형식은 `.env.example`을 참고하세요.

## 테스트

Docker가 실행 중이어야 합니다. testcontainers가 `short-url-test-pg`라는 Postgres 컨테이너 하나를 띄워 계속 재사용하고, 그 안에 테스트마다 새 DB를 만들어 axum 라우터에 요청을 직접 보내 HTTP 동작을 검증합니다. 컨테이너는 테스트가 끝나도 남아 있으며, 필요 없으면 `docker rm -f short-url-test-pg`로 지웁니다. Docker를 재시작해 컨테이너가 멈춰 있으면 테스트가 지우고 새로 띄웁니다.

```bash
cargo test
```

## 배포

`main`에 push하면 Railway(백엔드)와 Cloudflare(화면·프록시)가 자동으로 배포됩니다.

### 라우팅 구성

사용자는 Cloudflare 공개 주소 하나만 봅니다. 브라우저 코드는 같은 출처의 `/api/*`만 호출하므로 CORS 설정이 없고, Railway 주소는 브라우저 코드에 들어가지 않습니다.

| 경로 | 처리 |
| --- | --- |
| `/<숫자>` (`^/[0-9]+$`) | Worker가 Railway 백엔드로 프록시 (302 이동, 404 안내 화면은 백엔드가 결정) |
| `/api/*` | Worker가 Railway 백엔드로 프록시 |
| 그 밖의 모든 경로 | Dioxus 정적 파일. 없는 경로는 `index.html`(SPA 폴백) |

프록시는 메서드, 헤더(`Host` 제외, `X-Forwarded-Host/Proto` 추가), 본문, 쿼리를 그대로 넘기고 `redirect: "manual"`로 보내 백엔드의 302·404·405 응답을 수정 없이 브라우저에 전달합니다. 백엔드의 엔드포인트는 다음과 같습니다.

| 엔드포인트 | 설명 |
| --- | --- |
| `POST /api/links` | 링크 생성. `{"url": "..."}` 하나만 허용. 새 링크 201, 기존 링크 200 |
| `GET /api/links/{번호}` | 링크 조회 (JSON) |
| `GET /{번호}` | 302 + 저장된 원본 URL의 `Location` |
| `GET /healthz` | 200 `ok` |

### Railway (백엔드 + Postgres)

백엔드는 저장소 루트의 `Dockerfile`로 빌드합니다. 이미지는 `cargo build --release -p backend`로 백엔드 바이너리만 만들며(web은 포함하지 않음), 비루트 사용자로 실행됩니다. Railway 프로젝트에 Postgres와 백엔드 서비스(GitHub 저장소 `main` 연결)를 두고, 백엔드 서비스에 다음 변수를 설정합니다.

| 이름 | 값 |
| --- | --- |
| `DATABASE_URL` | `${{Postgres.DATABASE_URL}}` (Railway Postgres 서비스 변수 참조) |
| `PUBLIC_BASE_URL` | Cloudflare 공개 주소 (`https://s.zoek.workers.dev`). 단축 주소와 자기 링크 거부의 기준 |
| `BACKEND_BASE_URL` | Railway 백엔드 자신의 주소 (선택, 이 호스트를 가리키는 원본 URL 거부용) |
| `PORT` | Railway가 주입합니다. 직접 설정하지 않습니다. |

헬스체크 경로(`/healthz`)와 재시작 정책은 설정 파일이 아니라 Railway 서비스 설정(Settings → Deploy)에서 지정합니다. 마이그레이션은 백엔드가 시작할 때 자동으로 실행합니다.

### Cloudflare (화면 + 프록시)

Dioxus 화면(정적 파일)과 작은 JS Worker(`worker/`)를 Cloudflare Workers 하나(이름 `s`)로 배포합니다. 공개 주소는 `*.workers.dev`입니다.

- **백엔드 주소**: `worker/wrangler.jsonc`의 `vars.BACKEND_URL`. 비밀이 아니며 Worker 안에서만 쓰입니다. 주소가 바뀌면 이 값을 고칩니다.
- **정적 파일**: `dx bundle --platform web --release`가 만드는 `target/dx/web/release/web/public`.
- **CI**: `main`에 push하면(또는 수동 실행) `.github/workflows/deploy-web.yml`이 화면을 빌드하고 `worker/`에서 `wrangler deploy`를 실행합니다.
- **GitHub Secrets** (이름만): `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`

로컬에서 Worker 확인 (Cloudflare 로그인 불필요):

```bash
(cd web && dx bundle --platform web --release)
cd worker && npm install
npx wrangler dev   # http://localhost:8787
```

주의: 로컬 `wrangler dev`도 `BACKEND_URL`의 실제 Railway 백엔드를 호출하므로 생성한 링크가 실제 DB에 남습니다.

## 서비스 규칙

### 번호 발급 방식

번호는 Postgres `links` 테이블의 `BIGINT GENERATED ALWAYS AS IDENTITY` 기본키입니다. DB가 1부터 증가시키며 발급하므로 동시에 만들어도 겹치지 않고, 호출자가 번호를 지정할 수 없습니다. 실패한 삽입 등으로 번호 사이에 빈자리가 생길 수 있으며, 한 번 발급된 번호는 다시 쓰이지 않습니다. `/042`, `/0`처럼 0으로 시작하거나 0인 경로, DB 정수 범위를 넘는 경로는 번호로 인정하지 않고 404로 처리합니다.

### 중복 방지: 같은 원본 URL을 다시 입력하면

**같은 원본 URL을 다시 입력하면 새 번호를 만들지 않고 기존 번호를 돌려줍니다.** 응답은 200과 `existing: true`이며(새로 만들면 201과 `existing: false`), 화면은 "이미 만들어진 주소예요"를 보여 줍니다.

- `original_url` 컬럼에 UNIQUE 제약이 있습니다.
- 생성은 `INSERT ... ON CONFLICT (original_url) DO NOTHING RETURNING number`로 하고, 충돌해서 행이 돌아오지 않으면 `SELECT`로 기존 번호를 조회합니다. 기존 행은 수정하지 않습니다. 같은 URL로 동시에 요청이 와도 번호는 하나만 생깁니다.
- 비교는 앞뒤 공백을 지운 문자열의 정확한 일치입니다. 정규화하지 않으므로 `https://a.com`과 `https://a.com/`은 서로 다른 링크입니다.

### 생성 후 불변

- 수정·삭제 API가 없습니다. `/<번호>`와 `/api/links/{번호}`에 GET이 아닌 메서드(PUT, PATCH, DELETE 등)가 오면 405입니다.
- 생성 요청에 `url` 외의 필드(예: `number`)가 있으면 400입니다.
- 마지막 방어선으로 DB 트리거(`0002_links_immutable.sql`)가 `links` 테이블의 UPDATE, DELETE, TRUNCATE를 모두 거부합니다. SQL로 직접 시도해도 막히며, 통합 테스트(`backend/tests/immutability.rs`)가 이를 검증합니다.
- 서버를 재시작하거나 재배포해도 링크는 DB에 남아 같은 원본 URL로 이동합니다.

### 원본 URL 검증 규칙

앞뒤 공백을 지운 뒤 다음을 모두 만족해야 하며, 실패하면 400과 오류 메시지(아래 "언어" 규칙에 따라 한국어 또는 영어)를 돌려줍니다.

- 스킴이 `http` 또는 `https`
- 호스트가 있음
- 내부에 공백(제어 문자 포함)이 없음
- 사용자 정보(`user:pass@`)가 없음
- 2048바이트 이하
- 호스트가 `PUBLIC_BASE_URL`이나 `BACKEND_BASE_URL`의 호스트가 아님(단축 주소끼리 고리가 생기는 것을 방지)

리디렉션의 `Location`은 저장된 원본 URL(경로·쿼리·프래그먼트 포함)을 그대로 씁니다. 단축 주소에 붙은 쿼리는 무시합니다.

### 언어 (한국어/영어)

- 화면 오른쪽 위 버튼(`English` / `한국어`)을 누르면 화면 글자가 바로 바뀌고, 선택이 `lang` 쿠키(`ko` 또는 `en`, 1년)에 저장됩니다.
- 백엔드는 같은 쿠키를 보고 오류 메시지와 404 안내 화면의 언어를 정합니다. 404 안내 화면에도 같은 전환 버튼이 있습니다.
- 언어를 정하는 순서는 화면과 백엔드가 같습니다: `lang` 쿠키 → 브라우저 언어 목록(`Accept-Language`)에서 처음 나오는 `ko` 또는 `en` → 둘 다 없으면 한국어.
- 성공 응답(번호, 단축 주소)은 언어와 상관없이 같습니다. 이미 화면에 떠 있던 서버 오류 메시지는 언어를 바꿔도 다시 요청하지 않으므로 그대로 남습니다.

## 확인 시나리오

배포된 공개 주소에서 확인한 결과입니다(2026-11-09). 자동 점검 스크립트와 자동 테스트로 재현할 수 있습니다.

| 시나리오 | 기대 결과 | 결과 | 재현 방법 |
| --- | --- | --- | --- |
| 화면 열기 | `/` 200 HTML | 통과 | 점검 스크립트 |
| 새 URL 생성 | 201, 단축 주소가 공개 주소로 시작 | 통과 | 점검 스크립트 |
| 단축 주소 열기 | 302, `Location`이 원본 URL과 정확히 같음 (경로·쿼리·프래그먼트 유지) | 통과 | 점검 스크립트 |
| 같은 원본 URL 재입력 | 200, 같은 번호, `existing: true` | 통과 | 점검 스크립트 |
| 서로 다른 URL 5개 동시 생성 | 서로 다른 번호 5개, 각자의 원본 URL로 302 | 통과 | 점검 스크립트 |
| 같은 URL 동시 생성 | 번호 하나 | 통과(자동 테스트) | `cargo test` (`backend/tests/duplicates.rs`) |
| 없는 번호, `/042`, `/0` | 404 안내 화면(HTML, 기본 한국어) | 통과 | 점검 스크립트, `backend/tests/not_found.rs` |
| `lang=en` 쿠키 또는 영어 `Accept-Language` | 오류 메시지와 404 안내 화면이 영어, 쿠키가 `Accept-Language`보다 우선 | 통과(자동 테스트) | `cargo test` (`backend/tests/language.rs`) |
| 화면의 언어 버튼 | 글자가 바뀌고, 새로고침해도 선택 유지 | 통과(로컬 브라우저 확인) | 아래 수동 확인 6번 |
| PUT/PATCH/DELETE | 405, 이후에도 링크 그대로 | 통과 | 점검 스크립트 |
| 잘못된 URL(`ftp://`, 사용자 정보, 자기 링크) | 400 | 통과 | 점검 스크립트 |
| `url` 외 필드가 있는 본문 | 400 | 통과 | 점검 스크립트 |
| 호스트 없음, 공백 포함, 2048바이트 초과 | 400 | 통과(자동 테스트) | `cargo test` (`backend/tests/links.rs`) |
| DB의 UPDATE/DELETE/TRUNCATE | 트리거가 거부 | 통과(자동 테스트) | `cargo test` (`backend/tests/immutability.rs`) |
| Railway 재배포·프로세스 재시작 후 | 기존 링크가 같은 원본 URL로 이동 | 통과(수동 확인) | 링크 생성 후 `railway redeploy`, 다시 `/<번호>` 열기 |
| 시크릿 창·다른 브라우저에서 생성·복사·이동 | 생성·복사 표시·이동 동작 | 통과(사용자 수동 확인) | 아래 수동 확인 |

### 점검 스크립트

공개 주소를 인자로 받아 curl로 위 "점검 스크립트" 시나리오를 검사하고, 항목마다 PASS/FAIL을 출력하며 하나라도 실패하면 종료 코드 1로 끝납니다. `curl`과 기본 도구만 필요합니다.

```bash
scripts/smoke-check.sh                       # 기본값: https://s.zoek.workers.dev
scripts/smoke-check.sh https://example.workers.dev
```

주의: 링크는 삭제할 수 없으므로 한 번 실행할 때마다 대상 서비스에 링크가 6개 영구히 남습니다(원본 URL은 `https://example.org/smoke?run=...`).

### 수동 브라우저 확인

Dioxus 화면은 자동 테스트가 없으므로 직접 확인합니다.

1. 시크릿 창에서 서비스 URL을 열고 로그인 없이 원본 URL을 입력해 생성합니다. 입력창 아래에 단축 주소가 보이는지 확인합니다.
2. 복사 버튼을 눌러 "복사됨" 표시가 뜨는지 확인하고, 주소창에 붙여 넣어 클립보드 내용을 확인합니다.
3. 같은 원본 URL을 다시 입력해 "이미 만들어진 주소예요" 안내와 같은 번호가 나오는지 확인합니다.
4. 만든 단축 주소를 다른 브라우저에서 열어 원본 URL로 이동하는지 확인합니다.
5. 잘못된 URL(`ftp://example.org` 등)을 넣어 입력창 아래에 오류 이유가 보이는지 확인합니다.
6. 오른쪽 위 `English` 버튼을 눌러 화면이 영어로 바뀌는지, 잘못된 URL의 오류도 영어로 나오는지, 새로고침해도 영어로 남는지 확인합니다. 없는 번호(`/999999`)를 열어 404 안내 화면도 영어인지 확인합니다.

## Claude와 일한 방식

이 프로젝트는 Claude Code와 함께 만들었습니다. 사람은 무엇을 만들지 정하고, 계정·결제·비밀 값처럼 사람만 할 수 있는 일을 맡았습니다. 설계 정리, 구현, 검증, 배포 작업은 Claude가 진행했습니다.

### 역할

| 누가 | 한 일 |
| --- | --- |
| 사람 | 요구사항 제시, 설계 질문에 답하기, 결과 확인, GitHub·Railway·Cloudflare 로그인과 연동 승인, Cloudflare 이메일 인증과 API 토큰 발급, GitHub Secrets 등록, 실제 브라우저에서 복사 버튼 확인 |
| Claude (Opus, 진행 담당) | 설계 질문 정리, 스펙과 티켓 작성, 구현 에이전트에게 작업 지시, 결과 검토(테스트 재실행, 코드 읽기, 실제 서버 확인), Railway·Cloudflare 설정, 커밋과 push |
| Claude (Sonnet 서브 에이전트, 구현 담당) | 티켓 하나씩 구현과 테스트 작성 |

### 진행 순서

[Matt Pocock의 스킬](https://github.com/mattpocock/skills)을 따라 진행했습니다.

1. **설계 맞추기 (`grill-with-docs`)**: Claude가 질문을 묶어서 추천 답과 함께 던지고, 사람이 답하는 과정을 반복해 설계를 정했습니다. 정해진 용어는 [GLOSSARY.md](GLOSSARY.md)에, 되돌리기 어려운 결정은 [docs/adr/](docs/adr/)에 기록했습니다.
2. **스펙 (`to-spec`)**: 정한 내용을 [.scratch/numeric-short-links/spec.md](.scratch/numeric-short-links/spec.md) 하나로 정리했습니다. 테스트를 붙일 지점(백엔드 HTTP 경계, 배포 후 점검 스크립트)도 이때 함께 정했습니다.
3. **티켓 (`to-tickets`)**: 스펙을 DB부터 화면까지 한 번에 관통하는 작은 단위 티켓 8개로 나눠 [.scratch/numeric-short-links/issues/](.scratch/numeric-short-links/issues/)에 저장했습니다.
4. **구현과 검토**: 티켓 하나마다 Sonnet 에이전트가 구현했습니다. Opus는 결과를 그대로 믿지 않고 테스트를 다시 돌리고 코드를 읽은 뒤 다음 티켓으로 넘어갔습니다. 같은 파일을 고치는 티켓끼리 충돌하지 않도록 순서대로 진행했습니다.
5. **배포**: Railway와 Cloudflare 설정은 Claude가 CLI로 했습니다. 로그인과 토큰 등록처럼 사람이 해야 하는 단계는 그때그때 안내했습니다.
6. **기록**: 티켓마다 체크리스트와 `## Comments`에 실제로 확인한 것과 확인하지 못한 것을 구분해서 남겼습니다.

### 검토하며 잡은 문제

- 한글이 들어간 원본 URL이 이동 응답 헤더에서 500을 냈습니다. 저장은 원문 그대로 하고, 이동할 때만 ASCII 형태로 바꾸도록 고쳤습니다.
- 2048자 제한을 글자 수가 아닌 바이트 수로 확인하도록 바꿨습니다.
- Railway의 설정 파일 방식이 곧 지원 종료된다는 경고를 보고, 설정 파일 없이 Dockerfile 자동 인식을 쓰도록 계획을 바꿨습니다.
- Railway에 push 자동 배포가 연결되지 않은 것을 실제 push로 확인하고, 대시보드에서 연동을 다시 하도록 안내했습니다.
- GitHub Actions의 Cloudflare 배포가 토큰 권한 문제로 실패한 것을 로그에서 찾아, 필요한 권한을 안내한 뒤 다시 실행했습니다.
- 통합 테스트가 실행할 때마다 Postgres 컨테이너를 5개씩 남기는 문제를 찾아, 이름 붙인 컨테이너 하나를 재사용하도록 고쳤습니다.
- README 확인 시나리오 표에서 실제로 하지 않은 수동 확인이 "통과"로 적혀 있던 부분을 바로잡았습니다.
- 한국어/영어 전환을 넣을 때 오류 메시지가 서버에서 오므로 화면만 바꾸면 안 된다는 점을 보고, 화면과 백엔드가 같은 `lang` 쿠키를 보도록 설계했습니다. 이때 Docker 재시작 뒤 멈춘 테스트 컨테이너 때문에 테스트가 시작되지 않는 문제도 찾아 고쳤습니다.

### 이 저장소에서 Claude가 따르는 지침

- [AGENTS.md](AGENTS.md): 이슈 트래커, 분류 라벨, 도메인 문서 위치. `CLAUDE.md`는 이 파일을 불러옵니다.
- [docs/agents/](docs/agents/): 위 항목의 자세한 규칙

---

## English

### Deployed URLs

- Service URL: https://s.zoek.workers.dev
- Test short URL: https://s.zoek.workers.dev/4
- Test original URL: https://github.com/zoek1211/short-url

### About

Enter an original URL, no login needed, and get a short URL made only of digits, like `/123`. A link is never edited, deleted, or expired, and a number is never reused for another original URL. The button at the top right switches the screen between Korean and English.

Terms follow [GLOSSARY.md](GLOSSARY.md): **original URL** (the address to shorten), **number** (a positive integer that identifies one link), **short URL** (public base URL + number), **link** (the permanent pair of an original URL and a number).

Layout (Cargo workspace + Worker):

- `shared`: request/response types shared by the screen and the backend
- `backend`: axum + sqlx (Postgres) JSON API and redirects (Railway)
- `web`: Dioxus web (WASM) screen for creating links (no Dioxus fullstack server functions, see [ADR 0001](docs/adr/0001-dioxus-web-only-with-axum-backend.md))
- `worker`: Cloudflare Worker. Serves the static screen and proxies number paths and `/api/*` to the backend

### Running locally

Tools: Rust (with the `wasm32-unknown-unknown` target), Dioxus CLI `dx` 0.7, Docker.

1. Database: start Postgres with docker compose. The host port is **5434**.

   ```bash
   docker compose up -d
   ```

2. Backend: `sqlx` migrations run on startup and create the table and the immutability triggers.

   ```bash
   cp .env.example .env   # check the values
   set -a; source .env; set +a
   cargo run -p backend
   ```

3. In another terminal, start the screen. It opens at `http://localhost:8081`, and `/api/` requests go to the backend on `localhost:8080`.

   ```bash
   cd web && dx serve --port 8081
   ```

#### Environment variables

| Name | Meaning | Default |
| --- | --- | --- |
| `DATABASE_URL` | Postgres connection URL | none (required) |
| `PUBLIC_BASE_URL` | Public base URL used to build short URLs | `http://localhost:8080` |
| `BACKEND_BASE_URL` | The backend's own URL. Original URLs pointing at this host are rejected (optional) | none |
| `PORT` | Backend listen port | `8080` |

Real secrets are never committed. See `.env.example` for the format.

### Tests

Docker must be running. testcontainers starts one Postgres container named `short-url-test-pg` and keeps reusing it. Each test creates a fresh database inside it and sends requests straight to the axum router to check HTTP behavior. The container stays after the tests; remove it with `docker rm -f short-url-test-pg` when you no longer need it. If it was stopped by a Docker restart, the tests remove it and start a new one.

```bash
cargo test
```

### Deployment

Pushing to `main` deploys Railway (backend) and Cloudflare (screen and proxy) automatically.

#### Routing

Users only see the one Cloudflare public URL. Browser code only calls same-origin `/api/*`, so there is no CORS setup, and the Railway URL never appears in browser code.

| Path | Handled by |
| --- | --- |
| `/<number>` (`^/[0-9]+$`) | Worker proxies to the Railway backend (the backend decides between the 302 redirect and the 404 page) |
| `/api/*` | Worker proxies to the Railway backend |
| Everything else | Dioxus static files. Unknown paths get `index.html` (SPA fallback) |

The proxy forwards the method, headers (except `Host`, plus `X-Forwarded-Host/Proto`), body, and query, and uses `redirect: "manual"` so the backend's 302, 404, and 405 responses reach the browser unchanged. Backend endpoints:

| Endpoint | Description |
| --- | --- |
| `POST /api/links` | Create a link. Only `{"url": "..."}` is accepted. 201 for a new link, 200 for an existing one |
| `GET /api/links/{number}` | Look up a link (JSON) |
| `GET /{number}` | 302 with the stored original URL in `Location` |
| `GET /healthz` | 200 `ok` |

#### Railway (backend + Postgres)

The backend is built from the `Dockerfile` at the repository root. The image builds only the backend binary with `cargo build --release -p backend` (no web) and runs as a non-root user. The Railway project has a Postgres service and a backend service connected to the GitHub repository's `main`. Set these variables on the backend service:

| Name | Value |
| --- | --- |
| `DATABASE_URL` | `${{Postgres.DATABASE_URL}}` (reference to the Railway Postgres service variable) |
| `PUBLIC_BASE_URL` | Cloudflare public URL (`https://s.zoek.workers.dev`). Base for short URLs and for rejecting self-links |
| `BACKEND_BASE_URL` | The Railway backend's own URL (optional, used to reject original URLs pointing at this host) |
| `PORT` | Injected by Railway. Do not set it yourself. |

The healthcheck path (`/healthz`) and restart policy are set in the Railway service settings (Settings → Deploy), not in a config file. Migrations run automatically when the backend starts.

#### Cloudflare (screen + proxy)

The Dioxus screen (static files) and a small JS Worker (`worker/`) are deployed together as one Cloudflare Worker named `s`. The public URL is on `*.workers.dev`.

- **Backend URL**: `vars.BACKEND_URL` in `worker/wrangler.jsonc`. Not a secret, and only used inside the Worker. Change it if the backend URL changes.
- **Static files**: `target/dx/web/release/web/public`, produced by `dx bundle --platform web --release`.
- **CI**: on push to `main` (or a manual run), `.github/workflows/deploy-web.yml` builds the screen and runs `wrangler deploy` in `worker/`.
- **GitHub Secrets** (names only): `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`

Trying the Worker locally (no Cloudflare login needed):

```bash
(cd web && dx bundle --platform web --release)
cd worker && npm install
npx wrangler dev   # http://localhost:8787
```

Note: local `wrangler dev` still calls the real Railway backend in `BACKEND_URL`, so links you create stay in the real database.

### Service rules

#### How numbers are issued

The number is the `BIGINT GENERATED ALWAYS AS IDENTITY` primary key of the Postgres `links` table. The database issues it counting up from 1, so concurrent creates never collide, and callers cannot choose a number. Failed inserts can leave gaps, and an issued number is never reused. Paths that start with 0 or are 0 (like `/042`, `/0`), and paths beyond the database integer range, are not numbers and get a 404.

#### Duplicates: entering the same original URL again

**Entering the same original URL again does not create a new number; it returns the existing one.** The response is 200 with `existing: true` (a new link is 201 with `existing: false`), and the screen shows a notice that the URL was already shortened.

- The `original_url` column has a UNIQUE constraint.
- Creation uses `INSERT ... ON CONFLICT (original_url) DO NOTHING RETURNING number`; if a conflict returns no row, a `SELECT` reads the existing number. The existing row is never modified. Concurrent requests for the same URL produce only one number.
- Matching is an exact comparison of the string after trimming surrounding whitespace. There is no normalization, so `https://a.com` and `https://a.com/` are different links.

#### Immutable after creation

- There is no update or delete API. Any method other than GET (PUT, PATCH, DELETE, ...) on `/<number>` or `/api/links/{number}` gets 405.
- A create request with any field other than `url` (for example `number`) gets 400.
- As the last line of defense, database triggers (`0002_links_immutable.sql`) reject every UPDATE, DELETE, and TRUNCATE on the `links` table, even from direct SQL. The integration tests (`backend/tests/immutability.rs`) check this.
- Links survive server restarts and redeploys and keep redirecting to the same original URL.

#### Original URL validation

After trimming surrounding whitespace, all of these must hold; otherwise the response is 400 with an error message (Korean or English, following the "Language" rule below).

- The scheme is `http` or `https`
- There is a host
- No whitespace (including control characters) inside
- No user info (`user:pass@`)
- At most 2048 bytes
- The host is not the host of `PUBLIC_BASE_URL` or `BACKEND_BASE_URL` (prevents short URLs pointing at each other)

The redirect's `Location` is the stored original URL as is (path, query, and fragment included). A query added to the short URL is ignored.

#### Language (Korean/English)

- The button at the top right of the screen (`English` / `한국어`) switches the text right away and saves the choice in a `lang` cookie (`ko` or `en`, one year).
- The backend reads the same cookie to choose the language of error messages and the 404 page. The 404 page has the same switch button.
- The screen and the backend pick the language the same way: the `lang` cookie, then the first `ko` or `en` in the browser's language list (`Accept-Language`), and Korean if neither is there.
- Success responses (number, short URL) are the same in both languages. A server error message already on screen is not re-requested when you switch, so it stays in the language it came in.

### Verification scenarios

Results checked on the deployed public URL (2026-11-09). They can be reproduced with the smoke-check script and the automated tests.

| Scenario | Expected | Result | How to reproduce |
| --- | --- | --- | --- |
| Open the screen | `/` 200 HTML | Pass | smoke-check script |
| Create a new URL | 201, short URL starts with the public URL | Pass | smoke-check script |
| Open the short URL | 302, `Location` exactly equals the original URL (path, query, fragment kept) | Pass | smoke-check script |
| Enter the same original URL again | 200, same number, `existing: true` | Pass | smoke-check script |
| Create 5 different URLs concurrently | 5 different numbers, each redirects to its own original URL | Pass | smoke-check script |
| Create the same URL concurrently | One number | Pass (automated test) | `cargo test` (`backend/tests/duplicates.rs`) |
| Unknown number, `/042`, `/0` | 404 page (HTML, Korean by default) | Pass | smoke-check script, `backend/tests/not_found.rs` |
| `lang=en` cookie or English `Accept-Language` | Error messages and the 404 page in English; the cookie wins over `Accept-Language` | Pass (automated test) | `cargo test` (`backend/tests/language.rs`) |
| Language button on the screen | Text switches and the choice survives a reload | Pass (local browser check) | manual check step 6 below |
| PUT/PATCH/DELETE | 405, link unchanged afterwards | Pass | smoke-check script |
| Invalid URL (`ftp://`, user info, self-link) | 400 | Pass | smoke-check script |
| Body with fields other than `url` | 400 | Pass | smoke-check script |
| No host, whitespace inside, over 2048 bytes | 400 | Pass (automated test) | `cargo test` (`backend/tests/links.rs`) |
| UPDATE/DELETE/TRUNCATE in the database | Rejected by triggers | Pass (automated test) | `cargo test` (`backend/tests/immutability.rs`) |
| After a Railway redeploy or process restart | Existing links redirect to the same original URL | Pass (manual check) | create a link, `railway redeploy`, open `/<number>` again |
| Create, copy, and open in a private window and another browser | Create, "copied" notice, and redirect all work | Pass (manual check by the user) | manual check below |

#### Smoke-check script

Takes the public URL as an argument, checks the "smoke-check script" scenarios above with curl, prints PASS/FAIL per item, and exits with code 1 if anything fails. Only `curl` and standard tools are needed.

```bash
scripts/smoke-check.sh                       # default: https://s.zoek.workers.dev
scripts/smoke-check.sh https://example.workers.dev
```

Note: links cannot be deleted, so each run leaves 6 permanent links on the target service (original URLs are `https://example.org/smoke?run=...`).

#### Manual browser check

The Dioxus screen has no automated tests, so check it by hand.

1. Open the service URL in a private window and create a link from an original URL without logging in. Check that the short URL appears below the input.
2. Click the copy button, check that the "copied" notice appears, and paste into the address bar to check the clipboard.
3. Enter the same original URL again and check that the "already shortened" notice and the same number appear.
4. Open the short URL in another browser and check that it goes to the original URL.
5. Enter an invalid URL (such as `ftp://example.org`) and check that the reason appears below the input.
6. Click `English` at the top right and check that the screen switches to English, that invalid-URL errors are in English, and that English stays after a reload. Open an unknown number (`/999999`) and check that the 404 page is in English too.

### How this was built with Claude

This project was built with Claude Code. The human decided what to build and handled what only a person can do: accounts, billing, and secrets. Claude did the design write-up, implementation, verification, and deployment work.

#### Roles

| Who | What they did |
| --- | --- |
| Human | Gave the requirements, answered design questions, checked results, logged in to and approved GitHub, Railway, and Cloudflare connections, verified the Cloudflare email and issued the API token, registered GitHub Secrets, checked the copy button in a real browser |
| Claude (Opus, lead) | Organized design questions, wrote the spec and tickets, gave tasks to implementation agents, reviewed results (reran tests, read code, checked the live server), configured Railway and Cloudflare, committed and pushed |
| Claude (Sonnet subagents, implementers) | Implemented one ticket at a time and wrote its tests |

#### Process

Work followed [Matt Pocock's skills](https://github.com/mattpocock/skills).

1. **Agreeing on the design (`grill-with-docs`)**: Claude asked grouped questions with a recommended answer, and the human answered, round after round, until the design was settled. Settled terms went into [GLOSSARY.md](GLOSSARY.md), and hard-to-reverse decisions into [docs/adr/](docs/adr/).
2. **Spec (`to-spec`)**: The decisions were collected into one file, [.scratch/numeric-short-links/spec.md](.scratch/numeric-short-links/spec.md), along with where the tests attach (the backend HTTP boundary and the post-deploy smoke-check script).
3. **Tickets (`to-tickets`)**: The spec was split into 8 small tickets, each cutting through everything from the database to the screen, saved in [.scratch/numeric-short-links/issues/](.scratch/numeric-short-links/issues/).
4. **Implementation and review**: A Sonnet agent implemented each ticket. Opus did not take the result on trust: it reran the tests and read the code before moving to the next ticket. Tickets touching the same files ran one after another so they would not conflict.
5. **Deployment**: Claude configured Railway and Cloudflare through their CLIs, and walked the human through the steps only a person could do, such as logging in and registering tokens.
6. **Records**: Each ticket's checklist and `## Comments` separate what was actually verified from what was not.

#### Problems caught in review

- An original URL containing Korean text caused a 500 in the redirect header. The fix stores the original text and converts it to ASCII form only when redirecting.
- The 2048 limit now counts bytes, not characters.
- After seeing a warning that Railway's config-file approach is being retired, the plan switched to no config file and automatic Dockerfile detection.
- A real push showed that Railway's auto-deploy was not connected, and the human was guided to reconnect it in the dashboard.
- The GitHub Actions Cloudflare deploy failed on token permissions; the cause was found in the logs, the needed permissions were explained, and the run was retried.
- Integration tests left 5 Postgres containers behind on every run; they now reuse one named container.
- A row in the README's verification table marked a manual check as "Pass" that had not actually been done; it was corrected.
- When adding Korean/English switching, error messages come from the server, so switching only the screen was not enough; the screen and backend now read the same `lang` cookie. Along the way, a stopped test container left by a Docker restart was keeping the tests from starting; that was fixed too.

#### Guidelines Claude follows in this repository

- [AGENTS.md](AGENTS.md): issue tracker, triage labels, and where the domain docs live. `CLAUDE.md` loads this file.
- [docs/agents/](docs/agents/): the detailed rules for those items
