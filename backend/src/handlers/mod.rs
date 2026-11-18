pub mod auth;
pub mod catalog;
pub mod citas;
pub mod disponibilidad;
pub mod docs;
pub mod publico;
pub mod reportes;
pub mod salud;
pub mod staff;
pub mod ventas;

/// Roles con acceso al panel.
pub const STAFF: &[&str] = &["admin", "recepcionista", "estilista"];

/// Roles que pueden modificar agenda, catálogo, equipo y caja.
pub const ADMIN_RECEP: &[&str] = &["admin", "recepcionista"];
