-- Kode aset unik: <kode_desa>-<jenis>-<nomor urut>, contoh 0012-pipa-000045.
-- Format dan alasannya ada di crates/pipa-core/src/kode.rs. Nomor urut diambil dari kode_aset_urut
-- dan dikunci per (kode_desa, jenis) di dalam statement INSERT yang sama, sehingga tidak ada dua aset
-- yang mendapat nomor sama walau ditulis bersamaan. Nomor tidak pernah dipakai ulang, termasuk setelah hapus lunak.

ALTER TABLE features ADD COLUMN kode_aset TEXT;

CREATE TABLE kode_aset_urut (
    kode_desa TEXT NOT NULL,
    jenis     TEXT NOT NULL,
    terakhir  BIGINT NOT NULL DEFAULT 0 CHECK (terakhir >= 0),
    PRIMARY KEY (kode_desa, jenis)
);

-- 1. Isi kode untuk baris lama, berurutan menurut id. Tidak ada baris yang dihapus atau diubah selain kolom kode.
WITH asal AS (
    SELECT f.id, f.feature_type,
           CASE WHEN n.desa_id IS NULL
                THEN 'J' || lpad(f.network_id::text, 3, '0')
                ELSE lpad(n.desa_id::text, 4, '0')
           END AS kode_desa
    FROM features f
    JOIN networks n ON n.id = f.network_id
),
bernomor AS (
    SELECT id, kode_desa, feature_type,
           ROW_NUMBER() OVER (PARTITION BY kode_desa, feature_type ORDER BY id) AS nomor
    FROM asal
)
UPDATE features f
SET kode_aset = b.kode_desa || '-' || b.feature_type || '-' || lpad(b.nomor::text, 6, '0')
FROM bernomor b
WHERE f.id = b.id;

-- 2. Nomor terakhir per pasangan, dari kode yang baru diisi.
INSERT INTO kode_aset_urut (kode_desa, jenis, terakhir)
SELECT split_part(kode_aset, '-', 1), split_part(kode_aset, '-', 2), MAX(split_part(kode_aset, '-', 3)::bigint)
FROM features
WHERE kode_aset IS NOT NULL
GROUP BY 1, 2;

ALTER TABLE features ALTER COLUMN kode_aset SET NOT NULL;
CREATE UNIQUE INDEX features_kode_aset_uniq ON features (kode_aset);

-- 3. Kode dibuat otomatis saat insert bila kolom tidak diisi. Penulis (API, impor, atau SQL langsung) tidak perlu tahu formatnya.
CREATE FUNCTION features_isi_kode_aset() RETURNS trigger AS $$
DECLARE
    kd    TEXT;
    nomor BIGINT;
BEGIN
    IF NEW.kode_aset IS NOT NULL THEN
        RETURN NEW;
    END IF;
    SELECT CASE WHEN n.desa_id IS NULL
                THEN 'J' || lpad(NEW.network_id::text, 3, '0')
                ELSE lpad(n.desa_id::text, 4, '0')
           END
      INTO kd
      FROM networks n
     WHERE n.id = NEW.network_id;
    IF kd IS NULL THEN
        RAISE EXCEPTION 'jaringan % tidak ditemukan untuk kode aset', NEW.network_id USING ERRCODE = '23503';
    END IF;
    INSERT INTO kode_aset_urut (kode_desa, jenis, terakhir)
    VALUES (kd, NEW.feature_type, 1)
    ON CONFLICT (kode_desa, jenis) DO UPDATE SET terakhir = kode_aset_urut.terakhir + 1
    RETURNING terakhir INTO nomor;
    NEW.kode_aset := kd || '-' || NEW.feature_type || '-' || lpad(nomor::text, 6, '0');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER features_isi_kode_aset
    BEFORE INSERT ON features
    FOR EACH ROW EXECUTE FUNCTION features_isi_kode_aset();

-- 4. Kode tidak boleh berubah setelah dibuat, supaya riwayat dan referensi dari luar tetap valid.
CREATE FUNCTION features_kode_aset_tetap() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'kode aset % tidak boleh diubah', OLD.kode_aset USING ERRCODE = '23514';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER features_kode_aset_tetap
    BEFORE UPDATE OF kode_aset ON features
    FOR EACH ROW WHEN (NEW.kode_aset IS DISTINCT FROM OLD.kode_aset)
    EXECUTE FUNCTION features_kode_aset_tetap();
