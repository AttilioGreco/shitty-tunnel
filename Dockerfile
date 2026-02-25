FROM oven/bun:1-alpine AS frontend-builder

WORKDIR /frontend
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile
COPY frontend/ ./
RUN bun run build:embed

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev protobuf-dev protoc

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY --from=frontend-builder /frontend/dist ./frontend/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry   \
    --mount=type=cache,target=/usr/local/cargo/git        \
    --mount=type=cache,target=/build/target               \
    cargo build --release --bin shitty-tunnel          && \
    cp target/release/shitty-tunnel /shitty-tunnel

FROM alpine:latest
RUN apk add --no-cache ca-certificates

RUN adduser -D -u 1000 shitty
COPY --from=builder /shitty-tunnel /usr/local/bin/shitty-tunnel

USER shitty
WORKDIR /home/shitty

ENTRYPOINT ["/usr/local/bin/shitty-tunnel"]
CMD ["server", "--config", "/etc/shittyTunnel/server.toml"]

