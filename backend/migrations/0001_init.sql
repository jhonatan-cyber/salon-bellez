-- Esquema inicial del sistema de salón de belleza
CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE TABLE usuarios (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email           text NOT NULL,
    nombre          text NOT NULL,
    rol             text NOT NULL DEFAULT 'recepcionista'
                    CHECK (rol IN ('admin', 'recepcionista', 'estilista')),
    password_hash   text NOT NULL,
    activo          boolean NOT NULL DEFAULT true,
    creado_en       timestamptz NOT NULL DEFAULT now(),
    actualizado_en  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX usuarios_email_key ON usuarios (lower(email));

CREATE TABLE estilistas (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    usuario_id  uuid REFERENCES usuarios (id) ON DELETE SET NULL,
    nombre      text NOT NULL,
    telefono    text,
    color       text NOT NULL DEFAULT '#a855f7',
    activo      boolean NOT NULL DEFAULT true,
    creado_en   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE categorias_servicio (
    id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre  text NOT NULL UNIQUE,
    orden   integer NOT NULL DEFAULT 0
);

CREATE TABLE servicios (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    categoria_id     uuid REFERENCES categorias_servicio (id) ON DELETE SET NULL,
    nombre           text NOT NULL,
    descripcion      text,
    duracion_min     integer NOT NULL CHECK (duracion_min > 0),
    buffer_min       integer NOT NULL DEFAULT 0 CHECK (buffer_min >= 0),
    precio_centavos  integer NOT NULL CHECK (precio_centavos >= 0),
    activo           boolean NOT NULL DEFAULT true,
    creado_en        timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE clientes (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre            text NOT NULL,
    telefono          text,
    email             text,
    notas             text,
    acepta_marketing  boolean NOT NULL DEFAULT false,
    creado_en         timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX clientes_nombre_idx ON clientes (lower(nombre));
CREATE INDEX clientes_telefono_idx ON clientes (telefono);

CREATE TABLE horarios_estilista (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    estilista_id  uuid NOT NULL REFERENCES estilistas (id) ON DELETE CASCADE,
    dia_semana    smallint NOT NULL CHECK (dia_semana BETWEEN 0 AND 6),
    hora_inicio   time NOT NULL,
    hora_fin      time NOT NULL,
    UNIQUE (estilista_id, dia_semana, hora_inicio),
    CHECK (hora_fin > hora_inicio)
);

CREATE TABLE bloqueos (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    estilista_id  uuid NOT NULL REFERENCES estilistas (id) ON DELETE CASCADE,
    inicio        timestamptz NOT NULL,
    fin           timestamptz NOT NULL,
    motivo        text,
    CHECK (fin > inicio)
);

CREATE TABLE festivos (
    id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    fecha   date NOT NULL UNIQUE,
    nombre  text NOT NULL
);

CREATE TABLE citas (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    cliente_id    uuid NOT NULL REFERENCES clientes (id) ON DELETE RESTRICT,
    estilista_id  uuid NOT NULL REFERENCES estilistas (id) ON DELETE RESTRICT,
    inicio        timestamptz NOT NULL,
    fin           timestamptz NOT NULL,
    estado        text NOT NULL DEFAULT 'pendiente'
                  CHECK (estado IN ('pendiente', 'confirmada', 'atendida', 'no_show', 'cancelada')),
    notas         text,
    canal         text NOT NULL DEFAULT 'staff' CHECK (canal IN ('staff', 'online')),
    creado_por    uuid REFERENCES usuarios (id) ON DELETE SET NULL,
    creado_en     timestamptz NOT NULL DEFAULT now(),
    actualizado_en timestamptz NOT NULL DEFAULT now(),
    CHECK (fin > inicio),
    -- Una estilista no puede tener dos citas activas solapadas
    CONSTRAINT citas_sin_solape EXCLUDE USING gist (
        estilista_id WITH =,
        tstzrange(inicio, fin) WITH &&
    ) WHERE (estado <> 'cancelada')
);
CREATE INDEX citas_inicio_idx ON citas (inicio);
CREATE INDEX citas_estilista_inicio_idx ON citas (estilista_id, inicio);

CREATE TABLE cita_servicios (
    id                     uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    cita_id                uuid NOT NULL REFERENCES citas (id) ON DELETE CASCADE,
    servicio_id            uuid NOT NULL REFERENCES servicios (id) ON DELETE RESTRICT,
    descripcion            text NOT NULL,
    duracion_min           integer NOT NULL,
    precio_centavos        integer NOT NULL
);
CREATE INDEX cita_servicios_cita_idx ON cita_servicios (cita_id);

CREATE TABLE productos (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    nombre           text NOT NULL,
    sku              text UNIQUE,
    precio_centavos  integer NOT NULL DEFAULT 0 CHECK (precio_centavos >= 0),
    costo_centavos   integer NOT NULL DEFAULT 0 CHECK (costo_centavos >= 0),
    stock            integer NOT NULL DEFAULT 0,
    stock_minimo     integer NOT NULL DEFAULT 0,
    activo           boolean NOT NULL DEFAULT true,
    creado_en        timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ventas (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    cita_id             uuid REFERENCES citas (id) ON DELETE SET NULL,
    cliente_id          uuid REFERENCES clientes (id) ON DELETE SET NULL,
    subtotal_centavos   integer NOT NULL DEFAULT 0,
    descuento_centavos  integer NOT NULL DEFAULT 0,
    propina_centavos    integer NOT NULL DEFAULT 0,
    total_centavos      integer NOT NULL DEFAULT 0,
    estado              text NOT NULL DEFAULT 'pagada'
                        CHECK (estado IN ('abierta', 'pagada', 'cancelada')),
    abierta_por        uuid REFERENCES usuarios (id) ON DELETE SET NULL,
    creada_en           timestamptz NOT NULL DEFAULT now(),
    pagada_en           timestamptz
);
CREATE INDEX ventas_creada_en_idx ON ventas (creada_en);

CREATE TABLE venta_lineas (
    id                        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    venta_id                  uuid NOT NULL REFERENCES ventas (id) ON DELETE CASCADE,
    tipo                      text NOT NULL CHECK (tipo IN ('servicio', 'producto')),
    servicio_id               uuid REFERENCES servicios (id) ON DELETE SET NULL,
    producto_id               uuid REFERENCES productos (id) ON DELETE SET NULL,
    descripcion               text NOT NULL,
    cantidad                  integer NOT NULL DEFAULT 1 CHECK (cantidad > 0),
    precio_unitario_centavos  integer NOT NULL CHECK (precio_unitario_centavos >= 0),
    total_centavos            integer NOT NULL
);
CREATE INDEX venta_lineas_venta_idx ON venta_lineas (venta_id);

CREATE TABLE pagos (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    venta_id        uuid NOT NULL REFERENCES ventas (id) ON DELETE CASCADE,
    metodo          text NOT NULL CHECK (metodo IN ('efectivo', 'tarjeta', 'transferencia', 'otro')),
    monto_centavos  integer NOT NULL CHECK (monto_centavos > 0),
    referencia      text,
    creado_en       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE movimientos_inventario (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    producto_id uuid NOT NULL REFERENCES productos (id) ON DELETE CASCADE,
    delta       integer NOT NULL,
    motivo      text NOT NULL,
    referencia  text,
    creado_en   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX movimientos_producto_idx ON movimientos_inventario (producto_id, creado_en DESC);

CREATE TABLE consumo_servicio (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    servicio_id  uuid NOT NULL REFERENCES servicios (id) ON DELETE CASCADE,
    producto_id  uuid NOT NULL REFERENCES productos (id) ON DELETE CASCADE,
    cantidad     numeric(10, 2) NOT NULL DEFAULT 1 CHECK (cantidad > 0),
    UNIQUE (servicio_id, producto_id)
);

CREATE TABLE cierres_caja (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    fecha           date NOT NULL UNIQUE,
    total_centavos  integer NOT NULL,
    efectivo_centavos    integer NOT NULL DEFAULT 0,
    tarjeta_centavos     integer NOT NULL DEFAULT 0,
    transferencia_centavos integer NOT NULL DEFAULT 0,
    propinas_centavos   integer NOT NULL DEFAULT 0,
    notas           text,
    cerrado_por     uuid REFERENCES usuarios (id) ON DELETE SET NULL,
    creado_en       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE auditoria (
    id          bigserial PRIMARY KEY,
    usuario_id  uuid REFERENCES usuarios (id) ON DELETE SET NULL,
    accion      text NOT NULL,
    entidad     text NOT NULL,
    entidad_id  text,
    datos       jsonb,
    creado_en   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX auditoria_creado_en_idx ON auditoria (creado_en DESC);
