-- Kelola impor: batal impor, pencegahan impor ganda, dan jenis riwayat baru (batal_impor, pulihkan).
-- Nama CHECK lama dicari dari pg_constraint, supaya tidak bergantung pada nama otomatis Postgres.

DO $$
DECLARE c record;
BEGIN
    FOR c IN SELECT conname FROM pg_constraint WHERE conrelid = 'import_batches'::regclass AND contype = 'c' LOOP
        EXECUTE format('ALTER TABLE import_batches DROP CONSTRAINT %I', c.conname);
    END LOOP;
    FOR c IN SELECT conname FROM pg_constraint WHERE conrelid = 'feature_riwayat'::regclass AND contype = 'c' LOOP
        EXECUTE format('ALTER TABLE feature_riwayat DROP CONSTRAINT %I', c.conname);
    END LOOP;
END $$;

-- Batch bisa dibatalkan. Batch dibatalkan tidak pernah kembali ke staged atau committed.
ALTER TABLE import_batches ADD CONSTRAINT import_batches_status_check
    CHECK (status IN ('staged', 'committed', 'dibatalkan'));
ALTER TABLE import_batches ADD COLUMN dibatalkan_oleh TEXT;
ALTER TABLE import_batches ADD COLUMN dibatalkan_pada TIMESTAMPTZ;

-- sha256 berkas asli (bytes KML/KMZ yang di-upload), hex. Dipakai mencegah impor ganda.
-- NULL untuk batch lama yang diunggah sebelum kolom ini ada.
ALTER TABLE import_batches ADD COLUMN file_sha256 TEXT;
CREATE INDEX import_batches_sha256_idx ON import_batches (file_sha256) WHERE status = 'committed';

-- Riwayat aset: batal_impor (soft delete massal, seperti hapus) dan pulihkan (seperti ubah: sebelum dan sesudah).
ALTER TABLE feature_riwayat ADD CONSTRAINT feature_riwayat_aksi_check
    CHECK (aksi IN ('tambah', 'ubah', 'hapus', 'batal_impor', 'pulihkan'));
ALTER TABLE feature_riwayat ADD CONSTRAINT feature_riwayat_bentuk_check CHECK (
    (aksi = 'tambah' AND sebelum IS NULL AND sesudah IS NOT NULL)
    OR (aksi = 'ubah' AND sebelum IS NOT NULL AND sesudah IS NOT NULL)
    OR (aksi = 'pulihkan' AND sebelum IS NOT NULL AND sesudah IS NOT NULL)
    OR (aksi IN ('hapus', 'batal_impor') AND sebelum IS NOT NULL AND sesudah IS NULL)
);
