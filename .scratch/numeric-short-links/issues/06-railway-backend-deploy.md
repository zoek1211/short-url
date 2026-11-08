# 06: Railway 백엔드 배포

**What to build:** 백엔드와 Postgres가 Railway에서 돌고, 공개 GitHub 저장소에 push하면 백엔드가 자동으로 다시 배포된다. Railway 백엔드 주소로 생성과 `/<번호>` 이동이 동작하고, 재배포 후에도 기존 링크가 유지된다.

**Blocked by:** 01 로컬에서 생성하고 이동하기

**Status:** ready-for-agent

**사용자가 직접 할 단계:** GitHub 로그인과 저장소 생성 승인, Railway 로그인, Railway와 GitHub 저장소 연동 승인.

- [x] 백엔드를 빌드·실행하는 Dockerfile이 있다
- [x] 백엔드가 DB 접속 주소, 공개 기본 주소, 백엔드 자신의 주소, 수신 포트를 환경변수로 받는다
- [ ] 프로젝트가 git 저장소이고 공개 GitHub 저장소에 올라가 있다. 저장소에 비밀 값이 없다
- [ ] Railway에 백엔드 서비스와 Postgres가 있고, 백엔드가 Railway가 제공하는 DB 접속 주소를 쓴다
- [ ] push하면 Railway가 백엔드를 자동으로 다시 배포하고, 시작할 때 마이그레이션이 실행된다
- [ ] Railway 백엔드 주소로 생성 API와 `/<번호>` 이동이 HTTPS로 동작한다
- [ ] 재배포한 뒤에도 이전에 만든 번호가 같은 원본 URL로 이동한다

## Comments

- 저장소 쪽 작업 완료: 루트 `Dockerfile`(백엔드만 빌드, 비루트 실행), `.dockerignore`, `GET /healthz` + 테스트, README "Railway 배포" 섹션. 로컬에서 이미지를 빌드해 compose Postgres로 생성(201)·이동(302)·`/healthz`(200)를 확인했다.
- `railway.json`/`railway.toml`은 만들지 않았다(Config as Code 폐지 예정). 헬스체크 경로 `/healthz`와 재시작 정책은 Railway 서비스 설정에서 직접 지정한다.
- 나머지 항목(git/GitHub, Railway 프로젝트, 자동 재배포, HTTPS 확인, 재배포 후 유지)은 실제 Railway 작업이 필요해 남겨 둔다.
