FROM rust:slim-bookworm AS api-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY api/ .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/book-vault /book-vault

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=api-builder /book-vault /usr/local/bin/book-vault

EXPOSE 3761

ENV BOOKVAULT_CONFIG=/etc/bookvault/bookvault.toml

VOLUME ["/data", "/storage"]

CMD ["book-vault"]
