# Recon map: Ruang Kita (web)

Scope: core map, draw/measure and export. Not the satellite time-slider, regional statistics or the Geo* modules (GeoDisaster, GeoOSS, GeoFarm, GeoJakarta...).
For: the user's own project, arsgis (Rust API + web map). Ideas and patterns only.
Date: 2026-11-08

Evidence legend: **seen** = observed on screen this session, **text** = stated on the site's welcome dialog or landing text, **guess** = inferred, not verified.

## Sources

| # | source | URL | notes |
| --- | --- | --- | --- |
| 1 | marketing / welcome dialog | https://ruangkita.net/ | feature list in the welcome modal (S01). Main source of the feature list. |
| 2 | live app, logged out | https://ruangkita.net/ | S01 to S08 seen at human speed. Disclaimer accepted by the user. |
| 3 | usage guide ("Panduan penggunaan") | in-app button, no public URL found | not read yet |
| 4 | donation links | saweria.co/ruangkita, paypal.me | only revenue model seen. No pricing page, no changelog. |
| 5 | help center, app store, API docs | none found | web only, no native app |

Not done: no JS bundles read, no network calls logged, no logged-in features touched. GeoTools asks for an email and password or Google sign-in, so it is only documented from its welcome-dialog text.

## Core loop

Open the map, search or browse a region, switch on thematic layers from a big catalog, then measure, draw and export the result as GeoJSON or a PDF map.

## Screens

| ID | screen | route / how to reach | purpose | key components | states seen |
| --- | --- | --- | --- | --- | --- |
| S01 | Welcome dialog | first load, or "Buka panduan" button | pitch, feature list, links | feature cards (6), "Mulai Jelajahi", "Panduan penggunaan", Saweria, PayPal | filled |
| S02 | Main map | `/` after S01 | the product | search bar, layer chips row, bottom toolbar, zoom buttons, lat/long grid labels, basemap | filled (Surabaya default view) |
| S03 | Disclaimer / terms gate | after S01, before using the map | legal gate, acceptance recorded | scroll text (numbered points), checkbox, accept button disabled until ticked | unchecked, button disabled |
| S04 | Search results | type in "Cari wilayah" | find regions | search input with clear (x) button, dropdown grouped by level (KABUPATEN / KOTA, then KECAMATAN, ...), each row: icon, name, parent ("Jawa Timur"), code ("Kode: 35.73") | filled ("Malang": Kab. Pemalang, Kab. Malang, Kota Malang, then kecamatan matches). Empty and no-result not seen |
| S05 | Active layers panel | toolbar "Layer Aktif" | list and control layers that are on | right side panel, minimise, close, donate card, empty message "aktifkan layer dari Katalog Peta" | empty |
| S06 | All layers table | toolbar "Semua Tabel" | browse the catalog | left panel, layer search box, 3 filter selects, count ("345 layer"), rows with name, source, type badge (Feature / ArcGIS / Vektor) | filled |
| S07 | GeoData (data sources and importers) | toolbar "GeoData", right panel | add data: from hubs and services, or from the user's own files | shortcut pills (Peta interaktif, Peta wilayah, Citra satelit BMKG, Data statistik), accordion rows: GeoPortal Hub, Katalog Peta, ArcGIS REST (add service layer), GeoServer (WFS workspace and layers), Impor Tabel X,Y, Impor SHP, Impor GeoJSON, Impor GPX / KML, Impor KMZ, Timeline (Google Maps Timeline) | collapsed list, no login needed. Expanded rows not opened |
| S08 | Login / sign up | toolbar "GeoTools" | gate for GeoTools | split dialog, Masuk / Daftar tabs, email, password (min 6), forgot password, Google sign-in | filled, logged out |
| S09 | Alat panel (toolbox) | toolbar "Alat", right panel | one accordion of map tools | Geoprocessing card (tabs Alat / Riwayat, "Panduan singkat", "Tambahkan sumber data", tool search, 7 groups: Overlay Analysis 7, Proximity 4, Data Management 8, Extract 1, Generalization 1, Statistics 3, progress bar with Batalkan, history list), then sections Basemap, Cek Lokasi, Grid Koordinat, Konverter Koordinat (DD, DMS, DMM, UTM, WGS84), Draw (Point, Line, Polygon, Rectangle, Circle), Measure (Distance, Area), Export (Export PDF, Export TIF, Simpan JPEG) | filled, sections collapsed or open. Geoprocessing running state seen (progress bar), history empty |
| S09c | Drawn shape and edit toolbar | map after drawing | act on drawn features | small vertical toolbar at the bottom left: Export GeoJSON, Export SHP, draw rectangle, edit layers, delete layers. Rectangle is made by dragging. Success toast "1 fitur diekspor sebagai GeoJSON" | rectangle drawn, export toast |
| S10 | Legend panel | toolbar "Legenda", left panel | legend for active layers | panel title, minimise, close, donate card | empty (no active layers). Filled state not seen |
| S11 | Export (PDF, TIF, JPEG) | Alat > Export | save the map | three buttons: Export PDF, Export TIF (GeoTIFF), Simpan JPEG | buttons seen, not clicked (downloads). PDF preview not seen |
| S12 | Layer chips row | top bar, scrolls sideways | one-tap layers: Hotspot, Transjakarta, PM2.5, SST, Angin, Hujan Radar, Batas Provinsi, Radar Cuaca BMKG, VIIRS, ECMWF Fire, Konsesi (GFW) | chip with icon and "NEW" tag | seen |
| S13 | GeoNusa Hub / AI Geospatial Analysis (Beta) | buttons at the right of the search bar | portal and AI analysis | two icon buttons | not opened |

## Flows

```
F01 First visit
    S01 welcome -> S03 disclaimer (tick + accept) -> S02 map
    happy path clicks: 3 (Mulai Jelajahi, tick, accept)
    edge: accept button disabled until the box is ticked; choice recorded on this device (text)

F02 Turn on a layer
    S02 -> S12 chip tap  (or)  S02 -> S06 table -> pick layer
    -> S05 shows it as active, S10 legend updates (guess)
    happy path clicks: 1 (chip) or 3 (open table, search, pick)
    edge: 345 layers, so search and 3 filters (category, type, sort) are needed

F03 Find a place
    S02 -> type in search -> S04 dropdown grouped by level -> pick a result -> map flies there (last step guess)
    happy path clicks: 2 (type, pick)
    edge: ambiguous names (Malang matches Pemalang, Kab. Malang, Kota Malang), so the parent and code are shown on each row

F06 Add own data
    S02 -> toolbar GeoData (S07) -> Impor SHP / GeoJSON / GPX-KML / KMZ / Tabel X,Y -> layer appears in Layer Aktif
    or: ArcGIS REST / GeoServer (WFS) -> add service layer
    happy path clicks: 3 (guess, forms not opened). No login needed to reach the panel (seen)

F04 Measure and export
    S02 -> toolbar Alat -> open Draw -> pick Rectangle -> drag on map
    -> S09c toolbar: Export GeoJSON (1 click) or Export SHP
    S02 -> Alat -> Export -> Export PDF / Export TIF / Simpan JPEG
    happy path clicks: 4 to a GeoJSON file (seen), 3 to PDF (seen, not clicked)
    edge: Draw section is collapsed until opened; Alat panel scrolls a long way to reach Draw

F05 Use GeoTools (import files, GPX video)
    S02 -> GeoTools -> S08 login or sign up -> tool
    happy path clicks: 4+ and an account required (seen: login wall)
```

## Components

| component | variants | states | used on |
| --- | --- | --- | --- |
| Floating search bar | with 2 icon buttons at the right | idle, focused (guess) | S02 |
| Layer chip | normal, "NEW" red tag | off, on (guess), loading (guess) | S02, S12 |
| Bottom toolbar (round icon buttons) | 7 buttons: GeoTools, GeoData, Alat, Layer Aktif, Semua Tabel, Legenda, Help | idle, hover tooltip, active | S02 |
| Side panel | left (table), right (active layers) | open, minimised, closed | S05, S06 |
| Layer row | type badge: Feature, ArcGIS, Vektor | idle, hover (guess) | S06 |
| Filter select | 3 selects: category, type, sort | default shown as "Semua k..", "Semua ti..", "Nama A-.." | S06 |
| Donate card | Donasi (green), PayPal (outline) | static | S05, S06 |
| Modal | welcome, disclaimer, login | open | S01, S03, S08 |
| Zoom buttons | + and - | idle | S02 |
| Coordinate grid labels | lat (LS) and long (BT) along the edges | follows pan and zoom | S02 |
| Basemap | Google-style vector basemap | n/a | S02 |

## Inferred data model

```
Layer      id, title, source (BMKG, OPT Pertanian, Geologi...), kind (feature | arcgis | vector | raster),
           category, url_or_service, legend, style, is_new
           evidence: S06 rows and badges, "345 layer" count
           confidence: high
ActiveLayer  layer_id, order, opacity, visible
           evidence: S05 (Layer Aktif), welcome text
           confidence: medium
Region     id, level (province | regency | district | village), name, parent_id, geometry
           evidence: S01 text ("84.503 Desa/Kelurahan, 7.432 Kecamatan, 541 Kab/Kota, 38 Provinsi", BIG RBI June 2026)
           confidence: high (for the region tree), guess for the field names
Drawing    id, type (point | line | polygon | rectangle | circle), geometry, measure (length, area), props
           evidence: S01 text ("Gambar & Ukur")
           confidence: medium
Account    id, email, password_hash, google_id
           evidence: S08 form
           confidence: medium
Consent    accepted_at, device (local storage)
           evidence: S03 text "Pilihan Anda dicatat pada peramban ini"
           confidence: high
```

Relationships: Layer 1-n ActiveLayer, Region 1-n Region (parent), Account 1-n Drawing (for GeoTools, guess), Drawing is probably local only for guests.

## Feature matrix

See `features.csv`. Must: 17, should: 14, could: 7, skip: 5.

## Out of scope (cannot or should not be cloned)

- The 345-layer catalog itself: BMKG, BIG, Kementan and other agencies' data. arsgis can use the same public services under their own terms, but must not copy this site's curated list or layer styling wholesale.
- The Google-style basemap and its places data: licensed, use an open basemap.
- Satellite feeds (Himawari, MODIS, VIIRS, PACE, Sentinel): belongs to other slices, and needs its own agreement with each provider.
- The AI geospatial analysis (Beta): no behaviour seen.
- The brand, logo, name, copy and the "Satu Peta untuk Nusantara" tagline.
- The user base and the donation channel.

## Size

Screens 15 (12 seen, 3 partly or not seen), flows 6, entities 6. Hard parts: layer catalog with mixed sources (ArcGIS REST, WMS/vector tiles, feature services), tile and style performance for 345 layers, PDF export with legend and scale bar, region search over about 90k places. Size: M for the slice as scoped (an existing Rust API and web map in arsgis already covers the base).

## Gaps still open

Round 2 (viewport set to 1280x800, which fixed the panels not opening) closed S04, S07, S09, S10 and S11. Still not seen:
- S13 GeoNusa Hub and AI Geospatial Analysis (Beta): not opened.
- S10 legend with a layer on: the legend stayed empty after toggling the "Boundaries" chip, so filled-legend content is unknown.
- S11 PDF print preview, and what the Export TIF and JPEG outputs contain: download buttons not clicked.
- Expanded rows of GeoData (what the ArcGIS REST, GeoServer and import forms ask for) and the Geoprocessing tool forms.
- Distance and Area measure result display, Polygon, Line, Circle drawing.
- Empty and no-result states of search.
- The usage guide ("Panduan") content.

Note on the Alat panel: the original puts Draw, Measure, Export, Basemap, coordinate tools and the geoprocessing toolbox in one accordion, not in separate toolbar buttons. That is the pattern to compare against arsgis.
