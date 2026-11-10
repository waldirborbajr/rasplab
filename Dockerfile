# Stage 1 — cross-compile para arm64 na sua máquina
FROM --platform=$BUILDPLATFORM rust:1.82 AS builder

RUN apt-get update && apt-get install -y \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /app
COPY . .

RUN cargo build --release --target aarch64-unknown-linux-musl

# Stage 2 — imagem final mínima (sem Rust, sem toolchain)
# FROM debian:bookworm-slim

# Stage 2 — scratch: zero OS, static binary only
FROM scratch

WORKDIR /app
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/rasplab .

EXPOSE 9443

CMD ["/app/rasplab", "start"]


# Stage 2 — scratch: zero OS, static binary only
FROM scratch
