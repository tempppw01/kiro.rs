# syntax=docker/dockerfile:1.7

# 前端只生成静态文件，与最终镜像架构无关。
# 固定在构建机架构执行，避免多架构发布时通过 QEMU 运行 Node/pnpm。
FROM --platform=$BUILDPLATFORM node:22-alpine AS frontend-builder

WORKDIR /app/admin-ui
COPY admin-ui/package.json admin-ui/pnpm-lock.yaml admin-ui/.npmrc admin-ui/pnpm-workspace.yaml ./
RUN npm install -g pnpm@11
RUN --mount=type=cache,id=kiro-pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile --prefer-offline --reporter=append-only
COPY admin-ui ./
RUN pnpm build

FROM rust:1.92-alpine AS builder

RUN apk add --no-cache musl-dev perl make

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY --from=frontend-builder /app/admin-ui/dist /app/admin-ui/dist

RUN cargo build --release --no-default-features

FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/kiro-rs /app/kiro-rs

VOLUME ["/app/config"]

EXPOSE 8990

CMD ["./kiro-rs", "-c", "/app/config/config.json", "--credentials", "/app/config/credentials.json"]
