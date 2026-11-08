# 06: Railway 백엔드 배포

**What to build:** 백엔드와 Postgres가 Railway에서 돌고, 공개 GitHub 저장소에 push하면 백엔드가 자동으로 다시 배포된다. Railway 백엔드 주소로 생성과 `/<번호>` 이동이 동작하고, 재배포 후에도 기존 링크가 유지된다.

**Blocked by:** 01 로컬에서 생성하고 이동하기

**Status:** ready-for-agent

**사용자가 직접 할 단계:** GitHub 로그인과 저장소 생성 승인, Railway 로그인, Railway와 GitHub 저장소 연동 승인.

- [ ] 백엔드를 빌드·실행하는 Dockerfile이 있다
- [ ] 백엔드가 DB 접속 주소, 공개 기본 주소, 백엔드 자신의 주소, 수신 포트를 환경변수로 받는다
- [ ] 프로젝트가 git 저장소이고 공개 GitHub 저장소에 올라가 있다. 저장소에 비밀 값이 없다
- [ ] Railway에 백엔드 서비스와 Postgres가 있고, 백엔드가 Railway가 제공하는 DB 접속 주소를 쓴다
- [ ] push하면 Railway가 백엔드를 자동으로 다시 배포하고, 시작할 때 마이그레이션이 실행된다
- [ ] Railway 백엔드 주소로 생성 API와 `/<번호>` 이동이 HTTPS로 동작한다
- [ ] 재배포한 뒤에도 이전에 만든 번호가 같은 원본 URL로 이동한다
