-- Nama paket pekerjaan dari Arumanis (sinkron baca saja, pola sama dengan ref_desa). Dipakai untuk memberi
-- label aset dan batch impor. tahun dan desa_id boleh kosong: pekerjaan bisa belum punya desa di Arumanis.
CREATE TABLE ref_pekerjaan (
    id          BIGINT PRIMARY KEY,
    nama_paket  TEXT NOT NULL,
    tahun       SMALLINT,
    desa_id     BIGINT REFERENCES ref_desa (id) ON DELETE SET NULL,
    aktif       BOOLEAN NOT NULL DEFAULT true,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ref_pekerjaan_nama_idx ON ref_pekerjaan (nama_paket);

-- Pekerjaan yang dipakai aset. Diisi dari batch impor saat commit. NULL untuk aset lama dan aset yang dibuat manual.
ALTER TABLE features ADD COLUMN pekerjaan_id BIGINT REFERENCES ref_pekerjaan (id) ON DELETE SET NULL;
CREATE INDEX features_pekerjaan_idx ON features (pekerjaan_id);

-- Pekerjaan yang dipilih saat upload. Diterapkan ke semua aset batch itu ketika di-commit.
ALTER TABLE import_batches ADD COLUMN pekerjaan_id BIGINT REFERENCES ref_pekerjaan (id) ON DELETE SET NULL;
