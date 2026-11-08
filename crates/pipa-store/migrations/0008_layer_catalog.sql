-- Katalog layer tematik untuk panel Layer di peta. Hanya URL, gaya, dan peran minimum; bukan data milik instansi lain.
-- Dibaca lewat GET /api/v1/catalog, yang menyaring baris menurut peran pemanggil (pengunjung hanya melihat 'anon').

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
    LANGUAGE plpgsql AS $$ BEGIN NEW.updated_at = now(); RETURN NEW; END $$;

CREATE TABLE layer_catalog (
    id           BIGSERIAL PRIMARY KEY,
    slug         TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]+$'),
    nama         TEXT NOT NULL,
    kategori     TEXT NOT NULL,
    sumber       TEXT NOT NULL,                  -- instansi atau penyedia, ditampilkan di panel
    kind         TEXT NOT NULL
                 CHECK (kind IN ('internal', 'feature', 'arcgis', 'vector', 'wms', 'pmtiles')),
    url          TEXT,
    params       JSONB NOT NULL DEFAULT '{}',    -- untuk 'internal': {"internalKey": "..."}
    legend       JSONB,
    style        JSONB,
    attribution  TEXT,
    min_zoom     SMALLINT CHECK (min_zoom IS NULL OR min_zoom BETWEEN 0 AND 22),
    -- Peran minimum. 'anon' = publik. Layer yang memuat data pelanggan (sr) wajib 'editor'.
    min_role     TEXT NOT NULL DEFAULT 'anon' CHECK (min_role IN ('anon', 'viewer', 'editor', 'admin')),
    chip         BOOLEAN NOT NULL DEFAULT FALSE, -- tampil di baris chip atas peta
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
CREATE TRIGGER layer_catalog_updated BEFORE UPDATE ON layer_catalog
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Satu layer awal: poligon cakupan SPM per desa yang sudah ada di peta.
INSERT INTO layer_catalog (slug, nama, kategori, sumber, kind, params, min_role)
VALUES ('cakupan-spm', 'Cakupan SPM per desa', 'SPM air minum', 'Arumanis (sinkron)', 'internal',
        '{"internalKey": "cakupan-spm"}', 'anon')
ON CONFLICT (slug) DO NOTHING;
