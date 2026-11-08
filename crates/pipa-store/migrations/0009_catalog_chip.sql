-- Tampilkan layer cakupan SPM sebagai chip cepat di atas peta (baris chip membaca layer_catalog.chip).
-- Migrasi terpisah dari 0008 karena 0008 bisa saja sudah terpasang, dan berkas migrasi yang sudah jalan tidak boleh diubah.
UPDATE layer_catalog SET chip = TRUE, chip_urut = 1 WHERE slug = 'cakupan-spm' AND NOT chip;
