# Frontend (Astro + Svelte + Tailwind)

Salida **estática**. Las páginas consumen la API en `/api` (proxy de Astro en desarrollo).

```bash
bun install
bun run dev                 # http://localhost:4321
API_URL=http://localhost:3100 bun run dev   # si el backend no está en :8000
bunx --bun astro check
bunx --bun astro build      # genera dist/
```

Páginas:

- `/` — sitio público con servicios y equipo (`/api/public/*`)
- `/reservar` — reserva sin cuenta (servicio → estilista → horario → datos)
- `/login` — acceso del equipo; la cookie `salon_session` es httpOnly
- `/app/inicio` — dashboard del panel (KPIs, agenda del día, ingresos, equipo e inventario)
- `/app/agenda`, `/app/clientes`, `/app/servicios`, `/app/equipo`, `/app/caja` — panel interno

Detalles de la instalación completa (Postgres y API) en el `README.md` de la raíz.
