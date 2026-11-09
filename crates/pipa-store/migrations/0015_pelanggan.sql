-- Pelanggan: setiap pelanggan terhubung ke satu titik SR (features.feature_type = 'sr').
-- Pelanggan tidak dihapus. Pemutusan dicatat dengan status 'putus', sehingga riwayat tetap utuh.

CREATE TABLE pelanggan (
    id             BIGSERIAL PRIMARY KEY,
    nama           TEXT NOT NULL CHECK (length(btrim(nama)) >= 2),
    alamat         TEXT,
    nomor_meter    TEXT,
    golongan_tarif TEXT NOT NULL
                   CHECK (golongan_tarif IN ('sosial', 'rumah_tangga', 'niaga', 'industri', 'khusus')),
    status         TEXT NOT NULL DEFAULT 'aktif' CHECK (status IN ('aktif', 'putus')),
    sr_feature_id  BIGINT NOT NULL REFERENCES features (id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Nomor meter unik bila diisi. Pelanggan tanpa meter (belum terpasang) tidak dibatasi.
CREATE UNIQUE INDEX pelanggan_nomor_meter_uniq ON pelanggan (nomor_meter) WHERE nomor_meter IS NOT NULL;
CREATE INDEX pelanggan_sr_idx ON pelanggan (sr_feature_id);
CREATE INDEX pelanggan_nama_idx ON pelanggan (lower(nama));

-- Penjaga terakhir: sr_feature_id harus titik SR yang masih aktif (tidak dihapus lunak).
CREATE FUNCTION pelanggan_cek_sr() RETURNS trigger AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM features f
        WHERE f.id = NEW.sr_feature_id AND f.feature_type = 'sr' AND f.deleted_at IS NULL
    ) THEN
        RAISE EXCEPTION 'pelanggan harus terhubung ke titik SR yang ada' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER pelanggan_cek_sr
    BEFORE INSERT OR UPDATE OF sr_feature_id ON pelanggan
    FOR EACH ROW EXECUTE FUNCTION pelanggan_cek_sr();

-- Riwayat pelanggan: setiap tambah dan ubah menulis satu baris dalam transaksi yang sama.
-- Snapshot berbentuk objek data pelanggan (lihat pipa-store/src/pelanggan.rs).
CREATE TABLE pelanggan_riwayat (
    id           BIGSERIAL PRIMARY KEY,
    pelanggan_id BIGINT NOT NULL REFERENCES pelanggan (id),
    aksi         TEXT NOT NULL CHECK (aksi IN ('tambah', 'ubah')),
    diubah_oleh  TEXT NOT NULL,
    diubah_pada  TIMESTAMPTZ NOT NULL DEFAULT now(),
    sebelum      JSONB,
    sesudah      JSONB,
    CHECK (
        (aksi = 'tambah' AND sebelum IS NULL AND sesudah IS NOT NULL)
        OR (aksi = 'ubah' AND sebelum IS NOT NULL AND sesudah IS NOT NULL)
    )
);
CREATE INDEX pelanggan_riwayat_idx ON pelanggan_riwayat (pelanggan_id, diubah_pada DESC, id DESC);

-- Riwayat koreksi Pokmas per desa. Melengkapi pokmas_override (yang hanya menyimpan keadaan terakhir).
-- 'hapus' berarti koreksi dihapus dan desa kembali ke data Arumanis; sebelum berisi koreksi yang dihapus.
CREATE TABLE pokmas_riwayat (
    id          BIGSERIAL PRIMARY KEY,
    desa_id     BIGINT NOT NULL REFERENCES ref_desa (id),
    aksi        TEXT NOT NULL CHECK (aksi IN ('tambah', 'ubah', 'hapus')),
    diubah_oleh TEXT NOT NULL,
    diubah_pada TIMESTAMPTZ NOT NULL DEFAULT now(),
    sebelum     JSONB,
    sesudah     JSONB,
    CHECK (
        (aksi = 'tambah' AND sebelum IS NULL AND sesudah IS NOT NULL)
        OR (aksi = 'ubah' AND sebelum IS NOT NULL AND sesudah IS NOT NULL)
        OR (aksi = 'hapus' AND sebelum IS NOT NULL AND sesudah IS NULL)
    )
);
CREATE INDEX pokmas_riwayat_idx ON pokmas_riwayat (desa_id, diubah_pada DESC, id DESC);
