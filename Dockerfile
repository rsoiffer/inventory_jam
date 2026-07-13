FROM rust:1.96-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY vendor ./vendor
COPY src ./src
RUN cargo build --release --no-default-features --features=server,netcode,webtransport

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/inventory_jam /app/inventory_jam
COPY secrets/database-server-url.txt /app/secrets/database-server-url.txt
ENTRYPOINT ["/app/inventory_jam", "--mode", "server"]
