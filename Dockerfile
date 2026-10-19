FROM rust:1.90-bookworm AS development
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY packages ./packages
COPY services ./services
COPY migrations ./migrations
RUN cargo build --locked --workspace

FROM development AS builder
RUN cargo build --locked --release --workspace

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ctc-api /usr/local/bin/ctc-api
COPY --from=builder /app/target/release/ctc-worker /usr/local/bin/ctc-worker
USER 10001:10001
EXPOSE 8080
CMD ["ctc-api"]
