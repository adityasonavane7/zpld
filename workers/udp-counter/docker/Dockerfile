FROM rust:1.96 AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin zpld-supervisor --bin zpldctl --bin udp-counter

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/zpld-supervisor /usr/sbin/
COPY --from=builder /build/target/release/zpldctl /usr/bin/
COPY --from=builder /build/target/release/udp-counter /usr/sbin/
COPY workers/udp-counter/docker/supervisor.toml /etc/zpld/supervisor.toml
RUN mkdir -p /run/zpld
EXPOSE 9000/udp
