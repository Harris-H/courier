# Build stage
FROM rust:1.82-slim-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/

RUN apt-get update && apt-get install -y pkg-config libssl-dev && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/courier /app/courier

ENTRYPOINT ["/app/courier"]
CMD ["/app/config.toml"]
