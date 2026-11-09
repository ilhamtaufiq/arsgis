-- Riwayat perbaikan per aset (Tahap C). Opsional terhubung ke laporan gangguan.
CREATE TABLE perbaikan (
    id             BIGSERIAL PRIMARY KEY,
    feature_id     BIGINT NOT NULL REFERENCES features (id),
    gangguan_id    BIGINT REFERENCES gangguan (id) ON DELETE SET NULL,
    tanggal        DATE NOT NULL,
    tindakan       TEXT NOT NULL CHECK (char_length(btrim(tindakan)) BETWEEN 1 AND 500),
    teknisi        TEXT NOT NULL CHECK (char_length(btrim(teknisi)) BETWEEN 1 AND 100),
    biaya          NUMERIC CHECK (biaya IS NULL OR biaya >= 0),
    catatan        TEXT CHECK (catatan IS NULL OR char_length(catatan) <= 2000),
    dicatat_oleh   TEXT NOT NULL,
    dicatat_pada   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX perbaikan_feature_idx ON perbaikan (feature_id, tanggal DESC, id DESC);
CREATE INDEX perbaikan_gangguan_idx ON perbaikan (gangguan_id);

-- Jadwal perawatan berulang per aset. tanggal_berikutnya dihitung ulang setiap perawatan dicatat selesai.
CREATE TABLE jadwal_perawatan (
    id                   BIGSERIAL PRIMARY KEY,
    feature_id           BIGINT NOT NULL REFERENCES features (id),
    jenis                TEXT NOT NULL
                         CHECK (jenis IN ('pembersihan_bak', 'cek_valve', 'kalibrasi_meter', 'lainnya')),
    interval_hari        INTEGER NOT NULL CHECK (interval_hari BETWEEN 1 AND 3650),
    terakhir_dilakukan   DATE,
    tanggal_berikutnya   DATE NOT NULL,
    aktif                BOOLEAN NOT NULL DEFAULT true,
    diubah_oleh          TEXT NOT NULL,
    diubah_pada          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX jadwal_perawatan_jatuh_tempo_idx ON jadwal_perawatan (tanggal_berikutnya) WHERE aktif;
CREATE INDEX jadwal_perawatan_feature_idx ON jadwal_perawatan (feature_id);
