-- Estilista de ejemplo para que la agenda y la reserva pública no arranquen vacías.
-- Solo se inserta si todavía no existe ninguna estilista.
INSERT INTO estilistas (nombre, telefono, color, activo)
SELECT 'Karla Méndez', '5512345678', '#a855f7', true
WHERE NOT EXISTS (SELECT 1 FROM estilistas);

-- Horario de martes a sábado (0 = domingo, como num_days_from_sunday en la agenda).
INSERT INTO horarios_estilista (estilista_id, dia_semana, hora_inicio, hora_fin)
SELECT e.id, dias.dia, '09:00'::time, '19:00'::time
FROM estilistas e
CROSS JOIN (VALUES (2), (3), (4), (5), (6)) AS dias(dia)
WHERE e.nombre = 'Karla Méndez'
  AND NOT EXISTS (SELECT 1 FROM horarios_estilista h WHERE h.estilista_id = e.id);
