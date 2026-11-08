-- Model aset lengkap: enam jenis aset baru, atribut tipis per aset, dan material yang diperluas.
-- Semua kolom baru nullable, jadi data lama tidak berubah dan jalur tulis yang ada tetap jalan.

-- 1. Jenis aset. Perlu mengganti CHECK di features dan import_staging, karena impor juga menulis jenis ini.
ALTER TABLE features DROP CONSTRAINT features_feature_type_check;
ALTER TABLE features ADD CONSTRAINT features_feature_type_check
    CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering',
                            'bak_penahan_tekanan', 'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk'));

ALTER TABLE import_staging DROP CONSTRAINT import_staging_feature_type_check;
ALTER TABLE import_staging ADD CONSTRAINT import_staging_feature_type_check
    CHECK (feature_type IN ('pipa', 'valve', 'reservoir', 'sr', 'bronkaptering',
                            'bak_penahan_tekanan', 'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk'));

-- 2. Material. Nilai lama (PVC, HDPE, GIP, DCI) tetap valid.
ALTER TABLE features DROP CONSTRAINT features_material_check;
ALTER TABLE features ADD CONSTRAINT features_material_check
    CHECK (material IS NULL OR material IN ('PVC', 'HDPE', 'GIP', 'DCI', 'Steel', 'ACP', 'Lainnya'));

-- 3. Atribut tipis. Satuan ada di nama kolom agar tidak perlu kamus tambahan.
ALTER TABLE features
    ADD COLUMN kapasitas_m3 NUMERIC CHECK (kapasitas_m3 IS NULL OR kapasitas_m3 >= 0),
    ADD COLUMN debit_lps    NUMERIC CHECK (debit_lps IS NULL OR debit_lps >= 0),
    ADD COLUMN elevasi_m    NUMERIC,
    ADD COLUMN tahun_bangun SMALLINT,
    ADD COLUMN sumber_dana  TEXT;

-- 4. Kondisi. Kolom `kondisi` sudah berisi teks bebas (mis. 'baik', 'perlu cek', 'aktif') dan tidak
-- dipetakan ulang, karena artinya tidak bisa dipastikan dari data yang ada. CHECK biasa tidak dipakai
-- karena PostgreSQL memvalidasi ulang setiap baris saat diubah, sehingga baris lama yang diperbarui
-- untuk kolom lain ikut gagal. Karena itu pemicu hanya memeriksa nilai baru, yaitu saat kolom
-- ditulis dan nilainya berbeda dari nilai sebelumnya.
CREATE FUNCTION cek_kondisi_fitur() RETURNS trigger AS $$
BEGIN
    IF NEW.kondisi IS NULL OR (TG_OP = 'UPDATE' AND NEW.kondisi IS NOT DISTINCT FROM OLD.kondisi) THEN
        RETURN NEW;
    END IF;
    IF NEW.kondisi NOT IN ('baik', 'rusak_ringan', 'rusak_berat', 'tidak_berfungsi') THEN
        RAISE EXCEPTION 'kondisi tidak valid: %', NEW.kondisi USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER features_cek_kondisi
    BEFORE INSERT OR UPDATE OF kondisi ON features
    FOR EACH ROW EXECUTE FUNCTION cek_kondisi_fitur();
