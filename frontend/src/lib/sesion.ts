import { api, type Rol, type Usuario } from './api';

/** Devuelve el usuario de la sesión o null si no hay cookie válida. */
export async function obtenerSesion(): Promise<Usuario | null> {
  try {
    const res = await api<{ usuario: Usuario }>('/auth/me');
    return res.usuario;
  } catch {
    return null;
  }
}

/** Protege una página del panel: si no hay sesión, manda a /login. */
export async function protegerPagina(): Promise<Usuario | null> {
  const usuario = await obtenerSesion();
  if (!usuario) {
    const siguiente = encodeURIComponent(location.pathname + location.search);
    location.replace(`/login?siguiente=${siguiente}`);
    return null;
  }
  document.querySelectorAll<HTMLElement>('[data-usuario]').forEach((el) => {
    el.textContent = usuario.nombre;
  });
  document.querySelectorAll<HTMLElement>('[data-solo-admin]').forEach((el) => {
    el.hidden = usuario.rol !== 'admin';
  });
  return usuario;
}

/** admin y recepcionista pueden editar agenda, catálogo y caja. */
export function puedeEditar(rol: Rol): boolean {
  return rol === 'admin' || rol === 'recepcionista';
}

export async function cerrarSesion(): Promise<void> {
  try {
    await api('/auth/logout', { metodo: 'POST' });
  } finally {
    location.replace('/login');
  }
}
