# 숫자형 단축 URL 서비스

## 배포 URL

- 서비스 URL: https://s.zoek.workers.dev
- 테스트 단축 URL: https://s.zoek.workers.dev/4
- 테스트 원본 URL: https://github.com/zoek1211/short-url

## 소개

로그인 없이 원본 URL을 입력하면 `/123`처럼 숫자로만 된 단축 주소를 만들어 주는 서비스입니다. 한 번 발급한 링크는 수정·삭제·만료되지 않고, 번호는 다른 원본 URL에 다시 쓰이지 않습니다.

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

Docker가 실행 중이어야 합니다. 테스트마다 testcontainers로 새 Postgres DB를 만들고, axum 라우터에 요청을 직접 보내 HTTP 동작을 검증합니다.

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

앞뒤 공백을 지운 뒤 다음을 모두 만족해야 하며, 실패하면 400과 한국어 오류 메시지를 돌려줍니다.

- 스킴이 `http` 또는 `https`
- 호스트가 있음
- 내부에 공백(제어 문자 포함)이 없음
- 사용자 정보(`user:pass@`)가 없음
- 2048바이트 이하
- 호스트가 `PUBLIC_BASE_URL`이나 `BACKEND_BASE_URL`의 호스트가 아님(단축 주소끼리 고리가 생기는 것을 방지)

리디렉션의 `Location`은 저장된 원본 URL(경로·쿼리·프래그먼트 포함)을 그대로 씁니다. 단축 주소에 붙은 쿼리는 무시합니다.

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
| 없는 번호, `/042`, `/0` | 404 한국어 안내 화면(HTML) | 통과 | 점검 스크립트, `backend/tests/not_found.rs` |
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
