-- Pelaporan gangguan layanan (Tahap C). Laporan bisa menunjuk aset, titik koordinat, atau keduanya.
-- Status hanya maju: dilaporkan, ditangani, selesai. Aturan transisi ada di pipa-core (gangguan.rs).

CREATE TABLE gangguan (
    id               BIGSERIAL PRIMARY KEY,
    network_id       BIGINT NOT NULL REFERENCES networks (id),
    feature_id       BIGINT REFERENCES features (id) ON DELETE SET NULL,
    jenis            TEXT NOT NULL CHECK (jenis IN ('pipa_bocor', 'air_mati', 'kualitas_keruh', 'lainnya')),
    geom             geometry(Point, 4326),
    deskripsi        TEXT NOT NULL CHECK (char_length(btrim(deskripsi)) BETWEEN 1 AND 2000),
    pelapor          TEXT NOT NULL CHECK (char_length(btrim(pelapor)) BETWEEN 1 AND 100),
    status           TEXT NOT NULL DEFAULT 'dilaporkan'
                     CHECK (status IN ('dilaporkan', 'ditangani', 'selesai')),
    dilaporkan_pada  TIMESTAMPTZ NOT NULL DEFAULT now(),
    diperbarui_pada  TIMESTAMPTZ NOT NULL DEFAULT now(),
    diperbarui_oleh  TEXT NOT NULL,
    CHECK (feature_id IS NOT NULL OR geom IS NOT NULL)
);

CREATE INDEX gangguan_network_status_idx ON gangguan (network_id, status, dilaporkan_pada DESC);
CREATE INDEX gangguan_feature_idx ON gangguan (feature_id);
CREATE INDEX gangguan_geom_gist ON gangguan USING GIST (geom);

-- Foto disimpan di disk. Nama berkas dibuat server (acak), bukan nama asli dari klien.
CREATE TABLE gangguan_foto (
    id              BIGSERIAL PRIMARY KEY,
    gangguan_id     BIGINT NOT NULL REFERENCES gangguan (id) ON DELETE CASCADE,
    berkas          TEXT NOT NULL UNIQUE CHECK (berkas ~ '^[0-9a-f]{32}\.(jpg|png)$'),
    mime            TEXT NOT NULL CHECK (mime IN ('image/jpeg', 'image/png')),
    ukuran_byte     INTEGER NOT NULL CHECK (ukuran_byte > 0),
    diunggah_oleh   TEXT NOT NULL,
    diunggah_pada   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX gangguan_foto_gangguan_idx ON gangguan_foto (gangguan_id, id);

-- Log perubahan O&M: gangguan, perbaikan, jadwal perawatan, dan rehabilitasi.
-- Satu tabel untuk semua sumber supaya riwayat per aset bisa dibaca dari satu tempat.
CREATE TABLE om_riwayat (
    id           BIGSERIAL PRIMARY KEY,
    sumber       TEXT NOT NULL CHECK (sumber IN ('gangguan', 'perbaikan', 'jadwal_perawatan', 'rehab')),
    sumber_id    BIGINT NOT NULL,
    feature_id   BIGINT REFERENCES features (id) ON DELETE SET NULL,
    aksi         TEXT NOT NULL CHECK (aksi IN ('tambah', 'ubah', 'selesai', 'foto')),
    diubah_oleh  TEXT NOT NULL,
    diubah_pada  TIMESTAMPTZ NOT NULL DEFAULT now(),
    sebelum      JSONB,
    sesudah      JSONB
);

CREATE INDEX om_riwayat_feature_idx ON om_riwayat (feature_id, diubah_pada DESC, id DESC);
CREATE INDEX om_riwayat_sumber_idx ON om_riwayat (sumber, sumber_id, id DESC);
