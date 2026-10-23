-- Skema awal WebGIS perpipaan. Database ini terpisah dari database produksi Arumanis.

CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE networks (
    id           BIGSERIAL PRIMARY KEY,
    nama         TEXT NOT NULL,
    unit_spam_id BIGINT,
    desa_id      BIGINT,
    deskripsi    TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE features (
    id               BIGSERIAL PRIMARY KEY,
    network_id       BIGINT NOT NULL REFERENCES networks (id),
    feature_type     TEXT NOT NULL
                     CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering')),
    geom             geometry(Geometry, 4326) NOT NULL,
    nama             TEXT,
    diameter_mm      INTEGER CHECK (diameter_mm IS NULL OR diameter_mm > 0),
    material         TEXT CHECK (material IS NULL OR material IN ('PVC', 'HDPE', 'GIP', 'DCI')),
    tahun_pasang     SMALLINT,
    kondisi          TEXT,
    status           TEXT NOT NULL DEFAULT 'aktif',
    extra            JSONB,
    import_batch_id  BIGINT,
    deleted_at       TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX features_geom_gist ON features USING GIST (geom);
CREATE INDEX features_network_idx ON features (network_id);
CREATE INDEX features_type_idx ON features (feature_type);
CREATE INDEX features_import_idx ON features (import_batch_id);
