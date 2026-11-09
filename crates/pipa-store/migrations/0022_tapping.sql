-- Jenis tapping: titik sambungan pengambilan air dari pipa, dipisah dari SR (titik pelanggan).
-- Data tapping lama dari impor KML sempat tersimpan sebagai 'sr'. Migrasi ini memindahkannya ke 'tapping'
-- kecuali baris yang sudah terhubung ke pelanggan atau punya dusun (dusun hanya boleh untuk SR).
-- Kode aset tidak diubah karena pemicu features_kode_aset_tetap melarangnya. Kode lama tetap memakai 'sr'.

-- 1. Izinkan jenis baru di features dan import_staging. Nama CHECK sama dengan migrasi 0010.
ALTER TABLE features DROP CONSTRAINT features_feature_type_check;
ALTER TABLE features ADD CONSTRAINT features_feature_type_check
    CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering',
                            'bak_penahan_tekanan', 'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk',
                            'tapping'));

ALTER TABLE import_staging DROP CONSTRAINT import_staging_feature_type_check;
ALTER TABLE import_staging ADD CONSTRAINT import_staging_feature_type_check
    CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering',
                            'bak_penahan_tekanan', 'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk',
                            'tapping'));

-- 2. Pindahkan data tapping lama. Baris yang sudah punya pelanggan atau dusun dibiarkan dan dilaporkan.
DO $$
DECLARE
    n_diubah      INTEGER;
    n_pelanggan   INTEGER;
    n_dusun       INTEGER;
BEGIN
    SELECT count(*) INTO n_pelanggan
    FROM features f
    WHERE f.feature_type = 'sr' AND btrim(f.nama) ILIKE 'tapping%'
      AND EXISTS (SELECT 1 FROM pelanggan p WHERE p.sr_feature_id = f.id);

    SELECT count(*) INTO n_dusun
    FROM features f
    WHERE f.feature_type = 'sr' AND btrim(f.nama) ILIKE 'tapping%'
      AND f.dusun_id IS NOT NULL
      AND NOT EXISTS (SELECT 1 FROM pelanggan p WHERE p.sr_feature_id = f.id);

    UPDATE features f
    SET feature_type = 'tapping'
    WHERE f.feature_type = 'sr' AND btrim(f.nama) ILIKE 'tapping%'
      AND f.dusun_id IS NULL
      AND NOT EXISTS (SELECT 1 FROM pelanggan p WHERE p.sr_feature_id = f.id);
    GET DIAGNOSTICS n_diubah = ROW_COUNT;

    RAISE NOTICE 'migrasi 0022: % baris tapping dipindah dari sr; % dilewati karena sudah terhubung pelanggan; % dilewati karena punya dusun',
        n_diubah, n_pelanggan, n_dusun;
END $$;
