/** Zona horaria del salón. Debe coincidir con APP_TZ del backend. */
export const ZONA = 'America/Mexico_City';

const moneda = new Intl.NumberFormat('es-MX', {
  style: 'currency',
  currency: 'MXN',
});

const horaCorta = new Intl.DateTimeFormat('es-MX', {
  hour: '2-digit',
  minute: '2-digit',
  timeZone: ZONA,
});

const fechaLargaFmt = new Intl.DateTimeFormat('es-MX', {
  weekday: 'long',
  day: 'numeric',
  month: 'long',
  timeZone: ZONA,
});

const fechaCortaFmt = new Intl.DateTimeFormat('es-MX', {
  day: '2-digit',
  month: 'short',
  timeZone: ZONA,
});

/** 35000 -> "$350.00" */
export function pesos(centavos: number): string {
  return moneda.format((centavos ?? 0) / 100);
}

/** "2026-11-18T16:30:00Z" -> "10:30" */
export function hora(iso: string): string {
  return horaCorta.format(new Date(iso));
}

/** "2026-11-18T16:30:00Z" -> "miércoles, 18 de noviembre" */
export function fechaLarga(iso: string): string {
  return fechaLargaFmt.format(new Date(iso));
}

/** "2026-11-18T16:30:00Z" -> "18 nov" */
export function fechaCorta(iso: string): string {
  return fechaCortaFmt.format(new Date(iso));
}

/** Fecha de hoy (o del instante dado) como "YYYY-MM-DD" en la zona del salón. */
export function hoyLocal(): string {
  return new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    timeZone: ZONA,
  }).format(new Date());
}

/** "2026-11-18" + 3 -> "2026-11-21" (sin cruzar zonas horarias). */
export function sumarDias(fecha: string, dias: number): string {
  const [a, m, d] = fecha.split('-').map(Number);
  const base = new Date(Date.UTC(a, m - 1, d));
  base.setUTCDate(base.getUTCDate() + dias);
  return base.toISOString().slice(0, 10);
}

/** Nombre del día de la semana a partir de "YYYY-MM-DD". */
export function diaSemana(fecha: string): string {
  const [a, m, d] = fecha.split('-').map(Number);
  return fechaLargaFmt.format(new Date(Date.UTC(a, m - 1, d, 18)));
}

/**
 * Combina "YYYY-MM-DD" y "HH:MM" de la hora local del navegador en un instante
 * ISO en UTC. El navegador de recepción trabaja en la hora del salón.
 */
export function isoDesde(fecha: string, hhmm: string): string {
  return new Date(`${fecha}T${hhmm}:00`).toISOString();
}

/** Inicio del día local como ISO, para filtrar citas en el backend. */
export function inicioDelDia(fecha: string): string {
  return new Date(`${fecha}T00:00:00`).toISOString();
}

/** Inicio del día siguiente local como ISO. */
export function finDelDia(fecha: string): string {
  return new Date(`${sumarDias(fecha, 1)}T00:00:00`).toISOString();
}

/** Escapa texto para insertarlo en plantillas HTML. */
export function esc(texto: unknown): string {
  return String(texto ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}
