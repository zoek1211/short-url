# 07: Cloudflare 프록시와 화면 배포

**What to build:** 공개 주소(`*.workers.dev`)에서 시크릿 창으로 생성 화면을 열어 단축 주소를 만들고, 그 단축 주소를 다른 브라우저에서 열면 원본 URL로 이동한다. Railway 주소는 브라우저에 드러나지 않는다.

**Blocked by:** 06 Railway 백엔드 배포

**Status:** ready-for-agent

**사용자가 직접 할 단계:** Cloudflare 로그인, Cloudflare API 토큰 발급과 GitHub Secrets 등록.

- [x] Dioxus 화면 빌드 결과가 Workers 정적 파일로 올라간다
- [x] JS Worker가 숫자로만 된 경로와 `/api/*`를 Railway 백엔드로 넘기고, 나머지는 화면을 내준다
- [x] 프록시 요청은 리디렉션을 따라가지 않고, 백엔드의 302·404·405 응답을 그대로 브라우저에 전달한다
- [x] 백엔드 주소는 Worker 환경변수로 설정되고 브라우저 코드에 들어가지 않는다
- [x] Railway 백엔드의 공개 기본 주소 환경변수가 Cloudflare 공개 주소로 설정되어, 화면에 공개 주소 기준 단축 주소가 보인다
- [x] GitHub Actions가 push 때 화면을 빌드하고 `wrangler deploy`로 배포한다. 토큰은 GitHub Secrets에만 있다
- [ ] 공개 주소에 HTTPS로 접속해 로그인 없이 생성, 복사, 이동이 동작한다

## Comments

- 저장소 쪽 작업(2026-11-08): `worker/`(Worker JS, `wrangler.jsonc`, `package.json`)와 `.github/workflows/deploy-web.yml`, README "Cloudflare 배포" 절을 추가했다. 정적 파일 경로는 `target/dx/web/release/web/public`.
- 로컬 `wrangler dev`(무로그인)로 확인: `/` 200 화면 HTML, `POST /api/links` 201, `/2` 302 + 원본 URL `Location`, `/999999999` 404 한국어 HTML, `DELETE /2` 405, `/some/spa/path` 200 화면 HTML. 릴리스 번들에 railway 주소 없음.
- 남은 일(사용자): Cloudflare 로그인·API 토큰 발급, GitHub Secrets(`CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`) 등록, 첫 배포 후 Railway `PUBLIC_BASE_URL`을 workers.dev 주소로 변경, HTTPS 공개 주소에서 생성·복사·이동 확인. 체크하지 않은 항목 2개(`PUBLIC_BASE_URL` 설정, 공개 주소 동작)가 이에 해당한다.
- 첫 배포(2026-11-08, 로컬 `wrangler deploy`): 공개 주소 `https://short-url.short-url-worker.workers.dev`. Railway `PUBLIC_BASE_URL`을 이 주소로 바꿨다. `wrangler.jsonc`에 `workers_dev: true`, `preview_urls: false`를 명시했다.
- 공개 주소 확인: `/` 200, 기존 링크 재입력 200 + `existing:true`, `/2` 302, 공개 주소 자기 링크 400, 없는 번호 404 HTML, DELETE 405, SPA 경로 200. 내장 브라우저에서 잘못된 URL 오류 문구, 생성(`/4` → `https://github.com/zoek1211/short-url`), "이미 만들어진 주소예요" 안내를 확인했다.
- 복사 버튼은 내장 브라우저가 클립보드 권한을 막아 확인하지 못했다(복사 실패 시 화면에 아무 표시가 없음). 실제 브라우저에서 확인 필요.
- GitHub Secrets 등록은 사용자 몫(자동 모드에서 비밀 저장소 쓰기가 거부됨). 등록 전까지 워크플로는 push하지 않는다.
- GitHub Actions(2026-11-09): 첫 실행은 토큰이 계정에 적용되지 않아 "No access to the specified service"로 배포 단계만 실패했다. 사용자가 토큰을 다시 만들어(Workers Scripts Edit, Account Settings Read, User Details Read, 이 계정 한정) 등록한 뒤 재실행해 성공했다(run 42539508380). 배포 후 공개 주소에서 `/` 200, `/4` 302, 중복 200, 404, 405를 다시 확인했다.
- 남은 확인: 실제 브라우저(시크릿 창)에서 복사 버튼 동작. 확인되면 마지막 항목을 체크한다.
