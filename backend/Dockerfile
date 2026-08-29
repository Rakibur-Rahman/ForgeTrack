FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY migrations ./migrations
RUN cargo build --release --package api

FROM debian:bookworm-slim AS production
RUN useradd --system --create-home forge
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/migrations ./migrations
USER forge
EXPOSE 3001
CMD ["api"]

FROM rust:1.85-bookworm AS development
WORKDIR /app
CMD ["cargo", "run", "--package", "api"]
