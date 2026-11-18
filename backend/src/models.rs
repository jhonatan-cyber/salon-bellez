use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, FromRow)]
pub struct Usuario {
    pub id: Uuid,
    pub email: String,
    pub nombre: String,
    pub rol: String,
    pub activo: bool,
    pub creado_en: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Categoria {
    pub id: Uuid,
    pub nombre: String,
    pub orden: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Servicio {
    pub id: Uuid,
    pub categoria_id: Option<Uuid>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub duracion_min: i32,
    pub buffer_min: i32,
    pub precio_centavos: i32,
    pub activo: bool,
}

#[derive(Debug, Deserialize)]
pub struct ServicioInput {
    pub categoria_id: Option<Uuid>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub duracion_min: i32,
    #[serde(default)]
    pub buffer_min: i32,
    pub precio_centavos: i32,
    #[serde(default = "default_true")]
    pub activo: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Cliente {
    pub id: Uuid,
    pub nombre: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub notas: Option<String>,
    pub acepta_marketing: bool,
    pub creado_en: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ClienteInput {
    pub nombre: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub notas: Option<String>,
    #[serde(default)]
    pub acepta_marketing: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Estilista {
    pub id: Uuid,
    pub usuario_id: Option<Uuid>,
    pub nombre: String,
    pub telefono: Option<String>,
    pub color: String,
    pub activo: bool,
}

#[derive(Debug, Deserialize)]
pub struct EstilistaInput {
    pub usuario_id: Option<Uuid>,
    pub nombre: String,
    pub telefono: Option<String>,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_true")]
    pub activo: bool,
}

fn default_color() -> String {
    "#a855f7".to_string()
}

#[derive(Debug, Serialize, FromRow)]
pub struct Horario {
    pub id: Uuid,
    pub estilista_id: Uuid,
    pub dia_semana: i16,
    pub hora_inicio: NaiveTime,
    pub hora_fin: NaiveTime,
}

#[derive(Debug, Deserialize)]
pub struct HorarioInput {
    pub dia_semana: i16,
    pub hora_inicio: NaiveTime,
    pub hora_fin: NaiveTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Bloqueo {
    pub id: Uuid,
    pub estilista_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub motivo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BloqueoInput {
    pub inicio: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub motivo: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Festivo {
    pub id: Uuid,
    pub fecha: NaiveDate,
    pub nombre: String,
}

#[derive(Debug, Deserialize)]
pub struct FestivoInput {
    pub fecha: NaiveDate,
    pub nombre: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CitaRow {
    pub id: Uuid,
    pub cliente_id: Uuid,
    pub cliente_nombre: String,
    pub cliente_telefono: Option<String>,
    pub estilista_id: Uuid,
    pub estilista_nombre: String,
    pub inicio: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub estado: String,
    pub notas: Option<String>,
    pub canal: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CitaServicioRow {
    pub cita_id: Uuid,
    pub servicio_id: Uuid,
    pub descripcion: String,
    pub duracion_min: i32,
    pub precio_centavos: i32,
}

#[derive(Debug, Serialize)]
pub struct CitaDetalle {
    pub cita: CitaRow,
    pub servicios: Vec<CitaServicioRow>,
}

#[derive(Debug, Deserialize)]
pub struct CitaInput {
    pub cliente_id: Uuid,
    pub estilista_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub servicio_ids: Vec<Uuid>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EstadoInput {
    pub estado: String,
}

#[derive(Debug, Deserialize)]
pub struct CitaFiltro {
    pub desde: Option<DateTime<Utc>>,
    pub hasta: Option<DateTime<Utc>>,
    pub estilista_id: Option<Uuid>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Slot {
    pub inicio: DateTime<Utc>,
    pub fin: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Producto {
    pub id: Uuid,
    pub nombre: String,
    pub sku: Option<String>,
    pub precio_centavos: i32,
    pub costo_centavos: i32,
    pub stock: i32,
    pub stock_minimo: i32,
    pub activo: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProductoInput {
    pub nombre: String,
    pub sku: Option<String>,
    #[serde(default)]
    pub precio_centavos: i32,
    #[serde(default)]
    pub costo_centavos: i32,
    #[serde(default)]
    pub stock: i32,
    #[serde(default)]
    pub stock_minimo: i32,
    #[serde(default = "default_true")]
    pub activo: bool,
}

#[derive(Debug, Deserialize)]
pub struct AjusteStock {
    pub delta: i32,
    pub motivo: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Movimiento {
    pub id: Uuid,
    pub producto_id: Uuid,
    pub delta: i32,
    pub motivo: String,
    pub referencia: Option<String>,
    pub creado_en: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Venta {
    pub id: Uuid,
    pub cita_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
    pub subtotal_centavos: i32,
    pub descuento_centavos: i32,
    pub propina_centavos: i32,
    pub total_centavos: i32,
    pub estado: String,
    pub creada_en: DateTime<Utc>,
    pub pagada_en: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct VentaLinea {
    pub id: Uuid,
    pub tipo: String,
    pub descripcion: String,
    pub cantidad: i32,
    pub precio_unitario_centavos: i32,
    pub total_centavos: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Pago {
    pub id: Uuid,
    pub metodo: String,
    pub monto_centavos: i32,
    pub referencia: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VentaDetalle {
    pub venta: Venta,
    pub lineas: Vec<VentaLinea>,
    pub pagos: Vec<Pago>,
}

#[derive(Debug, Deserialize)]
pub struct LineaInput {
    pub tipo: String,
    #[serde(default)]
    pub servicio_id: Option<Uuid>,
    #[serde(default)]
    pub producto_id: Option<Uuid>,
    pub descripcion: String,
    #[serde(default = "uno")]
    pub cantidad: i32,
    pub precio_unitario_centavos: i32,
}

fn uno() -> i32 {
    1
}

#[derive(Debug, Deserialize)]
pub struct PagoInput {
    pub metodo: String,
    pub monto_centavos: i32,
    pub referencia: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VentaInput {
    pub cita_id: Option<Uuid>,
    #[serde(default)]
    pub lineas: Vec<LineaInput>,
    #[serde(default)]
    pub descuento_centavos: i32,
    #[serde(default)]
    pub propina_centavos: i32,
    #[serde(default)]
    pub pagos: Vec<PagoInput>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CierreCaja {
    pub id: Uuid,
    pub fecha: NaiveDate,
    pub total_centavos: i32,
    pub efectivo_centavos: i32,
    pub tarjeta_centavos: i32,
    pub transferencia_centavos: i32,
    pub propinas_centavos: i32,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CierreInput {
    pub fecha: NaiveDate,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReservaInput {
    pub servicio_id: Uuid,
    pub estilista_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub nombre: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub acepta_marketing: bool,
    /// Campo trampa: debe llegar vacío.
    #[serde(default)]
    pub website: String,
}
