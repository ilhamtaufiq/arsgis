# Rencana arsgis: SPM Air Minum + Data Jaringan Perpipaan

Status: fase A selesai (25 Oktober 2026). Fase B sampai E belum dikerjakan.

## 1. Tujuan

arsgis adalah peta kerja **SPM air minum** Kabupaten Cianjur.

- **Data SPAM air minum** (desa, Pokmas, unit SPAM, target, capaian, cakupan SPM) **diambil dari Arumanis**. Arumanis tetap sumber kebenaran.
- **Fungsi utama arsgis**: menambahkan **data fisik jaringan per unit SPAM**: perpipaan (transmisi dan distribusi), sambungan rumah (SR), bronkaptering, dan reservoir.
- Hasilnya: angka SPM di Arumanis punya bukti spasial, dan selisih antara data tercatat dan data lapangan bisa dilihat per desa.

## 2. Model data

```
Kecamatan
 └─ Desa ────────────── batas desa (poligon BPS)
     ├─ Pokmas (tepat 1 per desa)
     └─ Unit SPAM (1 atau lebih per desa, dikelola Pokmas desa itu)
          ├─ Capaian per tahun (SR, KK, jiwa, BJP)   ← dari Arumanis
          └─ Fitur jaringan (milik arsgis)
               ├─ pipa (transmisi / distribusi)
               ├─ sambungan rumah (SR)
               ├─ bronkaptering
               ├─ reservoir
               └─ valve (opsional, sudah ada)
```

Aturan:

- **Satu desa satu Pokmas.**
- **Satu Pokmas atau desa bisa memiliki beberapa unit SPAM.**
- **Setiap fitur jaringan milik tepat satu unit SPAM.** Konsep `networks` yang sekarang diganti oleh unit SPAM.

## 3. Kondisi data di Arumanis (dicek 24 Oktober 2026, baca saja)

| Item | Temuan |
|---|---|
| Desa | 364 |
| Unit SPAM | 364, saat ini **1 unit per desa** |
| Pengelola (Pokmas) | Ada untuk 364 unit, tapi nama Pokmas hanya terisi **220** (60%) |
| Perdes, kepala, bendahara, sekretaris | **Belum terisi.** Berisi tanda `-` (215 sampai 219 unit) atau kosong. Angka 215 sampai 219 di versi awal dokumen ini keliru karena tanda `-` terhitung sebagai isi. |
| Sistem layanan | Gravitasi 191, Pompa 26, Perpipaan 4, kosong 143 |
| Tahun pembangunan | **Kosong semua** |
| Capaian | 360 unit punya capaian, 189 unit punya anggaran |
| SPM | Target 534.952 KK, JP 54.029 KK, BJP 18.172 KK, **cakupan 13,5%** |
| Batas desa | 360 poligon (HDX-BPS 2020) di `apiamis/resources/geojson/id3203_cianjur_simplified.geojson`, 360 cocok dengan Arumanis lewat nama kecamatan + desa |
| Desa tanpa poligon | Sukabungah, Sukajembar (Campaka), Cikidangbayangbang (Mande), Bumbangsari (Takokak) |

### Ketidakcocokan model yang perlu diputuskan

Di Arumanis, **Pokmas disimpan per unit SPAM** (`tbl_pengelola.unit_spam_id` unik), bukan per desa. Selama satu desa hanya punya satu unit, hasilnya sama. Begitu satu desa punya dua unit, Arumanis akan punya dua baris Pokmas untuk desa yang sama, dan isinya bisa berbeda.

Pilihan:

1. **arsgis menurunkan Pokmas per desa** dari pengelola unit-unitnya. Jika berbeda, tandai sebagai konflik untuk diperbaiki di Arumanis. Arumanis tidak diubah.
2. **Ubah Arumanis** agar Pokmas disimpan per desa (perubahan lintas repo: skema, API, dan frontend Arumanis).

Rekomendasi: pilihan 1 untuk sekarang, pilihan 2 dibahas terpisah bila memang dibutuhkan.

## 4. Integrasi dengan Arumanis

| Data | Endpoint Arumanis | Auth |
|---|---|---|
| Desa, kecamatan, target, capaian ringkas per desa | `GET /api/public/spam-units/map-stats` | Tidak perlu |
| Unit SPAM lengkap (pengelola, capaian, anggaran) | `GET /api/spam-units` | Bearer token |
| Detail unit | `GET /api/spam-units/{id}` | Bearer token |
| Ringkasan SPM | `GET /api/spam-units/stats` | Bearer token |

Cara kerja:

- **Sinkron berkala ke tabel referensi di arsgis** (bukan proxy langsung setiap request). Alasan: peta perlu join dengan geometri di PostGIS, arsgis tetap jalan jika Arumanis sedang tidak bisa diakses, dan beban ke Arumanis kecil.
- Jadwal: tiap 1 jam, ditambah tombol "Sinkron sekarang" untuk admin. Setiap sinkron dicatat (waktu, jumlah baris, error).
- **arsgis tidak menulis balik ke Arumanis** di fase ini.
- Akses data yang butuh login memakai **akun layanan khusus dengan hak baca saja**, tokennya di env `ARUMANIS_TOKEN`, tidak pernah di repo. Akun admin pribadi tidak dipakai.

## 5. Perubahan skema arsgis

Tabel referensi (hasil sinkron, hanya ditulis oleh proses sinkron):

| Tabel | Kolom utama |
|---|---|
| `ref_desa` | `id` (= id desa Arumanis), nama, kecamatan, `kode_bps`, `geom` MultiPolygon (nullable untuk 4 desa), target KK, BJP master |
| `ref_pokmas` | `desa_id` (unik), nama, perdes, kepala, bendahara, sekretaris, `konflik` (bool), `sumber_unit_ids` |
| `ref_unit_spam` | `id` (= id unit Arumanis), `desa_id`, nama, sistem layanan, kapasitas, sumber dana, SR, KK, jiwa, BJP tercatat |
| `sync_runs` | waktu mulai dan selesai, status, jumlah baris, pesan error |

Tabel milik arsgis:

- `features`: `network_id` diganti `unit_spam_id` (merujuk `ref_unit_spam`). Kolom lain tetap: jenis, geometri, nama, diameter, material, fungsi, panjang.
- `import_batches`: ditambah `unit_spam_id`. Impor selalu untuk satu unit.
- `networks`: dihapus. Data dev yang ada boleh dibuang.

Pencocokan batas desa: otomatis lewat nama kecamatan + nama desa (dinormalisasi), ditambah tabel koreksi manual. Hasil uji: 360 dari 360 poligon cocok.

## 6. Fitur

### Peta

- Poligon desa diwarnai menurut **cakupan SPM** (JP + BJP dibanding target).
- Klik desa: target, capaian, cakupan, Pokmas, dan daftar unit SPAM.
- Pilih unit SPAM: jaringannya tampil dan peta memusat ke sana.
- Layer: pipa transmisi, pipa distribusi, SR, bronkaptering, reservoir, valve.

### Input data jaringan (editor dan admin)

- Impor KML **per unit SPAM** (sudah ada alur staging, laporan, dan commit; tinggal dikaitkan ke unit).
- Validasi tambahan:
  - Fitur harus berada di dalam batas desa unit itu, dengan toleransi kecil. Yang di luar masuk laporan.
  - Duplikat geometri di unit yang sama.
  - Jenis titik yang tidak dikenali.
- Tahap berikutnya: gambar dan ubah fitur langsung di peta.

### Dashboard

- Cakupan SPM per kecamatan dan per desa (angka dari Arumanis).
- **Kelengkapan data per desa**: jaringan sudah dipetakan atau belum, profil Pokmas lengkap atau belum.
- **Rekonsiliasi SR**: jumlah titik SR di peta dibanding SR tercatat di Arumanis, per desa dan per unit.

### Akses

- Pengunjung: peta cakupan desa dan jaringan tanpa SR.
- Viewer: sama dengan pengunjung, ditambah daftar unit.
- Editor: impor dan ubah jaringan, melihat SR dan profil Pokmas.
- Admin: pengguna, sinkron, dan koreksi pencocokan desa.
- Nama pengurus Pokmas termasuk data pribadi, jadi hanya untuk editor dan admin.

## 7. Tahapan

| Fase | Isi | Hasil |
|---|---|---|
| **A. Referensi** | Tabel `ref_*`, sinkron dari Arumanis, muat batas desa, pencocokan desa | Data desa, Pokmas, dan unit tersedia di arsgis |
| **B. Peta SPM** | Peta cakupan per desa, panel desa dan unit | Peta cakupan SPM bisa dipakai |
| **C. Jaringan per unit** | `features` ke `unit_spam_id`, impor KML per unit, validasi batas desa | Data perpipaan bisa dikumpulkan per unit |
| **D. Dashboard & rekonsiliasi** | Kelengkapan data, SR peta dibanding SR tercatat | Selisih data terlihat per desa |
| **E. Lanjutan** | Edit di peta, satu login dengan Arumanis, tulis balik (bila disetujui) | Opsional |

Urutan kerja: A, lalu B, lalu C, lalu D. Fase E menunggu keputusan.

## 8. Hasil fase A

- Tabel `ref_batas_desa`, `ref_desa`, `ref_desa_kode_override`, `ref_unit_spam`, `ref_pokmas`, `sync_runs` (migrasi 0006).
- Crate `pipa-sync`: klien baca Arumanis, penurunan Pokmas per desa, pembaca GeoJSON BPS.
- Perintah `pipa-api load-batas-desa <file>` dan `pipa-api sync`; sinkron otomatis tiap `ARUMANIS_SYNC_MENIT`.
- Endpoint `GET /api/v1/desa`, `POST /api/v1/admin/sync`, `GET /api/v1/admin/sync-runs`; tab "Sinkron Arumanis" di halaman admin.
- Uji dengan data asli: 364 desa, 364 unit, 364 baris Pokmas (220 bernama, 0 konflik), 360 desa cocok dengan poligon, 4 tanpa poligon. Total target, JP, BJP, cakupan 13,50%, SR 54.265, dan KK 54.029 sama persis dengan Arumanis.
- Token Arumanis harus berbentuk `id|rahasia`. Nilai dari cookie (`id%7Crahasia`) otomatis di-decode.

## 9. Keputusan yang dibutuhkan

1. **Pokmas**: pakai pilihan 1 (diturunkan di arsgis, konflik ditandai) atau ubah Arumanis?
2. **Akun layanan**: bisakah dibuatkan akun Arumanis khusus dengan hak baca saja untuk arsgis?
3. **Unit SPAM baru**: dibuat di Arumanis dulu lalu tersinkron ke arsgis (disarankan), atau boleh dibuat dari arsgis?
4. **Login**: arsgis tetap punya akun sendiri, atau satu login dengan Arumanis? Cookie Arumanis berlaku untuk domain `.cianjur.space`, jadi jika arsgis di subdomain yang sama, satu login cukup mudah dibuat.
5. **4 desa tanpa poligon**: cukup tampil di daftar, atau ada batas desa yang lebih baru?
6. **Atribut SR**: apa saja yang dicatat per titik SR (misalnya nomor pelanggan atau nama kepala keluarga)? Ini menentukan aturan privasinya.
