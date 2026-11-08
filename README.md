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
| `PORT` | 백엔드 수신 포트 | `8080` |

실제 비밀 값은 저장소에 넣지 않습니다. 형식은 `.env.example`을 참고하세요.
