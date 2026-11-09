-- Penandaan rehabilitasi per aset (Tahap C). Tabel terpisah 1:1 dengan features, supaya jalur tulis
-- features yang sudah ada (impor, aset) tidak perlu berubah. Aturan kondisi ada di pipa-core (rehab.rs).
CREATE TABLE aset_rehab (
    feature_id       BIGINT PRIMARY KEY REFERENCES features (id),
    status_rehab     TEXT NOT NULL
                     CHECK (status_rehab IN ('tidak_ada', 'perlu', 'prioritas', 'sedang_dikerjakan')),
    prioritas        SMALLINT CHECK (prioritas IS NULL OR prioritas BETWEEN 1 AND 10),
    catatan          TEXT CHECK (catatan IS NULL OR char_length(catatan) <= 2000),
    diubah_oleh      TEXT NOT NULL,
    diubah_pada      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status_rehab <> 'tidak_ada' OR prioritas IS NULL)
);

CREATE INDEX aset_rehab_status_idx ON aset_rehab (status_rehab) WHERE status_rehab <> 'tidak_ada';
