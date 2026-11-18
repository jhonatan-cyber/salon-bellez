export type Rol = 'admin' | 'recepcionista' | 'estilista';

export interface Usuario {
  id: string;
  email: string;
  nombre: string;
  rol: Rol;
  activo: boolean;
}

export interface Servicio {
  id: string;
  categoria_id: string | null;
  nombre: string;
  descripcion: string | null;
  duracion_min: number;
  buffer_min: number;
  precio_centavos: number;
  activo: boolean;
}

export interface Categoria {
  id: string;
  nombre: string;
  orden: number;
}

export interface Cliente {
  id: string;
  nombre: string;
  telefono: string | null;
  email: string | null;
  notas: string | null;
  acepta_marketing: boolean;
  creado_en: string;
}

export interface Estilista {
  id: string;
  usuario_id: string | null;
  nombre: string;
  telefono: string | null;
  color: string;
  activo: boolean;
}

export interface Horario {
  id: string;
  estilista_id: string;
  dia_semana: number;
  /** "HH:MM:SS" */
  hora_inicio: string;
  hora_fin: string;
}

export interface Slot {
  inicio: string;
  fin: string;
}

export interface Cita {
  id: string;
  cliente_id: string;
  cliente_nombre: string;
  cliente_telefono: string | null;
  estilista_id: string;
  estilista_nombre: string;
  inicio: string;
  fin: string;
  estado: string;
  notas: string | null;
  canal: string;
}

export interface CitaServicio {
  cita_id: string;
  servicio_id: string;
  descripcion: string;
  duracion_min: number;
  precio_centavos: number;
}

export interface CitaDetalle {
  cita: Cita;
  servicios: CitaServicio[];
}

export interface Producto {
  id: string;
  nombre: string;
  sku: string | null;
  precio_centavos: number;
  costo_centavos: number;
  stock: number;
  stock_minimo: number;
  activo: boolean;
}

export interface Venta {
  id: string;
  cita_id: string | null;
  cliente_id: string | null;
  subtotal_centavos: number;
  descuento_centavos: number;
  propina_centavos: number;
  total_centavos: number;
  estado: string;
  creada_en: string;
  pagada_en: string | null;
}

export interface VentaLinea {
  id: string;
  tipo: 'servicio' | 'producto';
  descripcion: string;
  cantidad: number;
  precio_unitario_centavos: number;
  total_centavos: number;
}

export interface Pago {
  id: string;
  metodo: string;
  monto_centavos: number;
  referencia: string | null;
}

export interface VentaDetalle {
  venta: Venta;
  lineas: VentaLinea[];
  pagos: Pago[];
}

export interface ResumenCaja {
  fecha: string;
  total_centavos: number;
  tickets: number;
}

/** Error de la API con el mensaje que mandó el backend en español. */
export class ErrorApi extends Error {}

interface Opciones {
  metodo?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  cuerpo?: unknown;
}

/**
 * Cliente mínimo de la API. Usa rutas relativas (`/api/...`) y manda la cookie
 * de sesión, que es httpOnly: el navegador nunca ve el token.
 */
export async function api<T>(ruta: string, opciones: Opciones = {}): Promise<T> {
  const { metodo = 'GET', cuerpo } = opciones;
  const res = await fetch(`/api${ruta}`, {
    method: metodo,
    credentials: 'include',
    headers: cuerpo === undefined ? {} : { 'Content-Type': 'application/json' },
    body: cuerpo === undefined ? undefined : JSON.stringify(cuerpo),
  });

  const texto = await res.text();
  let datos: unknown = null;
  if (texto) {
    try {
      datos = JSON.parse(texto);
    } catch {
      datos = null;
    }
  }

  if (!res.ok) {
    const mensaje =
      datos && typeof datos === 'object' && 'error' in datos
        ? String((datos as { error: unknown }).error)
        : `Error ${res.status}`;
    throw new ErrorApi(mensaje);
  }

  return datos as T;
}
