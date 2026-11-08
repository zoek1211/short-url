# 07: Cloudflare 프록시와 화면 배포

**What to build:** 공개 주소(`*.workers.dev`)에서 시크릿 창으로 생성 화면을 열어 단축 주소를 만들고, 그 단축 주소를 다른 브라우저에서 열면 원본 URL로 이동한다. Railway 주소는 브라우저에 드러나지 않는다.

**Blocked by:** 06 Railway 백엔드 배포

**Status:** ready-for-agent

**사용자가 직접 할 단계:** Cloudflare 로그인, Cloudflare API 토큰 발급과 GitHub Secrets 등록.

- [ ] Dioxus 화면 빌드 결과가 Workers 정적 파일로 올라간다
- [ ] JS Worker가 숫자로만 된 경로와 `/api/*`를 Railway 백엔드로 넘기고, 나머지는 화면을 내준다
- [ ] 프록시 요청은 리디렉션을 따라가지 않고, 백엔드의 302·404·405 응답을 그대로 브라우저에 전달한다
- [ ] 백엔드 주소는 Worker 환경변수로 설정되고 브라우저 코드에 들어가지 않는다
- [ ] Railway 백엔드의 공개 기본 주소 환경변수가 Cloudflare 공개 주소로 설정되어, 화면에 공개 주소 기준 단축 주소가 보인다
- [ ] GitHub Actions가 push 때 화면을 빌드하고 `wrangler deploy`로 배포한다. 토큰은 GitHub Secrets에만 있다
- [ ] 공개 주소에 HTTPS로 접속해 로그인 없이 생성, 복사, 이동이 동작한다
