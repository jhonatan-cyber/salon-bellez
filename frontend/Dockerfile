# Compila el frontend con Bun y lo sirve con Caddy (que además hace proxy de /api).
FROM oven/bun:1 AS build

WORKDIR /app
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile

COPY frontend/ ./
RUN bun run build

FROM caddy:2.11-alpine

COPY Caddyfile /etc/caddy/Caddyfile
COPY --from=build /app/dist /srv

EXPOSE 80
