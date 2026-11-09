-- Peran pengguna diperluas. Peran operator/teknisi tetap bernilai teks 'editor' agar data lama tidak berubah.
--   kepala_desa: hanya membaca ringkasan cakupan (tanpa data pelanggan perorangan).
--   bendahara : membaca data pelanggan dan cakupan, tidak mengubah apa pun.
-- Nama constraint bawaan PostgreSQL untuk CHECK kolom `users.role` adalah `users_role_check`.
ALTER TABLE users DROP CONSTRAINT users_role_check;
ALTER TABLE users ADD CONSTRAINT users_role_check
    CHECK (role IN ('viewer', 'editor', 'admin', 'kepala_desa', 'bendahara'));

-- Dusun per desa beserta jumlah rumah dan jiwa. Data ini diisi operator; tidak ada di Arumanis,
-- sehingga tidak diisi dengan angka perkiraan. Dusun belum punya data sama sekali sampai diisi.
CREATE TABLE dusun (
    id              BIGSERIAL PRIMARY KEY,
    desa_id         BIGINT NOT NULL REFERENCES ref_desa (id),
    nama            TEXT NOT NULL CHECK (length(btrim(nama)) > 0),
    jumlah_rumah    INTEGER NOT NULL DEFAULT 0 CHECK (jumlah_rumah >= 0),
    jumlah_jiwa     INTEGER NOT NULL DEFAULT 0 CHECK (jumlah_jiwa >= 0),
    diperbarui_oleh TEXT,
    diperbarui_pada TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Nama dusun tidak peka huruf besar-kecil dan spasi di ujung, per desa.
CREATE UNIQUE INDEX dusun_desa_nama_uniq ON dusun (desa_id, lower(btrim(nama)));

-- Dusun sebuah titik SR. Hanya SR yang boleh punya dusun (dijaga CHECK di bawah dan di API).
ALTER TABLE features ADD COLUMN dusun_id BIGINT REFERENCES dusun (id);
ALTER TABLE features ADD CONSTRAINT features_dusun_hanya_sr CHECK (dusun_id IS NULL OR feature_type = 'sr');
CREATE INDEX features_dusun_idx ON features (dusun_id) WHERE dusun_id IS NOT NULL;
