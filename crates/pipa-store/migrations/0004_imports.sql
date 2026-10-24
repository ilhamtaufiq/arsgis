-- Impor KML/GeoJSON lewat staging. Data baru masuk ke features hanya saat batch di-commit.

CREATE TABLE import_batches (
    id           BIGSERIAL PRIMARY KEY,
    nama_file    TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'staged' CHECK (status IN ('staged', 'committed')),
    network_id   BIGINT REFERENCES networks (id),
    laporan      JSONB NOT NULL,
    dibuat_oleh  TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE import_staging (
    id           BIGSERIAL PRIMARY KEY,
    batch_id     BIGINT NOT NULL REFERENCES import_batches (id) ON DELETE CASCADE,
    feature_type TEXT NOT NULL
                 CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering')),
    nama         TEXT,
    diameter_mm  INTEGER CHECK (diameter_mm IS NULL OR diameter_mm > 0),
    geom         geometry(Geometry, 4326) NOT NULL
);

CREATE INDEX import_staging_batch_idx ON import_staging (batch_id);
