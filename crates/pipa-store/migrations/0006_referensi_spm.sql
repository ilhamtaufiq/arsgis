-- Referensi SPM air minum. Data desa, unit SPAM, dan Pokmas berasal dari Arumanis (sinkron, baca saja);
-- batas desa dari GeoJSON BPS. Tabel ref_* hanya ditulis oleh proses sinkron dan pemuat batas desa.

-- Normalisasi nama wilayah untuk pencocokan: huruf kecil, hanya huruf dan angka.
CREATE OR REPLACE FUNCTION norm_wilayah(t TEXT) RETURNS TEXT
    LANGUAGE SQL IMMUTABLE PARALLEL SAFE
    AS $$ SELECT regexp_replace(lower(coalesce(t, '')), '[^a-z0-9]', '', 'g') $$;

CREATE TABLE ref_batas_desa (
    kode_bps   TEXT PRIMARY KEY,
    kecamatan  TEXT NOT NULL,
    desa       TEXT NOT NULL,
    geom       geometry(MultiPolygon, 4326) NOT NULL,
    sumber     TEXT,
    dimuat_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ref_batas_desa_geom_gist ON ref_batas_desa USING GIST (geom);
CREATE INDEX ref_batas_desa_nama_idx ON ref_batas_desa (norm_wilayah(kecamatan), norm_wilayah(desa));

-- id = id desa di Arumanis.
CREATE TABLE ref_desa (
    id          BIGINT PRIMARY KEY,
    nama        TEXT NOT NULL,
    kecamatan   TEXT NOT NULL,
    kode_bps    TEXT REFERENCES ref_batas_desa (kode_bps) ON DELETE SET NULL,
    target_kk   INTEGER NOT NULL DEFAULT 0,
    jp_kk       INTEGER NOT NULL DEFAULT 0,
    bjp_master  INTEGER NOT NULL DEFAULT 0,
    bjp_unit    INTEGER NOT NULL DEFAULT 0,
    sr          INTEGER NOT NULL DEFAULT 0,
    jiwa        INTEGER NOT NULL DEFAULT 0,
    unit_count  INTEGER NOT NULL DEFAULT 0,
    aktif       BOOLEAN NOT NULL DEFAULT true,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ref_desa_kecamatan_idx ON ref_desa (kecamatan);

-- Koreksi manual pencocokan desa ke poligon. kode_bps NULL berarti desa memang belum punya poligon.
CREATE TABLE ref_desa_kode_override (
    desa_id   BIGINT PRIMARY KEY,
    kode_bps  TEXT,
    catatan   TEXT
);

-- id = id unit SPAM di Arumanis. Capaian = jumlah seluruh rekam tahunan unit.
CREATE TABLE ref_unit_spam (
    id              BIGINT PRIMARY KEY,
    desa_id         BIGINT NOT NULL REFERENCES ref_desa (id),
    nama            TEXT,
    sistem_layanan  TEXT,
    sumber_dana     TEXT,
    is_simspam      BOOLEAN NOT NULL DEFAULT false,
    sr              INTEGER NOT NULL DEFAULT 0,
    kk              INTEGER NOT NULL DEFAULT 0,
    jiwa            INTEGER NOT NULL DEFAULT 0,
    bjp_kk          INTEGER NOT NULL DEFAULT 0,
    aktif           BOOLEAN NOT NULL DEFAULT true,
    synced_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ref_unit_spam_desa_idx ON ref_unit_spam (desa_id);

-- Satu Pokmas per desa, diturunkan dari pengelola unit-unit desa itu.
-- Nama pengurus adalah data pribadi: hanya untuk editor dan admin.
CREATE TABLE ref_pokmas (
    desa_id          BIGINT PRIMARY KEY REFERENCES ref_desa (id),
    nama             TEXT,
    perdes           TEXT,
    kepala           TEXT,
    bendahara        TEXT,
    sekretaris       TEXT,
    konflik          BOOLEAN NOT NULL DEFAULT false,
    sumber_unit_ids  BIGINT[] NOT NULL DEFAULT '{}',
    synced_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sync_runs (
    id                BIGSERIAL PRIMARY KEY,
    dipicu_oleh       TEXT NOT NULL,
    mulai_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    selesai_at        TIMESTAMPTZ,
    status            TEXT NOT NULL DEFAULT 'berjalan'
                      CHECK (status IN ('berjalan', 'berhasil', 'sebagian', 'gagal')),
    jumlah_desa       INTEGER,
    jumlah_unit       INTEGER,
    jumlah_pokmas     INTEGER,
    desa_tanpa_batas  INTEGER,
    pesan             TEXT
);
