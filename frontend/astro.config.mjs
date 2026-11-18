// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';

// URL del backend en desarrollo (cámbiala con API_URL si el puerto 8000 está ocupado).
const apiUrl = process.env.API_URL ?? 'http://localhost:8000';

// https://astro.build/config
export default defineConfig({
  integrations: [svelte()],
  vite: {
    plugins: [tailwindcss()],
    server: {
      proxy: {
        // En desarrollo el frontend habla con el backend axum en :8000.
        // En producción Caddy hace el mismo trabajo en un solo origen.
        '/api': {
          target: apiUrl,
          changeOrigin: true,
        },
      },
    },
  },
});
