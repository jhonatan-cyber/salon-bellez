# Salón de belleza — sistema de agenda, caja e inventario

Sistema para un solo local: agenda con reserva pública sin cuenta, catálogo de servicios,
clientes, caja con cierre diario, inventario y reportes. Interfaz en español, precios en MXN y
zona horaria configurable (`America/Mexico_City` por defecto).

- **Backend**: Rust 1.100 · axum 0.8.9 · sqlx 0.9 (Postgres, consultas en runtime) · PostgreSQL 18
- **Frontend**: Astro 7.4 (salida estática) · Bun 1.4 · Tailwind CSS 4.3 · islas Svelte 5 (agenda y caja)
- **Despliegue**: Docker Compose (Postgres + API + Caddy) en un VPS, o Neon como Postgres gestionado

## Estructura

```
backend/            API axum
  migrations/       0001_init.sql (esquema + anti-doble-reserva), 0002_seed.sql (datos demo)
  src/main.rs       arranque, migraciones, admin inicial, estático opcional
  src/routes.rs     rutas /api y guardia de origen
  src/handlers/     salud, auth, catálogo, staff, disponibilidad, citas, ventas, reportes, público
frontend/           Astro estático (Bun)
  src/pages/        index, reservar, login, app/{agenda,clientes,servicios,caja}
  src/components/   Agenda.svelte, Caja.svelte (islas)
  src/lib/          api.ts (cliente tipado), formato.ts (MXN y fechas), sesion.ts
docker-compose.yml  db + api + web
Caddyfile           frontend estático y proxy /api -> api:3000
```

## Puesta en marcha

### Con Docker Compose (recomendado)

```bash
cp .env.example .env      # ajusta JWT_SECRET, ADMIN_EMAIL y ADMIN_PASSWORD
docker compose up --build
```

- Frontend y API en un solo origen: <http://localhost:8080>
- Postgres escucha en `localhost:5432` (usuario/clave `salon` por defecto)
- Las migraciones corren solas al arrancar la API
- La administradora inicial se crea con `ADMIN_EMAIL`/`ADMIN_PASSWORD` si no existe

Para un dominio real: cambia `:80` por tu dominio en `Caddyfile`, publica 80/443 y define
`APP_ORIGIN=https://tu-dominio.mx` (Caddy emite el certificado TLS automáticamente).

> Con `APP_ENV=production` la cookie de sesión se marca `Secure` y el navegador no la guarda
> sobre `http://`. Si pruebas el stack en `http://localhost:8080`, usa `APP_ENV=development`
> en `.env`; en el VPS con HTTPS déjalo en `production`.

### En desarrollo

Requisitos: Rust 1.100 (`rust-toolchain.toml`), Bun 1.4 y un Postgres 18 en `localhost:5432`.

```bash
# 1. Base de datos (o usa solo el servicio db de Compose)
docker compose up -d db

# 2. API  (http://localhost:3000)
cd backend
export DATABASE_URL=postgres://salon:salon@localhost:5432/salon
export ADMIN_EMAIL=admin@salon.mx ADMIN_PASSWORD=una-clave-larga
cargo run

# 3. Frontend (http://localhost:4321)
cd frontend
bun install
bun run dev          # o: bunx --bun astro dev --background
```

En desarrollo Vite hace proxy de `/api` hacia `http://localhost:3000`
(`astro.config.mjs`), así que no hay CORS y la cookie de sesión funciona igual que en
producción. Si el puerto 3000 está ocupado, arranca la API en otro puerto y pasa `API_URL`:

```bash
PORT=3100 cargo run
API_URL=http://localhost:3100 bun run dev
```

> sqlx embebe las migraciones al compilar. Si agregas un archivo nuevo en `backend/migrations`,
> toca `backend/src/db.rs` (o `cargo clean -p salon-api`) para que el binario lo incluya.

Migraciones manuales, si las necesitas:

```bash
cd backend
sqlx migrate run --database-url postgres://salon:salon@localhost:5432/salon
```

### Verificación

```bash
cd backend  && cargo clippy --all-targets -- -D warnings && cargo test
cd frontend && bunx --bun astro check && bunx --bun astro build
curl http://localhost:3000/api/health
```

## Variables de entorno

| Variable | Uso | Por defecto |
| --- | --- | --- |
| `DATABASE_URL` | Cadena de Postgres | `postgres://salon:salon@localhost:5432/salon` |
| `JWT_SECRET` | Firma HS256 de la cookie (≥32 caracteres) | — (obligatoria en producción) |
| `PORT` | Puerto de la API | `3000` |
| `APP_ENV` | `development` / `production` | `development` |
| `APP_ORIGIN` | Origen permitido para mutaciones y cookies seguras | `http://localhost:4321` |
| `APP_TZ` | Zona horaria del salón | `America/Mexico_City` |
| `ADMIN_EMAIL`, `ADMIN_PASSWORD` | Administradora inicial (≥8 caracteres) | — |
| `RUST_LOG` | Nivel de logs | `salon_api=info,tower_http=warn` |

## Roles

| Rol | Puede |
| --- | --- |
| `admin` | Todo, incluidos usuarios y borrar servicios |
| `recepcionista` | Agenda, clientes, servicios (sin borrar), productos, caja, reportes |
| `estilista` | Consultar agenda, clientes y catálogo; cambiar el estado de una cita |

La sesión es una cookie **httpOnly** `salon_session` con JWT HS256 de 12 horas
(`SameSite=Lax`, `Secure` en producción). El token nunca se guarda en JavaScript ni en
`localStorage`.

## API

Todo vive bajo `/api`. Documentación interactiva en `/api/docs` (Swagger UI) con el esquema en
`/api/openapi.json`.

| Método y ruta | Descripción |
| --- | --- |
| `GET /health` | Estado del servicio |
| `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`, `POST /auth/password` | Sesión de staff |
| `GET/POST /servicios`, `PUT/DELETE /servicios/{id}`, `POST /categorias` | Catálogo |
| `GET/POST /clientes`, `GET/PUT /clientes/{id}?q=` | Clientes e historial |
| `GET/POST /estilistas`, `PUT /estilistas/{id}` | Equipo |
| `GET/POST /estilistas/{id}/horarios`, `DELETE /horarios/{id}` | Horarios semanales |
| `GET/POST /estilistas/{id}/bloqueos`, `DELETE /bloqueos/{id}` | Bloqueos puntuales |
| `GET/POST /festivos`, `DELETE /festivos/{id}` | Días cerrados |
| `GET /disponibilidad?servicio_id&estilista_id&fecha[&duracion_min]` | Huecos libres de 15 min |
| `GET/POST /citas`, `GET/PATCH /citas/{id}`, `POST /citas/{id}/estado` | Agenda |
| `GET/POST /ventas`, `GET /ventas/{id}` | Ventas con líneas y pagos |
| `GET /caja/resumen?fecha`, `POST /caja/cierre`, `GET /caja/cierres` | Caja del día |
| `GET/POST /productos`, `PUT /productos/{id}`, `POST /productos/{id}/ajuste`, `GET /productos/{id}/movimientos` | Inventario |
| `GET /reportes/{resumen,ingresos-por-dia,por-servicio,por-estilista,top-clientes}` | Reportes por rango |
| `GET /public/servicios`, `GET /public/estilistas`, `GET /public/disponibilidad` | Portal sin cuenta |
| `POST /public/reservas` | Reserva pública (rate limit + honeypot) |

## Reglas de negocio

- **Anti-doble-reserva en la base**: `EXCLUDE USING gist` sobre `(estilista_id, tstzrange(inicio, fin))`
  con los estados activos. Dos recepcionistas no pueden ocupar el mismo hueco ni por error;
  la API responde `409` con «ya existe una cita en ese horario».
- **Disponibilidad**: franjas de `horarios_estilista` menos citas activas y bloqueos, en pasos
  de 15 minutos. `duracion_min` permite cotizar una cita de varios servicios con su duración total.
- **Zona horaria**: la base guarda `timestamptz`; las fechas de agenda, caja y reportes se
  interpretan en `APP_TZ`. El frontend formatea en la misma zona (`src/lib/formato.ts`).
- **Portal público**: sin cuenta. Límite de 10 reservas por minuto por IP, campo trampa
  (`website`) y validación de teléfono o correo. Un cliente se reutiliza por teléfono o correo.
- **Mutaciones**: `origin_guard` rechaza peticiones con `Origin` ajeno al salón.
- **Caja**: al cobrar una cita se marcan sus servicios como `atendida`, se descuentan los
  productos vendidos y los insumos de `consumo_servicio`. El cierre diario cuadra por método de pago.
- **Inventario**: cada cambio de stock deja un movimiento (`ajuste`, `venta`, `consumo`).

## Decisiones y desviaciones del plan

- **Consultas en runtime** (`query_as`) en lugar de macros compile-time: el binario se compila
  sin `DATABASE_URL`. El precio es que los errores de SQL aparecen al ejecutar, no al compilar.
- **Sin `utoipa`**: `/api/openapi.json` está escrito a mano y `/api/docs` carga Swagger UI desde
  CDN. En un VPS sin salida a internet conviene servirlo local.
- **Cookie propia en lugar de sesiones en base**: el JWT firmado evita una tabla de sesiones;
  cerrar sesión borra la cookie (no hay revocación por servidor).
- **Rate limit en memoria**: suficiente para un local; si algún día hay varias instancias hace
  falta un almacén compartido (Redis).
- **Astro estático**: no hay SSR. La protección del panel es del lado del cliente y la
  autorización real la hace siempre la API.

## Fuera de alcance (fase 2)

Recordatorios por WhatsApp/email, comisiones, programa de lealtad, fotos antes/después,
multi-sucursal, PWA y pagos en línea.
