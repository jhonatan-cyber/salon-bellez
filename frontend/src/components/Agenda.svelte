<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    type CitaDetalle,
    type Cliente,
    type Estilista,
    type Servicio,
    type Slot,
  } from '../lib/api';
  import {
    diaSemana,
    hora,
    hoyLocal,
    inicioDelDia,
    finDelDia,
    pesos,
    sumarDias,
  } from '../lib/formato';

  let fecha = $state(hoyLocal());
  let filtro = $state('');
  let citas = $state<CitaDetalle[]>([]);
  let estilistas = $state<Estilista[]>([]);
  let servicios = $state<Servicio[]>([]);
  let clientes = $state<Cliente[]>([]);
  let cargando = $state(true);
  let error = $state<string | null>(null);
  let aviso = $state<string | null>(null);

  let panel = $state(false);
  let clienteId = $state('');
  let estilistaId = $state('');
  let seleccion = $state<string[]>([]);
  let notas = $state('');
  let slots = $state<Slot[]>([]);
  let inicioSel = $state<string | null>(null);
  let buscandoSlots = $state(false);
  let guardando = $state(false);
  let nuevoCliente = $state({ nombre: '', telefono: '' });

  let citasFiltradas = $derived(
    filtro ? citas.filter((c) => c.cita.estilista_id === filtro) : citas
  );
  let serviciosElegidos = $derived(servicios.filter((s) => seleccion.includes(s.id)));
  let duracionTotal = $derived(serviciosElegidos.reduce((t, s) => t + s.duracion_min, 0));
  let totalEstimado = $derived(serviciosElegidos.reduce((t, s) => t + s.precio_centavos, 0));

  const colores: Record<string, string> = {
    pendiente: 'bg-arena text-tenue',
    confirmada: 'bg-rosa/70 text-ciruela',
    atendida: 'bg-salvia/15 text-salvia',
    no_show: 'bg-amber-100 text-amber-800',
    cancelada: 'bg-red-100 text-red-700',
  };

  function etiqueta(estado: string): string {
    return estado.replace('_', ' ');
  }

  async function cargarCitas() {
    cargando = true;
    error = null;
    try {
      citas = await api<CitaDetalle[]>(
        `/citas?desde=${encodeURIComponent(inicioDelDia(fecha))}&hasta=${encodeURIComponent(finDelDia(fecha))}`
      );
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo cargar la agenda.';
    } finally {
      cargando = false;
    }
  }

  async function cargarBase() {
    try {
      const [equipo, catalogo, listaClientes] = await Promise.all([
        api<Estilista[]>('/estilistas'),
        api<{ servicios: Servicio[] }>('/servicios'),
        api<Cliente[]>('/clientes'),
      ]);
      estilistas = equipo.filter((e) => e.activo);
      servicios = catalogo.servicios.filter((s) => s.activo);
      clientes = listaClientes;
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo cargar el catálogo.';
    }
  }

  function alternarServicio(id: string) {
    seleccion = seleccion.includes(id)
      ? seleccion.filter((s) => s !== id)
      : [...seleccion, id];
    slots = [];
    inicioSel = null;
  }

  async function verHorarios() {
    if (!estilistaId || serviciosElegidos.length === 0) {
      error = 'Elige estilista y al menos un servicio.';
      return;
    }
    error = null;
    buscandoSlots = true;
    slots = [];
    inicioSel = null;
    try {
      const datos = await api<{ slots: Slot[] }>(
        `/disponibilidad?servicio_id=${serviciosElegidos[0].id}&estilista_id=${estilistaId}` +
          `&fecha=${fecha}&duracion_min=${duracionTotal}`
      );
      slots = datos.slots;
      if (slots.length === 0) error = 'No hay horarios libres ese día para esta estilista.';
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo consultar la disponibilidad.';
    } finally {
      buscandoSlots = false;
    }
  }

  async function crearClienteRapido() {
    if (nuevoCliente.nombre.trim().length < 2) return;
    try {
      const creado = await api<Cliente>('/clientes', {
        metodo: 'POST',
        cuerpo: {
          nombre: nuevoCliente.nombre.trim(),
          telefono: nuevoCliente.telefono.trim() || null,
        },
      });
      clientes = [...clientes, creado];
      clienteId = creado.id;
      nuevoCliente = { nombre: '', telefono: '' };
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo crear el cliente.';
    }
  }

  async function guardar() {
    if (!clienteId || !estilistaId || !inicioSel || seleccion.length === 0) {
      error = 'Faltan datos: cliente, estilista, servicios y horario.';
      return;
    }
    guardando = true;
    error = null;
    try {
      await api<CitaDetalle>('/citas', {
        metodo: 'POST',
        cuerpo: {
          cliente_id: clienteId,
          estilista_id: estilistaId,
          inicio: inicioSel,
          servicio_ids: seleccion,
          notas: notas.trim() || null,
        },
      });
      aviso = 'Cita creada.';
      panel = false;
      clienteId = '';
      estilistaId = '';
      seleccion = [];
      notas = '';
      slots = [];
      inicioSel = null;
      await cargarCitas();
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo crear la cita.';
    } finally {
      guardando = false;
    }
  }

  async function cambiarEstado(id: string, estado: string) {
    error = null;
    try {
      await api(`/citas/${id}/estado`, { metodo: 'POST', cuerpo: { estado } });
      await cargarCitas();
    } catch (e) {
      error = e instanceof Error ? e.message : 'No se pudo actualizar la cita.';
    }
  }

  onMount(async () => {
    await Promise.all([cargarBase(), cargarCitas()]);
  });
</script>

<section class="space-y-5">
  <header class="flex flex-wrap items-end justify-between gap-3">
    <div>
      <h1 class="text-2xl text-ciruela-oscuro">Agenda</h1>
      <p class="text-sm text-tenue">{diaSemana(fecha)}</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <button
        class="btn-suave px-3"
        type="button"
        onclick={() => {
          fecha = sumarDias(fecha, -1);
          void cargarCitas();
        }}>←</button>
      <input
        class="campo w-auto"
        type="date"
        bind:value={fecha}
        onchange={() => void cargarCitas()}
      />
      <button
        class="btn-suave px-3"
        type="button"
        onclick={() => {
          fecha = sumarDias(fecha, 1);
          void cargarCitas();
        }}>→</button>
      <select class="campo w-auto" bind:value={filtro}>
        <option value="">Todo el equipo</option>
        {#each estilistas as e (e.id)}
          <option value={e.id}>{e.nombre}</option>
        {/each}
      </select>
      <button class="btn-primario" type="button" onclick={() => (panel = !panel)}>
        {panel ? 'Cerrar' : 'Nueva cita'}
      </button>
    </div>
  </header>

  {#if error}
    <p class="rounded-xl bg-rosa/60 px-4 py-3 text-sm text-ciruela-oscuro">{error}</p>
  {/if}
  {#if aviso}
    <p class="rounded-xl bg-salvia/15 px-4 py-3 text-sm text-salvia">{aviso}</p>
  {/if}

  {#if panel}
    <div class="card space-y-5">
      <h2 class="text-lg text-ciruela-oscuro">Nueva cita</h2>

      <div class="grid gap-4 md:grid-cols-2">
        <div>
          <label class="etiqueta" for="cliente">Cliente</label>
          <select class="campo" id="cliente" bind:value={clienteId}>
            <option value="">Selecciona…</option>
            {#each clientes as c (c.id)}
              <option value={c.id}>{c.nombre}{c.telefono ? ` · ${c.telefono}` : ''}</option>
            {/each}
          </select>
          <div class="mt-2 flex gap-2">
            <input class="campo" placeholder="Cliente nuevo" bind:value={nuevoCliente.nombre} />
            <input class="campo" placeholder="Teléfono" bind:value={nuevoCliente.telefono} />
            <button class="btn-suave shrink-0" type="button" onclick={crearClienteRapido}>
              Agregar
            </button>
          </div>
        </div>

        <div>
          <label class="etiqueta" for="estilista">Estilista</label>
          <select class="campo" id="estilista" bind:value={estilistaId}>
            <option value="">Selecciona…</option>
            {#each estilistas as e (e.id)}
              <option value={e.id}>{e.nombre}</option>
            {/each}
          </select>
          <label class="etiqueta mt-3" for="notas">Notas</label>
          <input class="campo" id="notas" bind:value={notas} placeholder="Alergias, preferencias…" />
        </div>
      </div>

      <div>
        <p class="etiqueta">Servicios</p>
        <div class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {#each servicios as s (s.id)}
            <label
              class="flex cursor-pointer items-center gap-2 rounded-xl border px-3 py-2 text-sm"
              class:border-ciruela={seleccion.includes(s.id)}
              class:bg-rosa={seleccion.includes(s.id)}
              class:border-arena={!seleccion.includes(s.id)}
            >
              <input
                type="checkbox"
                checked={seleccion.includes(s.id)}
                onchange={() => alternarServicio(s.id)}
              />
              <span class="flex-1">{s.nombre}</span>
              <span class="text-xs text-tenue">{s.duracion_min} min</span>
            </label>
          {/each}
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-3 border-t border-arena pt-4 text-sm">
        <span class="text-tenue">Duración: {duracionTotal} min</span>
        <span class="text-tenue">Total: {pesos(totalEstimado)}</span>
        <button class="btn-suave ml-auto" type="button" onclick={verHorarios}>
          {buscandoSlots ? 'Buscando…' : 'Ver horarios'}
        </button>
      </div>

      {#if slots.length > 0}
        <div class="grid grid-cols-3 gap-2 sm:grid-cols-6 lg:grid-cols-8">
          {#each slots as slot (slot.inicio)}
            <button
              class="rounded-xl border px-2 py-2 text-sm"
              class:bg-ciruela={inicioSel === slot.inicio}
              class:text-crema={inicioSel === slot.inicio}
              class:border-ciruela={inicioSel === slot.inicio}
              class:border-arena={inicioSel !== slot.inicio}
              type="button"
              onclick={() => (inicioSel = slot.inicio)}
            >
              {hora(slot.inicio)}
            </button>
          {/each}
        </div>
      {/if}

      <button class="btn-primario w-full" type="button" disabled={guardando} onclick={guardar}>
        {guardando ? 'Guardando…' : 'Guardar cita'}
      </button>
    </div>
  {/if}

  {#if cargando}
    <p class="text-sm text-tenue">Cargando agenda…</p>
  {:else if citasFiltradas.length === 0}
    <p class="card text-sm text-tenue">No hay citas agendadas para este día.</p>
  {:else}
    <ul class="space-y-2">
      {#each citasFiltradas as c (c.cita.id)}
        <li class="card flex flex-wrap items-center gap-4">
          <div class="w-14 text-center">
            <p class="font-display text-lg text-ciruela-oscuro">{hora(c.cita.inicio)}</p>
            <p class="text-xs text-tenue">{hora(c.cita.fin)}</p>
          </div>
          <div class="min-w-40 flex-1">
            <p class="font-medium">{c.cita.cliente_nombre}</p>
            <p class="text-sm text-tenue">
              {c.servicios.map((s) => s.descripcion).join(' + ') || 'Sin servicios'}
            </p>
            <p class="text-xs text-tenue">
              {c.cita.estilista_nombre} ·
              {c.cita.canal === 'online' ? 'Reserva en línea' : 'Recepción'}
              {c.cita.cliente_telefono ? ` · ${c.cita.cliente_telefono}` : ''}
            </p>
          </div>
          <span class="insignia {colores[c.cita.estado] ?? 'bg-arena text-tenue'}">
            {etiqueta(c.cita.estado)}
          </span>
          <div class="flex flex-wrap gap-1">
            {#if c.cita.estado === 'pendiente'}
              <button
                class="btn-texto"
                type="button"
                onclick={() => cambiarEstado(c.cita.id, 'confirmada')}>Confirmar</button>
            {/if}
            {#if c.cita.estado === 'pendiente' || c.cita.estado === 'confirmada'}
              <button
                class="btn-texto"
                type="button"
                onclick={() => cambiarEstado(c.cita.id, 'atendida')}>Atendida</button>
              <button
                class="btn-texto"
                type="button"
                onclick={() => cambiarEstado(c.cita.id, 'no_show')}>No asistió</button>
              <button
                class="btn-texto text-red-700"
                type="button"
                onclick={() => cambiarEstado(c.cita.id, 'cancelada')}>Cancelar</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
