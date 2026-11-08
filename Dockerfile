# syntax=docker/dockerfile:1
# 백엔드 바이너리만 빌드합니다. web(WASM)은 이미지에 포함하지 않습니다.
FROM rust:1.99-slim-bookworm AS builder
WORKDIR /app

# 1) 의존성 레이어 캐시: 매니페스트와 빈 소스만으로 먼저 빌드합니다.
#    cargo가 워크스페이스를 해석하려면 web 매니페스트가 필요하지만, web은 빌드하지 않습니다.
COPY Cargo.toml Cargo.lock ./
COPY shared/Cargo.toml shared/Cargo.toml
COPY backend/Cargo.toml backend/Cargo.toml
COPY web/Cargo.toml web/Cargo.toml
RUN mkdir -p shared/src backend/src web/src \
    && echo "" > shared/src/lib.rs \
    && echo "" > backend/src/lib.rs \
    && echo "fn main() {}" > backend/src/main.rs \
    && echo "fn main() {}" > web/src/main.rs \
    && cargo build --release -p backend

# 2) 실제 소스로 다시 빌드합니다. (sqlx::migrate!()가 backend/migrations를 바이너리에 넣습니다.)
COPY shared/src shared/src
COPY backend/src backend/src
COPY backend/migrations backend/migrations
RUN touch shared/src/lib.rs backend/src/lib.rs backend/src/main.rs \
    && cargo build --release -p backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home app
COPY --from=builder /app/target/release/backend /usr/local/bin/backend
USER app
# Railway가 PORT를 주입합니다. 백엔드는 0.0.0.0:$PORT에서 수신합니다.
ENV PORT=8080
EXPOSE 8080
CMD ["backend"]
