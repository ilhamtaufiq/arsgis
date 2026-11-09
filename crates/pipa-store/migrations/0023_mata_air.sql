-- Jenis mata_air: sumber air alami, dipisah dari intake (sumur bor).
-- Tidak ada pemindahan data: data mata air lama, jika ada, tetap di jenis asalnya.

-- 1. Izinkan jenis baru di features dan import_staging. Nama CHECK sama dengan migrasi 0010 dan 0022.
ALTER TABLE features DROP CONSTRAINT features_feature_type_check;
ALTER TABLE features ADD CONSTRAINT features_feature_type_check
    CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering',
                            'bak_penahan_tekanan', 'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk',
                            'tapping', 'mata_air'));

ALTER TABLE import_staging DROP CONSTRAINT import_staging_feature_type_check;
ALTER TABLE import_staging ADD CONSTRAINT import_staging_feature_type_check
    CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering',
                            'bak_penahan_tekanan', 'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk',
                            'tapping', 'mata_air'));
