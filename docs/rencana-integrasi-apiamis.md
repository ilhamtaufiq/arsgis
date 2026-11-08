# Rencana integrasi data jaringan dari apiamis dan bun

Status: 8 November 2026. Bagian 3 selesai. Bagian 4 belum dikerjakan karena `ARUMANIS_TOKEN` di `.env` kosong.

## 1. Yang ditemukan

| Sumber | Temuan |
|---|---|
| GeoJSON Cianjur | `apiamis/resources/geojson/id3203_cianjur_simplified.geojson`, `bun/.../id3203_cianjur_simplified.geojson`, dan `arsgis/data/batas-desa-cianjur.geojson` **identik** (hash sama, 360 desa, HDX-BPS-2020). `bun` hanya menambah pecahan per kecamatan dan versi tanpa penyederhanaan (3,5 MB). Tidak ada data baru, tidak ada tindakan. |
| Peta perpipaan | Tabel `tbl_peta_peripaan`: GeoJSON hasil unggah KML/KMZ per pekerjaan. Rute `GET /api/peripaan?pekerjaan_id=&per_page=` (daftar tanpa geometri, memuat `file_url`, `media_id`, `pekerjaan_id`) dan `GET /api/peripaan/{id}` (dengan `geojson`). Hanya perlu login. |
| Tautan ke unit SPAM | `tbl_unit_spam_pekerjaan` menautkan pekerjaan ke unit SPAM. `GET /api/spam-units/air-minum-pekerjaan` memuat `linked_unit_ids` per pekerjaan. Hasilnya dibatasi menurut peran pemanggil (`access::restriction`). |
| Titik survei | Tabel `tbl_survey_lokasi` (lat, lng, jenis SPAM perpipaan, pengeboran, MCK, status verifikasi) lewat `GET /api/survey-lokasi`. Responsnya memuat nama surveyor, alamat, dan `detail` JSON. **Ditunda**; kalau dipakai, sembunyikan bidang itu dari pengunjung. |
| Koordinat foto | `foto_pekerjaan.koordinat` (teks). Belum dikaji. |
| KMZ lokal | 4 berkas di `apiamis/storage/app/public/12219..12222` (dua identik): jaringan sekitar Bojongkasih (17 fitur), Cikalongkulon (36), Karangwangi (13). |

## 2. Cocok dengan tujuan arsgis

`docs/rencana-spm-air-minum.md` menyebut tugas utama arsgis adalah data fisik jaringan per unit SPAM. Arumanis sudah menyimpannya sebagai peta perpipaan per pekerjaan, jadi tidak perlu digambar ulang.

## 3. Sudah dikerjakan

- Unggahan impor menerima **KMZ** selain KML. Dikenali dari isi berkas (awalan zip), bukan dari nama. Dipakai `doc.kml`, atau berkas `.kml` pertama. Isi dibatasi 50 MB setelah dibuka. `pipa_import::kml_text`, 6 tes.
- **Klasifikasi titik diperbaiki** dengan data nyata: singkatan `RV` (reservoir) dan `BRONCAP` (bronkaptering) sebelumnya jatuh ke `sr`, sehingga angka rekonsiliasi SR salah. Sekarang pencocokan per kata. Hasil pada tiga KMZ: Bojongkasih 1 bronkaptering + 10 titik bernomor; Cikalongkulon 2 bronkaptering + 1 reservoir; Karangwangi 1 bronkaptering + 1 reservoir + 5 titik bernomor.
- Titik **bernomor** ("BJKASIH 10") tetap tanpa jenis dan masuk ke jenis default yang dipilih editor saat unggah (biasanya `sr`).

## 4. Belum dikerjakan: tarik peta perpipaan dari Arumanis

Alur yang dirancang, memakai pola sinkron yang sudah ada:

1. Ambil `GET /api/spam-units/air-minum-pekerjaan`, ambil pekerjaan yang punya `linked_unit_ids`.
2. Untuk tiap pekerjaan, `GET /api/peripaan?pekerjaan_id=...&per_page=-1`, ambil `file_url` dan `media_id`.
3. Unduh KMZ dari `file_url`, lalu `kml_text` dan `parse_kml`.
4. Simpan sebagai batch **staging** per unit (`dibuat_oleh = sinkron:apiamis`). Editor meninjau dan menyimpan, seperti impor manual. Batas desa unit tetap memfilter fitur di luar desa.
5. Idempotensi: simpan `media_id` dan hash isi di batch. Berkas yang sama tidak diimpor dua kali (KMZ 12219 dan 12220 identik).

Syarat dan keputusan yang masih terbuka:

- **Kunci API sudah ditulis, belum dipakai.** Di apiamis: tabel `api_keys`, header `X-API-Key`, `GET /api/public/v1/spam-units` dan `GET /api/public/v1/peripaan` (dengan `unit_spam_ids`), serta perintah `create-api-key`. Di arsgis: `ARUMANIS_API_KEY` dipakai untuk sinkron unit SPAM. Belum dijalankan terhadap MySQL nyata. Langkah berikutnya: jalankan `apiamis-api migrate`, buat kunci, isi `ARUMANIS_API_KEY`, lalu tarik daftar peripaan dan unduh KMZ-nya (belum ditulis di arsgis).
- Token Sanctum (`ARUMANIS_TOKEN`) tetap menjadi cadangan, tetapi kedaluwarsa (default 12 jam) dan kosong di `.env`.
- Apakah `file_url` (`{app_url}/storage/{media_id}/{file_name}`) bisa diunduh tanpa login? Belum dicek.
- Satu pekerjaan bisa tertaut ke lebih dari satu unit. Aturan yang diusulkan: fitur masuk ke unit yang batas desanya memuatnya, sisanya dilewati dan dilaporkan.
- Titik bernomor dari sinkron otomatis: jenis default `sr` (data pelanggan, hanya editor) atau dilewati? Disarankan **dilewati** sampai ada keputusan, supaya angka SR tidak terisi tebakan.
- Otomatis disimpan atau tetap ditinjau editor? Disarankan tetap ditinjau.
- Risiko nama: konvensi nama di lapangan tidak seragam (`MKJKL` dan `MKJCKL` di satu berkas). Titik dengan nama tak dikenal dilaporkan, bukan ditebak.
