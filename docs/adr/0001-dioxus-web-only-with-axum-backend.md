# Dioxus는 웹 화면만, 백엔드는 axum으로 분리

배포 조건상 화면은 Cloudflare Workers에, 생성 API와 리디렉션은 Railway에 따로 올려야 한다. Dioxus fullstack의 서버 함수는 화면과 서버가 한 프로그램으로 함께 배포된다고 가정하므로, `web`은 Dioxus 웹(WASM) 전용으로 두고 `backend`는 axum JSON API로 분리했다. 두 쪽이 주고받는 요청·응답 타입은 `shared` 크레이트 하나에서 관리한다.
