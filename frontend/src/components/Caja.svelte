<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    type CitaDetalle,
    type Producto,
    type ResumenCaja,
    type Venta,
    type VentaDetalle,
  } from '../lib/api';
  import { finDelDia, hora, hoyLocal, inicioDelDia, pesos } from '../lib/formato';

  let fecha = $state(hoyLocal());
  let citas = $state<CitaDetalle[]>([]);
  let productos = $state<Producto[]>([]);
  let ventas = $state<Venta[]>([]);
  let resumen = $state<ResumenCaja | null>(null);

  let seleccion = $state<CitaDetalle | null>(null);
  let extras = $state<{ producto: Producto; cantidad: number }[]>([]);
  let descuento = $state(0);
  let propina = $state(0);
  let metodo = $state<'efectivo' | 'tarjeta' | 'transferencia'>('efectivo');
  let productoElegido = $state('');
  let guardando = $state(false);
  let error = $state<string | null>(null);
  let aviso = $state<string | null>(null);
  let ultimaVenta = $state<VentaDetalle | null>(null);

  const metodos = [
    { valor: 'efectivo', texto: 'Efectivo' },
    { valor: 'tarjeta', texto: 'Tarjeta' },
    { valor: 'transferencia', texto: 'Transferencia' },
  ] as const;

  let subtotalServicios = $derived(
    seleccion ? seleccion.servicios.reduce((t, s) => t + s.precio_centavos, 0) : 0
  );
  let subtotalProductos = $derived(
    extras.reduce((t, e) => t + e.producto.precio_centavos * e.cantidad, 0)
  );
  let subtotal = $derived(subtotalServicios + subtotalProductos);
  let descuentoCentavos = $derived(Math.round((descuento || 0) * 100));
  let propinaCentavos = $derived(Math.round((propina || 0) * 100));
  let total = $derived(Math.max(0, subtotal - descuentoCentavos + propinaCentavos));

  async function cargar() {
    error = null;
    try {
      const [agenda, catalogo, listaVentas, totales] = await Promise.all([
        api<CitaDetalle[]>(
          `/citas?desde=${encodeURIComponent(inicioDelDia(fecha))}&hasta=${encodeURIComponent(
            finDelDia(fecha)
          )}`
        ),
        api<Producto[]>('/productos'),
        api<Venta[]>(`/ventas?desde=${fecha}&hasta=${fecha}`),
        api<ResumenCaja>(`/caja/resumen?fecha=${fecha}`),
      ]);
      citas = agenda.filter((c) => c.cita.estado !== 'cancelada');
      productos = catalogo.filter((p) => p.activo);
      ventas = listaVentas;
      resumen = totales;
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo cargar la caja.';
    }
  }

  function elegir(cita: CitaDetalle) {
    seleccion = cita;
    extras = [];
    descuento = 0;
    propina = 0;
    aviso = null;
    ultimaVenta = null;
  }

  function agregarProducto() {
    const producto = productos.find((p) => p.id === productoElegido);
    if (!producto) return;
    const existente = extras.find((e) => e.producto.id === producto.id);
    extras = existente
      ? extras.map((e) =>
          e.producto.id === producto.id ? { ...e, cantidad: e.cantidad + 1 } : e
        )
      : [...extras, { producto, cantidad: 1 }];
    productoElegido = '';
  }

  function quitarExtra(id: string) {
    extras = extras.filter((e) => e.producto.id !== id);
  }

  async function cobrar() {
    if (!seleccion) {
      error = 'Elige la cita que vas a cobrar.';
      return;
    }
    guardando = true;
    error = null;
    aviso = null;
    try {
      const detalle = await api<VentaDetalle>('/ventas', {
        metodo: 'POST',
        cuerpo: {
          cita_id: seleccion.cita.id,
          lineas: extras.map((e) => ({
            tipo: 'producto',
            producto_id: e.producto.id,
            descripcion: e.producto.nombre,
            cantidad: e.cantidad,
            precio_unitario_centavos: e.producto.precio_centavos,
          })),
          descuento_centavos: descuentoCentavos,
          propina_centavos: propinaCentavos,
          pagos: [{ metodo, monto_centavos: total }],
        },
      });
      ultimaVenta = detalle;
      aviso = `Venta registrada por ${pesos(detalle.venta.total_centavos)}.`;
      seleccion = null;
      extras = [];
      descuento = 0;
      propina = 0;
      await cargar();
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo registrar la venta.';
    } finally {
      guardando = false;
    }
  }

  async function cerrarCaja() {
    if (!confirm(`¿Cerrar la caja del ${fecha}?`)) return;
    error = null;
    try {
      const cierre = await api<{ total_centavos: number; efectivo_centavos: number }>(
        '/caja/cierre',
        { metodo: 'POST', cuerpo: { fecha } }
      );
      aviso = `Cierre guardado: ${pesos(cierre.total_centavos)} (efectivo ${pesos(
        cierre.efectivo_centavos
      )}).`;
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo cerrar la caja.';
    }
  }

  onMount(async () => {
    await cargar();
  });
</script>

<section class="space-y-6">
  <header class="flex flex-wrap items-end justify-between gap-3">
    <div>
      <h1 class="text-2xl text-ciruela-oscuro">Caja</h1>
      <p class="text-sm text-tenue">Cobra servicios y productos del día.</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <input class="campo w-auto" type="date" bind:value={fecha} onchange={() => void cargar()} />
      <button class="btn-suave" type="button" onclick={() => void cargar()}>Actualizar</button>
      <button class="btn-primario" type="button" onclick={cerrarCaja}>Cerrar caja</button>
    </div>
  </header>

  {#if error}
    <p class="rounded-xl bg-rosa/60 px-4 py-3 text-sm text-ciruela-oscuro">{error}</p>
  {/if}
  {#if aviso}
    <p class="rounded-xl bg-salvia/15 px-4 py-3 text-sm text-salvia">{aviso}</p>
  {/if}

  <div class="grid gap-4 sm:grid-cols-3">
    <div class="card">
      <p class="etiqueta">Ingresos del día</p>
      <p class="font-display text-2xl text-ciruela-oscuro">{pesos(resumen?.total_centavos ?? 0)}</p>
    </div>
    <div class="card">
      <p class="etiqueta">Tickets</p>
      <p class="font-display text-2xl text-ciruela-oscuro">{resumen?.tickets ?? 0}</p>
    </div>
    <div class="card">
      <p class="etiqueta">Ticket promedio</p>
      <p class="font-display text-2xl text-ciruela-oscuro">
        {pesos(
          resumen && resumen.tickets > 0 ? Math.round(resumen.total_centavos / resumen.tickets) : 0
        )}
      </p>
    </div>
  </div>

  <div class="grid gap-5 lg:grid-cols-[1fr_1.1fr]">
    <div class="card space-y-4">
      <h2 class="text-lg text-ciruela-oscuro">Cobrar cita</h2>

      {#if !seleccion}
        <div class="max-h-72 space-y-2 overflow-y-auto pr-1">
          {#each citas as c (c.cita.id)}
            <button
              type="button"
              class="flex w-full items-center justify-between gap-3 rounded-xl border border-arena px-3 py-2 text-left text-sm hover:border-ciruela"
              onclick={() => elegir(c)}
            >
              <span>
                <span class="font-medium">{c.cita.cliente_nombre}</span>
                <span class="block text-xs text-tenue">
                  {hora(c.cita.inicio)} · {c.cita.estilista_nombre}
                </span>
              </span>
              <span class="text-xs text-tenue">{c.cita.estado}</span>
            </button>
          {/each}
          {#if citas.length === 0}
            <p class="text-sm text-tenue">No hay citas para este día.</p>
          {/if}
        </div>
      {:else}
        <div class="rounded-xl bg-arena/40 px-3 py-2 text-sm">
          <p class="font-medium">{seleccion.cita.cliente_nombre}</p>
          <p class="text-xs text-tenue">
            {hora(seleccion.cita.inicio)} · {seleccion.cita.estilista_nombre}
          </p>
        </div>

        <ul class="space-y-1 text-sm">
          {#each seleccion.servicios as s (s.servicio_id)}
            <li class="flex justify-between">
              <span>{s.descripcion}</span>
              <span class="text-tenue">{pesos(s.precio_centavos)}</span>
            </li>
          {/each}
        </ul>

        <div class="flex gap-2">
          <select class="campo" bind:value={productoElegido}>
            <option value="">Agregar producto…</option>
            {#each productos as p (p.id)}
              <option value={p.id}>{p.nombre} · {pesos(p.precio_centavos)}</option>
            {/each}
          </select>
          <button class="btn-suave shrink-0" type="button" onclick={agregarProducto}>
            Agregar
          </button>
        </div>

        {#if extras.length > 0}
          <ul class="space-y-1 text-sm">
            {#each extras as e (e.producto.id)}
              <li class="flex items-center justify-between">
                <span>{e.cantidad}× {e.producto.nombre}</span>
                <span class="flex items-center gap-2">
                  <span class="text-tenue">{pesos(e.producto.precio_centavos * e.cantidad)}</span>
                  <button
                    class="text-xs text-red-700"
                    type="button"
                    onclick={() => quitarExtra(e.producto.id)}>quitar</button>
                </span>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="grid gap-3 sm:grid-cols-2">
          <div>
            <label class="etiqueta" for="descuento">Descuento (MXN)</label>
            <input
              class="campo"
              id="descuento"
              type="number"
              min="0"
              step="0.01"
              bind:value={descuento}
            />
          </div>
          <div>
            <label class="etiqueta" for="propina">Propina (MXN)</label>
            <input
              class="campo"
              id="propina"
              type="number"
              min="0"
              step="0.01"
              bind:value={propina}
            />
          </div>
        </div>

        <div>
          <p class="etiqueta">Método de pago</p>
          <div class="flex flex-wrap gap-2">
            {#each metodos as m (m.valor)}
              <button
                class="btn-suave"
                class:bg-ciruela={metodo === m.valor}
                class:text-crema={metodo === m.valor}
                class:border-ciruela={metodo === m.valor}
                type="button"
                onclick={() => (metodo = m.valor)}>{m.texto}</button>
            {/each}
          </div>
        </div>

        <dl class="space-y-1 border-t border-arena pt-3 text-sm">
          <div class="flex justify-between"><dt class="text-tenue">Subtotal</dt><dd>{pesos(subtotal)}</dd></div>
          <div class="flex justify-between"><dt class="text-tenue">Descuento</dt><dd>-{pesos(descuentoCentavos)}</dd></div>
          <div class="flex justify-between"><dt class="text-tenue">Propina</dt><dd>{pesos(propinaCentavos)}</dd></div>
          <div class="flex justify-between text-base font-medium">
            <dt>Total</dt><dd>{pesos(total)}</dd>
          </div>
        </dl>

        <div class="flex gap-2">
          <button class="btn-primario flex-1" type="button" disabled={guardando} onclick={cobrar}>
            {guardando ? 'Registrando…' : `Cobrar ${pesos(total)}`}
          </button>
          <button
            class="btn-suave"
            type="button"
            onclick={() => {
              seleccion = null;
              extras = [];
            }}>Cancelar</button>
        </div>
      {/if}
    </div>

    <div class="card space-y-3">
      <h2 class="text-lg text-ciruela-oscuro">Ventas del día</h2>
      {#if ventas.length === 0}
        <p class="text-sm text-tenue">Todavía no hay ventas registradas.</p>
      {:else}
        <ul class="divide-y divide-arena text-sm">
          {#each ventas as v (v.id)}
            <li class="flex items-center justify-between py-2">
              <div>
                <p class="font-medium">{pesos(v.total_centavos)}</p>
                <p class="text-xs text-tenue">
                  {v.pagada_en ? hora(v.pagada_en) : hora(v.creada_en)} · {v.estado}
                </p>
              </div>
              <span class="text-xs text-tenue">{v.cita_id ? 'Cita' : 'Mostrador'}</span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if ultimaVenta}
        <div class="rounded-xl bg-arena/40 p-3 text-sm">
          <p class="font-medium">Último ticket</p>
          <ul class="mt-1 space-y-1">
            {#each ultimaVenta.lineas as l (l.id)}
              <li class="flex justify-between text-xs">
                <span>{l.cantidad}× {l.descripcion}</span>
                <span class="text-tenue">{pesos(l.total_centavos)}</span>
              </li>
            {/each}
          </ul>
          <p class="mt-2 flex justify-between font-medium">
            <span>Total</span><span>{pesos(ultimaVenta.venta.total_centavos)}</span>
          </p>
        </div>
      {/if}
    </div>
  </div>
</section>
