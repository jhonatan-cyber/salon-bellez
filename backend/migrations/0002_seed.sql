-- Datos demo (idempotente)
INSERT INTO categorias_servicio (nombre, orden) VALUES
    ('Cabello', 1),
    ('Uñas', 2),
    ('Estética', 3)
ON CONFLICT (nombre) DO NOTHING;

INSERT INTO servicios (categoria_id, nombre, descripcion, duracion_min, buffer_min, precio_centavos)
SELECT c.id, v.nombre, v.descripcion, v.duracion, v.buffer, v.precio
FROM (VALUES
    ('Cabello', 'Corte de dama', 'Corte, lavado y peinado', 45, 10, 25000),
    ('Cabello', 'Tinte completo', 'Coloración profesional', 120, 15, 85000),
    ('Cabello', 'Peinado de fiesta', 'Recogido o ondas', 60, 10, 45000),
    ('Uñas', 'Manicura semipermanente', 'Limado, cutícula y esmalte', 60, 10, 30000),
    ('Uñas', 'Pedicura completa', 'Con exfoliación y masaje', 75, 10, 35000),
    ('Estética', 'Depilación de cejas', 'Diseño con cera', 20, 5, 12000),
    ('Estética', 'Facial hidratante', 'Limpieza e hidratación', 60, 10, 50000)
) AS v(categoria, nombre, descripcion, duracion, buffer, precio)
JOIN categorias_servicio c ON c.nombre = v.categoria
WHERE NOT EXISTS (SELECT 1 FROM servicios s WHERE s.nombre = v.nombre);

INSERT INTO productos (nombre, sku, precio_centavos, costo_centavos, stock, stock_minimo)
SELECT v.nombre, v.sku, v.precio, v.costo, v.stock, v.minimo
FROM (VALUES
    ('Shampoo profesional 500ml', 'SH-500', 28000, 16000, 12, 4),
    ('Acondicionador 500ml', 'AC-500', 26000, 15000, 9, 3),
    ('Esmalte gel', 'EG-01', 15000, 7000, 20, 5),
    ('Mascarilla facial', 'MF-01', 9000, 4000, 25, 8)
) AS v(nombre, sku, precio, costo, stock, minimo)
WHERE NOT EXISTS (SELECT 1 FROM productos p WHERE p.sku = v.sku);

INSERT INTO festivos (fecha, nombre) VALUES
    ('2027-01-01', 'Año Nuevo'),
    ('2027-02-01', 'Día de la Constitución'),
    ('2027-09-16', 'Día de la Independencia')
ON CONFLICT (fecha) DO NOTHING;
