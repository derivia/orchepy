# builder
FROM rust:bookworm AS builder

WORKDIR /usr/src/orchepy

COPY Cargo.toml ./

RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN cargo build --release || true

COPY . .

RUN cargo install sqlx-cli --version 0.7.4 --no-default-features --features rustls,postgres

RUN cargo build --release

# runner
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/orchepy/target/release/orchepy .
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx

COPY ./src/db/migrations ./src/db/migrations

EXPOSE 3296

CMD ["./orchepy"]

