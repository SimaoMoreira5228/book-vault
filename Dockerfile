# ---- Stage 1: Build Rust API ----
FROM rust:1.85-slim-bookworm AS api-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY api/ .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/book-vault /book-vault

# ---- Stage 2: Build SvelteKit frontend ----
FROM node:22-alpine AS web-builder

WORKDIR /web
COPY web/ .
RUN corepack enable && pnpm install --frozen-lockfile && pnpm build

# ---- Stage 3: Runtime ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=api-builder /book-vault /usr/local/bin/book-vault
COPY --from=web-builder /web/build /usr/share/bookvault/web

EXPOSE 8080

ENV BOOKVAULT_CONFIG=/etc/bookvault/bookvault.toml
ENV BOOKVAULT_WEB_DIR=/usr/share/bookvault/web

VOLUME ["/data", "/storage"]

CMD ["book-vault"]
