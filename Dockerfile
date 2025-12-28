FROM rust:1.91.1 AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
WORKDIR /app

COPY --from=builder /app/target/release/pump_dump_scanner .

CMD ["./pump_dump_scanner"]
