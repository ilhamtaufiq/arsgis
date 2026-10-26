-- Impor per unit SPAM. Setiap unit SPAM punya satu baris di `networks` (kolom unit_spam_id dan desa_id).
-- dalam_batas: true bila titik/garis berada di batas desa unit; false bila di luar; NULL bila desa belum punya poligon.
ALTER TABLE import_staging ADD COLUMN dalam_batas BOOLEAN;
