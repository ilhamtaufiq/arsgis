-- Panjang geodetik ruas pipa (meter), dihitung saat fitur ditulis agar dashboard tidak menghitung ulang setiap kali dibuka.

ALTER TABLE features ADD COLUMN panjang_m DOUBLE PRECISION;

CREATE OR REPLACE FUNCTION hitung_panjang_pipa() RETURNS trigger AS $$
BEGIN
    NEW.panjang_m := CASE
        WHEN NEW.feature_type = 'pipa' AND GeometryType(NEW.geom) IN ('LINESTRING', 'MULTILINESTRING')
            THEN ST_Length(NEW.geom::geography)
        ELSE NULL
    END;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER features_panjang_pipa
    BEFORE INSERT OR UPDATE OF geom, feature_type ON features
    FOR EACH ROW EXECUTE FUNCTION hitung_panjang_pipa();

-- Isi data yang sudah ada. Pemicu di atas ikut berjalan karena kolom geom diperbarui.
UPDATE features SET geom = geom WHERE feature_type = 'pipa';
