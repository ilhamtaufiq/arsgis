# arsgis

WebGIS perpipaan SPAM. Fase 1: viewer read-only (GeoJSON per bbox, layer switcher, info panel).

## Stack

- Backend: Rust (axum, sqlx), crate `pipa-core`, `pipa-store`, `pipa-api`
- Frontend: React + TypeScript + Vite, MapLibre GL, TanStack Query, Zustand
- Database: PostgreSQL + PostGIS (database GIS terpisah dari produksi Arumanis)

## Menjalankan lokal

```bash
# 1. Database PostGIS (contoh)
createdb arsgis_dev && psql arsgis_dev -c "CREATE EXTENSION postgis;"

# 2. API (migrasi berjalan saat startup)
cp .env.example .env
psql "$DATABASE_URL" -f crates/pipa-store/seed/dev.sql   # data contoh, sekali saja
cargo run -p pipa-api

# 3. Frontend (mode pengembangan, proxy ke :8080)
cd web && bun install && bun run dev
```

Build produksi: `cd web && bun run build`, lalu API menyajikan `web/dist` (variabel `WEB_DIST`).

## Endpoint

| Method | Path | Keterangan |
| --- | --- | --- |
| GET | `/healthz` | Health check (ping database) |
| GET | `/api/v1/networks` | Daftar jaringan |
| GET | `/api/v1/networks/:id/geojson?bbox=w,s,e,n&types=pipa,valve` | FeatureCollection dalam bbox, maksimal 2° per sisi dan 50.000 fitur |

## Catatan keamanan

- Fitur `sr` (sambungan rumah) termasuk data pelanggan. Saat ini **belum ada autentikasi**, jadi endpoint masih terbuka. Jangan deploy ke domain publik sebelum auth dan pemisahan peran selesai.
