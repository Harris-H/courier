# Backend build stage
ARG RUST_IMAGE=docker.1ms.run/rust:1.94-slim-bookworm
ARG DEBIAN_IMAGE=docker.1ms.run/debian:bookworm-slim
FROM ${RUST_IMAGE} AS builder
WORKDIR /build

# Use China crates.io mirror (USTC)
RUN mkdir -p /usr/local/cargo && printf '[source.crates-io]\nreplace-with = "ustc"\n[source.ustc]\nregistry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"\n' > /usr/local/cargo/config.toml

COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
RUN apt-get update && apt-get install -y pkg-config libssl-dev && \
    cargo build --release

# Runtime stage
FROM ${DEBIAN_IMAGE}

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
