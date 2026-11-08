# 숫자형 단축 URL 서비스

## 배포 URL

> 아직 배포 전입니다. Cloudflare·Railway 배포 후 실제 주소로 바꿉니다.

- 서비스 URL: (배포 후 작성)
- 테스트 단축 URL: (배포 후 작성)
- 테스트 원본 URL: (배포 후 작성)

## 소개

로그인 없이 원본 URL을 입력하면 `/123`처럼 숫자로만 된 단축 주소를 만들어 주는 서비스입니다. 한 번 발급한 링크는 수정·삭제·만료되지 않습니다.

- `shared`: 화면과 백엔드가 주고받는 요청·응답 타입
- `backend`: axum + sqlx(Postgres) JSON API와 리디렉션 처리
- `web`: Dioxus 웹(WASM) 생성 화면

## 로컬 실행

필요한 도구: Rust(+ `wasm32-unknown-unknown` 타겟), Dioxus CLI `dx` 0.7, Docker.

1. DB를 띄웁니다. Postgres가 호스트 포트 5434에서 열립니다.

   ```bash
   docker compose up -d
   ```

2. 백엔드를 실행합니다. 시작할 때 마이그레이션으로 테이블을 만듭니다.

   ```bash
   cp .env.example .env   # 값 확인 후
   set -a; source .env; set +a
   cargo run -p backend
   ```

3. 다른 터미널에서 화면을 띄웁니다. `http://localhost:8081`에서 열리고, `/api/` 요청은 `localhost:8080` 백엔드로 넘어갑니다.

   ```bash
   cd web && dx serve --port 8081
   ```

## 테스트

Docker가 실행 중이어야 합니다. 테스트마다 testcontainers로 새 Postgres DB를 만듭니다.

```bash
cargo test
```

## 환경변수

| 이름 | 설명 | 기본값 |
| --- | --- | --- |
| `DATABASE_URL` | Postgres 접속 주소 | 없음(필수) |
| `PUBLIC_BASE_URL` | 단축 주소를 만들 때 쓰는 공개 기본 주소 | `http://localhost:8080` |
| `BACKEND_BASE_URL` | 백엔드 자체 주소. 이 호스트를 가리키는 원본 URL을 거부하는 데 씀(선택) | 없음 |
| `PORT` | 백엔드 수신 포트 | `8080` |

실제 비밀 값은 저장소에 넣지 않습니다. 형식은 `.env.example`을 참고하세요.

## Railway 배포

백엔드는 Railway에서 저장소 루트의 `Dockerfile`로 빌드합니다. 이미지는 `cargo build --release -p backend`로 백엔드 바이너리만 만들며(web은 포함하지 않음), 비루트 사용자로 실행됩니다.

Railway 백엔드 서비스에 설정할 환경변수:

| 이름 | 값 |
| --- | --- |
| `DATABASE_URL` | `${{Postgres.DATABASE_URL}}` (Railway Postgres 서비스 변수 참조) |
| `PUBLIC_BASE_URL` | 단축 주소로 보여 줄 공개 기본 주소 |
| `BACKEND_BASE_URL` | 백엔드 자신의 주소 (선택) |
| `PORT` | Railway가 주입합니다. 직접 설정하지 않습니다. |

- 마이그레이션: 백엔드가 시작할 때 자동으로 실행합니다.
- 헬스체크: `GET /healthz`가 200 `ok`를 돌려줍니다. 헬스체크 경로(`/healthz`)와 재시작 정책은 설정 파일이 아니라 Railway 서비스 설정에서 지정합니다.
- Cloudflare(웹 화면) 배포는 아래 "Cloudflare 배포"를 참고하세요.

## Cloudflare 배포

Dioxus 화면(정적 파일)과 작은 JS Worker(`worker/`)를 Cloudflare Workers 하나로 배포합니다. 공개 주소는 `*.workers.dev`입니다.

| 경로 | 처리 |
| --- | --- |
| `/api/*` | Railway 백엔드로 프록시 |
| `/<숫자>` (`^/[0-9]+$`) | Railway 백엔드로 프록시 (404 여부는 백엔드가 결정) |
| 그 밖의 모든 경로 | 정적 파일. 없는 경로는 `index.html`(SPA 폴백) |

프록시는 메서드, 헤더(`Host` 제외, `X-Forwarded-Host/Proto` 추가), 본문, 쿼리를 그대로 넘기고, `redirect: "manual"`로 보내 백엔드의 302·404·405 응답을 수정 없이 브라우저에 전달합니다.

- **백엔드 주소**: `worker/wrangler.jsonc`의 `vars.BACKEND_URL`에 있습니다. 비밀이 아니며 Worker 안에서만 쓰여 브라우저 코드에는 들어가지 않습니다. 주소가 바뀌면 이 값을 고칩니다.
- **정적 파일**: `dx bundle --platform web --release`가 만드는 `target/dx/web/release/web/public`을 그대로 씁니다.
- **CI**: `main`에 push하면(또는 수동 실행) `.github/workflows/deploy-web.yml`이 화면을 빌드하고 `worker/`에서 `wrangler deploy`를 실행합니다.
- **GitHub Secrets** (이름만): `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`
- **Railway**: 화면에 공개 주소 기준 단축 주소가 보이려면 Railway 백엔드의 `PUBLIC_BASE_URL`을 Cloudflare 공개 주소로 설정합니다.

로컬에서 Worker 확인 (Cloudflare 로그인 불필요):

```bash
(cd web && dx bundle --platform web --release)
cd worker && npm install
npx wrangler dev   # http://localhost:8787
```

주의: 로컬 `wrangler dev`도 `BACKEND_URL`의 실제 Railway 백엔드를 호출하므로 생성한 링크가 실제 DB에 남습니다.
