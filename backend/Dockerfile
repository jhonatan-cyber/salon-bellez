# Binario de la API en Rust 1.100 (sqlx con consultas en runtime: no necesita DATABASE_URL al compilar).
FROM rust:1.100-slim AS build

RUN apt-get update \
	&& apt-get install -y --no-install-recommends pkg-config \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY rust-toolchain.toml ./
COPY backend ./backend

WORKDIR /src/backend
RUN cargo build --release --locked --bin salon-api

FROM debian:trixie-slim

RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /src/backend/target/release/salon-api /app/salon-api

ENV PORT=3000 APP_ENV=production
EXPOSE 3000

CMD ["/app/salon-api"]
