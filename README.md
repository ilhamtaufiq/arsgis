# arsgis

WebGIS perpipaan SPAM: GeoJSON per bbox, layer transmisi/distribusi, login dengan peran, dan basemap PMTiles self-hosted.

## Stack

- Backend: Rust (axum, sqlx), crate `pipa-core`, `pipa-store`, `pipa-api`
- Frontend: React + TypeScript + Vite, Leaflet, protomaps-leaflet (basemap PMTiles), TanStack Query, Zustand
- Database: PostgreSQL + PostGIS (database terpisah dari produksi Arumanis)

## Prasyarat

- Rust (stable), Bun, PostgreSQL 16 dengan ekstensi PostGIS
- `pmtiles` CLI, hanya untuk membuat file basemap (lihat di bawah)

## Menjalankan lokal

```bash
# 1. Database. Contoh memakai cluster lokal di port 5433 dengan user postgres tanpa password.
createdb -h 127.0.0.1 -p 5433 -U postgres arsgis_dev
psql "postgres://postgres@127.0.0.1:5433/arsgis_dev" -c "CREATE EXTENSION IF NOT EXISTS postgis;"

# 2. Konfigurasi. API membaca `.env` dari folder proyek. Variabel di shell tetap menang jika di-set.
cp .env.example .env          # isi DATABASE_URL dan JWT_SECRET (minimal 32 karakter)

# 3. Jalankan API. Migrasi berjalan saat startup.
cargo run -p pipa-api

# 4. Data contoh, sekali saja (setelah API pernah dijalankan supaya tabel sudah ada):
psql "$DATABASE_URL" -f crates/pipa-store/seed/dev.sql

# 5. Akun untuk login (password minimal 10 karakter, dibaca dari PIPA_PASSWORD atau stdin):
PIPA_PASSWORD='ganti-dengan-password' cargo run -p pipa-api -- create-user admin_dev admin

# 6. Batas desa (sekali, atau saat GeoJSON diperbarui) dan sinkron data SPM dari Arumanis:
cargo run -p pipa-api -- load-batas-desa data/batas-desa-cianjur.geojson
cargo run -p pipa-api -- sync      # sinkron sekali; server juga menyinkron otomatis (ARUMANIS_SYNC_MENIT)

# 7. Frontend mode pengembangan (Vite di 5173, meneruskan /api, /healthz, /basemap ke :8080)
cd web && bun install && bun run dev
```

Buka `http://localhost:5173`. Tanpa `cargo run` yang aktif, halaman tidak bisa memuat data.

### Build produksi

```bash
cd web && bun run build          # menghasilkan web/dist
cd .. && cargo build --release -p pipa-api
```

API menyajikan `web/dist` dari variabel `WEB_DIST`, dan basemap dari `BASEMAP_DIR` (default `data`).

## Basemap PMTiles

File basemap tidak di-commit (`data/*.pmtiles` ada di `.gitignore`). Buat file untuk area kerja dengan `pmtiles extract` dari build Protomaps:

```bash
# CLI: versi yang sudah diuji. Alternatif: unduh binary dari release go-pmtiles.
GOBIN=$HOME/bin go install github.com/protomaps/go-pmtiles@v1.22.1

mkdir -p data
$HOME/bin/go-pmtiles extract https://build.protomaps.com/20251201.pmtiles \
  data/basemap-cianjur.pmtiles --bbox=106.77,-7.40,107.75,-6.65 --maxzoom=15
```

Nama file dan bbox dapat diubah. Jika mengganti nama file, ubah juga `BASEMAP_URL` di `web/src/MapView.tsx`. Tanpa file ini, peta tetap tampil tetapi tanpa basemap.

## Endpoint

| Method | Path | Keterangan |
| --- | --- | --- |
| GET | `/healthz` | Health check (ping database) |
| GET | `/api/v1/networks` | Daftar jaringan |
| GET | `/api/v1/networks/:id/geojson?bbox=w,s,e,n&types=pipa,valve` | FeatureCollection dalam bbox, maksimal 2° per sisi dan 50.000 fitur. Tanpa `types`, hanya jenis publik yang dikembalikan. |
| POST | `/api/v1/auth/login` | Login (username, password), set cookie sesi `HttpOnly` |
| POST | `/api/v1/auth/logout` | Hapus cookie sesi |
| POST | `/api/v1/imports?network_id=N&nama_file=...&titik=sr` | Unggah KML ke staging untuk satu unit SPAM (`network_id` wajib). Fitur diperiksa terhadap batas desa unit. (editor, admin) |
| GET | `/api/v1/imports/:id` | Laporan dan status batch impor (editor, admin) |
| POST | `/api/v1/imports/:id/commit?network_id=N` | Simpan fitur yang di dalam batas desa; fitur di luar dilewati. Sekali saja per batch (editor, admin) |
| GET | `/api/v1/auth/me` | Pengguna saat ini (401 jika belum login) |
| GET | `/basemap/*` | Berkas PMTiles, dengan range request |
| GET | `/api/v1/desa` | Desa dengan capaian dan cakupan SPM (publik, tanpa nama pengurus Pokmas) |
| POST | `/api/v1/admin/sync` | Sinkron sekarang dari Arumanis (admin; 409 bila sinkron lain berjalan) |
| GET | `/api/v1/admin/sync-runs` | Riwayat sinkron (admin) |

## Peran

| Peran | Bisa melihat |
| --- | --- |
| Anonim | Pipa, valve, reservoir, bronkaptering |
| viewer | Sama dengan anonim |
| editor, admin | Termasuk `sr` (sambungan rumah) |

Fitur `sr` berisi data pelanggan. Endpoint GeoJSON mengembalikan 403 jika peran tidak boleh membaca jenis yang diminta.

## Data SPM air minum

Data desa, capaian, unit SPAM, dan Pokmas berasal dari Arumanis dan disimpan sebagai tabel referensi `ref_*`.
arsgis hanya membaca. Lihat `docs/rencana-spm-air-minum.md`.

- Desa dan capaian per desa: `GET /api/public/spam-units/map-stats` Arumanis (publik).
- Unit SPAM, pengelola, dan capaian per unit: `GET /api/spam-units` Arumanis (butuh `ARUMANIS_TOKEN`).
- Pokmas: satu per desa, diturunkan dari pengelola unit dengan id terkecil. Bila unit-unit di satu desa mencatat nama berbeda, `konflik` bernilai true.
- Batas desa: `data/batas-desa-cianjur.geojson` (HDX-BPS 2020, 360 desa). Desa dicocokkan lewat nama kecamatan + desa; koreksi manual di tabel `ref_desa_kode_override`.
- Setiap unit SPAM punya satu baris di `networks` (`unit_spam_id`, `desa_id`), dibuat dan diperbarui saat sinkron. Impor dan peta memakai baris ini sebagai jaringan.
- Impor per unit: titik harus berada di dalam batas desa unit; garis diterima bila minimal separuh panjangnya di dalam batas. Fitur di luar tidak disimpan. Desa tanpa batas tidak bisa diimpor.
- Sinkron yang gagal tidak menghapus data. Bila Arumanis mengembalikan 0 desa atau 0 unit, data lama dipertahankan.

## Variabel lingkungan

| Nama | Wajib | Keterangan |
| --- | --- | --- |
| `DATABASE_URL` | ya | Koneksi PostgreSQL |
| `JWT_SECRET` | ya | Minimal 32 karakter. Buat dengan `openssl rand -hex 32`. |
| `PORT` | tidak | Default 8080 |
| `WEB_DIST` | tidak | Default `web/dist` |
| `BASEMAP_DIR` | tidak | Default `data` |
| `ARUMANIS_API_URL` | tidak | Default `https://apiamis.cianjur.space/api` |
| `ARUMANIS_TOKEN` | tidak | Token akun Arumanis baca saja, untuk unit SPAM dan Pokmas |
| `ARUMANIS_SYNC_MENIT` | tidak | Jeda sinkron otomatis, default 60; 0 mematikan |
| `COOKIE_SECURE` | tidak | Set `true` di produksi (HTTPS) |
| `RUST_LOG` | tidak | Filter log tracing |

## Catatan keamanan

- Jangan deploy ke domain publik sebelum `JWT_SECRET` diganti dan `COOKIE_SECURE=true` berjalan di belakang HTTPS.
- Akun dibuat lewat `create-user`. Tidak ada registrasi publik.
