# Backend build stage
FROM docker.1ms.run/rust:1.94-slim-bookworm AS builder
WORKDIR /build

# Use China crates.io mirror (USTC)
RUN mkdir -p /usr/local/cargo && printf '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"\n' > /usr/local/cargo/config.toml

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
RUN apt-get update && apt-get install -y pkg-config libssl-dev && \
    cargo build --release

# Runtime stage
FROM docker.1ms.run/debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/courier /app/courier
# Frontend is pre-built locally and copied in
COPY web/dist /app/web/dist

VOLUME ["/app/data"]
EXPOSE 9090

ENTRYPOINT ["/app/courier"]
CMD ["/app/config.toml"]
