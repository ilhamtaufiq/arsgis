-- Koreksi Pokmas yang diisi langsung di arsgis.
-- Tabel terpisah dari ref_pokmas karena ref_pokmas dikosongkan dan diisi ulang oleh setiap sinkron dari Arumanis.
-- Satu desa, satu Pokmas: baris di sini menggantikan seluruh data Pokmas hasil sinkron untuk desa itu.
-- Menghapus baris ini mengembalikan desa ke data Arumanis.
CREATE TABLE pokmas_override (
    desa_id          BIGINT PRIMARY KEY REFERENCES ref_desa (id),
    nama             TEXT NOT NULL,
    perdes           TEXT,
    kepala           TEXT,
    bendahara        TEXT,
    sekretaris       TEXT,
    catatan          TEXT,
    -- Username (bukan id) karena token sesi hanya membawa username, dan username unik di tabel users.
    diperbarui_oleh  TEXT NOT NULL REFERENCES users (username) ON UPDATE CASCADE,
    diperbarui_pada  TIMESTAMPTZ NOT NULL DEFAULT now()
);
