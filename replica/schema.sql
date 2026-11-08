-- USULAN migrasi 0008 untuk crates/pipa-store/migrations/ (belum dijalankan).
-- Mengikuti konvensi 0001-0007: BIGSERIAL, TIMESTAMPTZ, CHECK, komentar Indonesia.
-- Akses: pemeriksaan di data layer (handler axum), sama seperti peran sr yang sudah ada. Bukan RLS.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
    LANGUAGE plpgsql AS $$ BEGIN NEW.updated_at = now(); RETURN NEW; END $$;

-- ---------------------------------------------------------------------------
-- Katalog layer tematik (S06, S12, S05). Hanya URL dan gaya, bukan data milik instansi lain.
-- ---------------------------------------------------------------------------
CREATE TABLE layer_catalog (
    id           BIGSERIAL PRIMARY KEY,
    slug         TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]+$'),
    nama         TEXT NOT NULL,
    kategori     TEXT NOT NULL,
    sumber       TEXT NOT NULL,                  -- nama instansi / penyedia, ditampilkan di panel
    kind         TEXT NOT NULL
                 CHECK (kind IN ('internal', 'feature', 'arcgis', 'vector', 'wms', 'pmtiles')),
    url          TEXT,
    params       JSONB NOT NULL DEFAULT '{}',    -- layers, format, query tanpa token, dsb.
    legend       JSONB,                          -- [{label, warna}] atau url gambar
    style        JSONB,
    attribution  TEXT,
    min_zoom     SMALLINT CHECK (min_zoom IS NULL OR min_zoom BETWEEN 0 AND 22),
    -- Peran minimum. 'anon' = publik. Layer yang memuat data pelanggan (sr) wajib 'editor'.
    min_role     TEXT NOT NULL DEFAULT 'anon' CHECK (min_role IN ('anon', 'viewer', 'editor', 'admin')),
    chip         BOOLEAN NOT NULL DEFAULT FALSE, -- tampil di baris chip atas peta (S12)
    chip_urut    SMALLINT,
    baru_sampai  DATE,                           -- tag NEW sampai tanggal ini
    aktif        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT layer_butuh_url CHECK (kind = 'internal' OR url IS NOT NULL),
    CONSTRAINT chip_butuh_urut CHECK (NOT chip OR chip_urut IS NOT NULL)
);
CREATE INDEX layer_catalog_kategori_idx ON layer_catalog (kategori) WHERE aktif;
CREATE INDEX layer_catalog_kind_idx ON layer_catalog (kind) WHERE aktif;
CREATE INDEX layer_catalog_chip_idx ON layer_catalog (chip_urut) WHERE chip AND aktif;
CREATE INDEX layer_catalog_nama_trgm ON layer_catalog USING GIN (nama gin_trgm_ops);
CREATE TRIGGER layer_catalog_updated BEFORE UPDATE ON layer_catalog
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- Layer milik pengguna: hasil gambar (S09) dan impor file (S07). Satu pemilik.
-- ---------------------------------------------------------------------------
CREATE TABLE user_layers (
    id            BIGSERIAL PRIMARY KEY,
    owner_id      BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    nama          TEXT NOT NULL CHECK (length(nama) BETWEEN 1 AND 120),
    asal          TEXT NOT NULL
                  CHECK (asal IN ('draw', 'shp', 'geojson', 'gpx', 'kml', 'kmz', 'xy', 'timeline')),
    jumlah_fitur  INTEGER NOT NULL DEFAULT 0 CHECK (jumlah_fitur >= 0),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, nama)
);
CREATE INDEX user_layers_owner_idx ON user_layers (owner_id);
CREATE TRIGGER user_layers_updated BEFORE UPDATE ON user_layers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE user_features (
    id          BIGSERIAL PRIMARY KEY,
    layer_id    BIGINT NOT NULL REFERENCES user_layers (id) ON DELETE CASCADE,
    geom        geometry(Geometry, 4326) NOT NULL,
    props       JSONB NOT NULL DEFAULT '{}',   -- atribut asli file, plus panjang/luas hasil ukur
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX user_features_layer_idx ON user_features (layer_id);
CREATE INDEX user_features_geom_gist ON user_features USING GIST (geom);

-- ---------------------------------------------------------------------------
-- Wilayah administratif untuk pencarian (S04) dan unduh batas. Terpisah dari ref_batas_desa
-- (yang khusus Cianjur dan dipakai impor). Diisi pemuat CLI `load-wilayah`, bukan oleh pengguna.
-- level: 1 provinsi, 2 kab/kota, 3 kecamatan, 4 desa/kelurahan.
-- ---------------------------------------------------------------------------
CREATE TABLE ref_wilayah (
    kode        TEXT PRIMARY KEY,               -- kode Kemendagri bertitik, mis. 35.73
    level       SMALLINT NOT NULL CHECK (level BETWEEN 1 AND 4),
    nama        TEXT NOT NULL,
    nama_norm   TEXT GENERATED ALWAYS AS (norm_wilayah(nama)) STORED,
    parent_kode TEXT REFERENCES ref_wilayah (kode) ON DELETE RESTRICT,
    geom        geometry(MultiPolygon, 4326),   -- disederhanakan; NULL bila belum ada batas
    kode_bps    TEXT REFERENCES ref_batas_desa (kode_bps) ON DELETE SET NULL,  -- hanya level 4 Cianjur
    sumber      TEXT,
    dimuat_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT induk_sesuai_level CHECK ((level = 1) = (parent_kode IS NULL))
);
CREATE INDEX ref_wilayah_parent_idx ON ref_wilayah (parent_kode);
CREATE INDEX ref_wilayah_level_idx ON ref_wilayah (level);
CREATE INDEX ref_wilayah_nama_trgm ON ref_wilayah USING GIN (nama_norm gin_trgm_ops);
CREATE INDEX ref_wilayah_geom_gist ON ref_wilayah USING GIST (geom);

-- Batas keras:
--  * Ukuran impor ditegakkan di aplikasi (maks 50.000 fitur per layer, sama dengan batas bbox yang ada).
--  * Geometri divalidasi dengan ST_IsValid / ST_MakeValid di aplikasi sebelum insert, bukan CHECK,
--    supaya satu fitur buruk tidak menggagalkan seluruh impor.
--  * Tidak ada tabel persetujuan penafian: pilihan disimpan di peramban (localStorage), seperti aslinya.
