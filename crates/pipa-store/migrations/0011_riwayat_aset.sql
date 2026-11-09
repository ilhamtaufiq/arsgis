-- Riwayat perubahan aset. Setiap tambah, ubah, dan hapus menulis satu baris di sini, dalam transaksi yang sama.
-- sebelum dan sesudah berisi fitur dalam bentuk GeoJSON Feature (id, geometry, properties) saat itu.
--   tambah: hanya sesudah.   ubah: sebelum dan sesudah.   hapus (soft delete): hanya sebelum.
-- Fitur tidak pernah dihapus fisik, jadi riwayat tetap bisa dibaca setelah hapus.

CREATE TABLE feature_riwayat (
    id           BIGSERIAL PRIMARY KEY,
    feature_id   BIGINT NOT NULL REFERENCES features (id) ON DELETE CASCADE,
    aksi         TEXT NOT NULL CHECK (aksi IN ('tambah', 'ubah', 'hapus')),
    diubah_oleh  TEXT NOT NULL,
    diubah_pada  TIMESTAMPTZ NOT NULL DEFAULT now(),
    sebelum      JSONB,
    sesudah      JSONB,
    CHECK (
        (aksi = 'tambah' AND sebelum IS NULL AND sesudah IS NOT NULL)
        OR (aksi = 'ubah' AND sebelum IS NOT NULL AND sesudah IS NOT NULL)
        OR (aksi = 'hapus' AND sebelum IS NOT NULL AND sesudah IS NULL)
    )
);

CREATE INDEX feature_riwayat_feature_idx ON feature_riwayat (feature_id, diubah_pada DESC, id DESC);
