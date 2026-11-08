# Architecture: arsgis map tools (a rebuild of Ruang Kita's core map, draw/measure and export)

Built from `replica/recon.md` and `replica/features.csv` (2026-11-08). arsgis already exists, so this is a plan to extend it, not to start fresh. Nothing below changes the Arumanis sync, the pipe network features or the PMTiles basemap.

## What arsgis already has (from README and file list, verify with `/replica-diff`)

Rust axum API with cookie login and roles (viewer, editor, admin), PostGIS features by bbox, KML import to staging, desa boundaries and SPM coverage, Arumanis sync, a React map with search (`Search.tsx`), legend (`Legend.tsx`), dashboard, admin and landing/login pages. I have not read the front-end code, so "has" means a file of that name exists.

## Stack

| layer | choice | why |
| --- | --- | --- |
| API | Rust, axum 0.8, sqlx 0.8 (existing) | already built, one binary, no reason to add a second service |
| database | PostgreSQL 16 + PostGIS (existing), plus `pg_trgm` | same DB; trigram index gives fast fuzzy region search |
| web | React 19 + Vite + TypeScript, Leaflet 1.9, Zustand, TanStack Query, Tailwind 4 and the existing ui components (existing) | keep, the recon map's screens fit panels on a Leaflet map |
| basemap | protomaps-leaflet with the self-hosted PMTiles (existing) | open data and no Google tiles. The basemap switcher adds raster options from open providers |
| auth | existing JWT cookie + argon2, no public sign-up | the original's sign-in only gates GeoTools; arsgis keeps its admin-created accounts |
| draw and edit | `@geoman-io/leaflet-geoman-free` | covers point, line, polygon, rectangle, circle, edit and delete in one library |
| measure and geoprocessing | `@turf/turf` in the browser | distance, area (geodesic), buffer, clip, intersect, union, dissolve with no server round trip |
| coordinates | `proj4` | DD, DMS, DMM and UTM conversion on WGS84 |
| import | `shpjs` (SHP, handles `.prj`), `@tmcw/togeojson` (GPX, KML), `jszip` (KMZ) | all client-side, files never leave the browser until the user saves a layer |
| export | `@mapbox/shp-write` (SHP), `html-to-image` (JPEG), `jsPDF` (PDF) | client-side first; GeoJSON is a plain Blob |
| external layers | `esri-leaflet` (ArcGIS REST), Leaflet `tileLayer.wms` | added directly from the browser |
| payments, email | none | the original takes donations only; not needed |
| jobs | none new. The existing sync loop stays; `load-wilayah` is a CLI like `load-batas-desa` | no schedule is needed for the new features |
| hosting | unchanged (the API serves `web/dist`) | |

GeoTIFF export is the one feature that does not fit client-side. It needs a raster render, so it is deferred and would need GDAL on the server.

## Schema

New tables: 4 (`layer_catalog`, `user_layers`, `user_features`, `ref_wilayah`). Full SQL in `replica/schema.sql`, written as a proposed migration `0008`. Not applied.
Access rules: **data-layer checks** in the axum handlers, the same way `sr` is already restricted. No row-level security.

- `layer_catalog.min_role` decides who sees a layer. Any layer holding customer data (`sr`) must be `editor`. Handlers filter the list by role, they never rely on the client hiding it.
- `user_layers` and `user_features` belong to one owner. Every query adds `owner_id = current_user`. An admin can list but not read other users' features.
- `ref_wilayah` and the catalog's public rows are read-only for everyone. Only the CLI loader and admin routes write.
- Disclaimer acceptance stays in the browser (`localStorage`), as in the original. No table.

Hard constraints the recon found:
- Region tree integrity: a province has no parent and every lower level must have one (`induk_sesuai_level`).
- Names are ambiguous ("Malang" matches Pemalang, Kab. Malang and Kota Malang), so each result carries the parent and the code, and results are ordered by level then name similarity.
- One layer name per owner (`UNIQUE (owner_id, nama)`).

## API

New routes, 12 plus 1 optional. Existing routes are unchanged.

| method path | does | who | input | output | flow |
| --- | --- | --- | --- | --- | --- |
| GET `/api/v1/catalog` | list active layers the caller may see, with legend and chip flags | anon (filtered by role) | `kategori`, `kind`, `q` | `[{slug, nama, kategori, sumber, kind, url, params, legend, style, chip, baru}]` | F02 |
| GET `/api/v1/admin/catalog` | list all rows including inactive | admin | none | rows | F02 |
| POST `/api/v1/admin/catalog` | add a catalog layer | admin | layer body | row | F02 |
| PATCH `/api/v1/admin/catalog/{id}` | edit or deactivate | admin | partial body | row | F02 |
| DELETE `/api/v1/admin/catalog/{id}` | remove | admin | none | 204 | F02 |
| GET `/api/v1/wilayah/search` | fuzzy search grouped by level, each row with parent and code | anon | `q` (min 2 chars), `limit` | `{kabkota:[…], kecamatan:[…], desa:[…]}` | F03 |
| GET `/api/v1/wilayah/{kode}/geojson` | boundary of one region, simplified | anon | `kode`, optional `simplify` | Feature | F03, region download |
| GET `/api/v1/my/layers` | list own saved layers | viewer+ | none | `[{id, nama, asal, jumlah_fitur}]` | F04, F06 |
| POST `/api/v1/my/layers` | save drawn or imported features as a layer | viewer+ | `{nama, asal, features}` (max 50,000, geometry validated) | `{id}` | F04, F06 |
| GET `/api/v1/my/layers/{id}/geojson` | load own layer | owner | none | FeatureCollection | F04, F06 |
| PUT `/api/v1/my/layers/{id}` | replace features after editing | owner | features | `{jumlah_fitur}` | F04 |
| DELETE `/api/v1/my/layers/{id}` | delete own layer | owner | none | 204 | F04 |
| (optional) GET `/api/v1/catalog/{slug}/proxy` | forward a request to a catalog service that lacks CORS | anon | path and query | upstream body | F02 |

The proxy is optional and only for hosts already in `layer_catalog.url`. It must never accept an arbitrary URL (SSRF). Layers a user adds through ArcGIS REST or GeoServer in GeoData stay browser-only and session-only.

Webhooks in or out: none.
Jobs: none new. The Arumanis sync stays as is. `load-wilayah` is a manual CLI run when BPS or BIG data is refreshed.

Public data sources only, with arsgis's own keys where needed. Nothing from the original's private endpoints.

## The parts that bite

- **Projections.** Everything is stored and drawn in EPSG:4326. Area and distance must be geodesic (turf uses a sphere, fine for a map tool, but state that it is approximate). UTM conversion must pick the zone from longitude. SHP import must read `.prj` and reproject, or reject files without one.
- **Region data is the big job.** The original lists 38 provinces, 541 kab/kota, 7,432 kecamatan, 84,503 desa from BIG RBI. arsgis only has 360 Cianjur desa (HDX-BPS 2020). Start with Jawa Barat or Cianjur, load more later, and check the licence of whichever national set is used.
- **Role leaks through export.** GeoJSON and SHP export of an active layer must use the data the API already returned to that role. Test that a viewer cannot export `sr`.
- **Size and speed.** The 50,000-feature cap and 2° bbox limit already exist; apply the same cap to imports and render big layers on a Leaflet canvas renderer. A 345-layer catalog list filters fine in the browser.
- **CORS and mixed content.** Many ArcGIS and WMS hosts do not send CORS headers or serve http. The catalog should store only layers that work from the browser, or use the allow-listed proxy.
- **Canvas export taint.** JPEG and PDF export from the map canvas fails if a raster tile layer has no CORS. Vector PMTiles basemap exports fine. Test each catalog layer type.
- **PDF scale bar.** Scale depends on latitude and zoom; compute it from the map's meters-per-pixel, do not hard-code.
- **Licences.** Do not use Google-style tiles. Keep OSM and Protomaps attribution on the map and in exports. Catalog entries need an `attribution` text.
- **Idempotency and races.** Saving a layer twice with the same name hits the unique constraint; return 409 and let the user rename. Replacing features (`PUT`) runs in one transaction.
- **Not needed:** time zones, payments, email, realtime, offline.
- **Deletion.** Deleting a user cascades their layers and features (`ON DELETE CASCADE`).

## Build order

Milestone screens use the S-IDs from `recon.md`.

1. **Vertical slice (a few days).** Draw a rectangle and export GeoJSON, plus one catalog layer toggled from the Layer Aktif panel.
   - Screens: S02, S05, S09 (Draw only), S09c.
   - Client only for draw and export (geoman, Blob download).
   - Tables: `layer_catalog`. Routes: `GET /catalog`.
2. **Must-haves.**
   - Layers: catalog list with search and filters, toggle, active panel, legend, chips. Screens S05, S06, S10, S12. Routes: `GET /catalog`, admin catalog routes.
   - Search: S04 with `ref_wilayah` loaded for Cianjur first. Routes: `GET /wilayah/search`.
   - Draw and measure: all five draw tools, Distance and Area, edit and delete. Screens S09, S09c. Client only.
   - Export: GeoJSON, PDF with legend and scale. Screen S11.
   - Map: basemap switcher, coordinate grid, zoom. Disclaimer gate S03 with browser-stored consent.
3. **Should-haves.**
   - Saved layers: `user_layers`, `user_features`, the `/my/layers` routes.
   - Import: SHP, GeoJSON, GPX/KML, KMZ, ArcGIS REST (S07). Export SHP, JPEG. Region boundary download.
   - Tools: coordinate check and converter, a small geoprocessing set (buffer, clip, intersect, union, dissolve) with a client-side history tab.
   - GPS location.
4. **Could-haves.** X,Y table import, WFS, Google Timeline, reverse geocode, GPX animation, GeoTIFF export (server GDAL).
5. **Fixes from `/replica-entrepreneur`**, once it has run.

Out of scope, as in the recon: the original's layer data, Google-style tiles, satellite feeds, AI analysis, Geo* modules and its branding.

## Next

`/replica-design`: rebuild the panel and toolbar patterns as tokens that fit arsgis's existing Tailwind and ui components. Because arsgis already has a design, consider skipping to `/replica-build` for the vertical slice.
