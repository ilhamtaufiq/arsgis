-- Data contoh untuk pengembangan lokal. Koordinat fiktif di sekitar Ciranjang, Cianjur (lon lat, EPSG:4326).
INSERT INTO networks (nama, deskripsi)
VALUES ('Jaringan Contoh Ciranjang', 'Data uji pengembangan, bukan data lapangan');

INSERT INTO features (network_id, feature_type, geom, nama, diameter_mm, material, tahun_pasang, kondisi)
VALUES
  (1, 'pipa',    ST_GeomFromText('LINESTRING(107.1400 -6.8200, 107.1450 -6.8210, 107.1500 -6.8205)', 4326), 'Transmisi A', 150, 'DCI', 2015, 'baik'),
  (1, 'pipa',    ST_GeomFromText('LINESTRING(107.1450 -6.8210, 107.1460 -6.8250)', 4326), 'Distribusi B1', 63, 'HDPE', 2019, 'baik'),
  (1, 'pipa',    ST_GeomFromText('LINESTRING(107.1460 -6.8250, 107.1500 -6.8280)', 4326), 'Distribusi B2', 50, 'PVC', 2008, 'perlu cek'),
  (1, 'valve',   ST_GeomFromText('POINT(107.1450 -6.8210)', 4326), 'Valve V1', NULL, NULL, 2015, 'baik'),
  (1, 'reservoir', ST_GeomFromText('POINT(107.1500 -6.8205)', 4326), 'Reservoir R1', NULL, NULL, 2012, 'baik'),
  (1, 'sr',      ST_GeomFromText('POINT(107.1470 -6.8240)', 4326), 'Sambungan contoh', 15, 'PVC', 2020, 'aktif');
