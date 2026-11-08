# Build log

One line per screen. Date is 2026-11-08.

| ID | status | what is missing | notes |
| --- | --- | --- | --- |
| S02 | done (already existed) | none for this slice | Map, zoom and search were already in arsgis. Added two toolbar buttons (layers, drawing) and the left panel. |
| S05 | partial | only one layer; no opacity or ordering | Layer panel lists the catalog and a "Layer aktif" section with an off switch. Empty, loading (two skeleton rows), error with retry, and no-layers states are built. |
| S09 | partial | only the Rectangle tool | Draw panel with rectangle, result count, "Unduh GeoJSON", "Hapus semua". Esc cancels. geoman is installed, so Point, Line, Polygon and Circle are small next steps. Measure, Basemap, coordinate tools and geoprocessing are not started. |
| S09c | partial | Export SHP, per-shape edit and delete | Export GeoJSON works. Delete is "all at once". |
| S10 | partial | legend for catalog layers | arsgis already shows the coverage legend; it still disappears when its layer is turned off. |

## What is faked

Nothing is faked any more (updated after `/replica-backend`). `fetchCatalog()` in `web/src/catalogApi.ts` calls the real `GET /api/v1/catalog`, backed by migration `0008_layer_catalog.sql`. The route, migration and panel have not been run together against a real database yet; see `replica/backend.md`. The browser checks below were done earlier, against a stub.

## Checked

- `bun run typecheck` clean. `bun run test`: 5 tests pass (role filter for the catalog, export file name).
- In the browser at desktop and 390px width, using a stub for the login and data calls: layer toggle, rectangle draw, export file contents (a closed Polygon in lon/lat order), clear all, and the empty export state.
- Keyboard: buttons and checkboxes are reachable and show a focus ring. Not tested: full flow with keyboard only, a screen reader, and the real API.
- Console errors seen came from the test stub (no basemap file, wrong shape for the networks reply, Vite websocket), not from the new code.

## Harder or bigger than it looked

- The map is behind the login gate, so a real browser test needs the API and an account. I used a stub instead of the real credentials.
- Git Bash kept writing `bash.exe.stackdump` files into the working folder, and Vite crashed while watching one. I deleted them. If it keeps happening, the cause is the shell, not the app.
- `bun add` rewrote `web/bun.lock` (252 lines changed) as well as adding one line to `web/package.json`.

## Round 2: map page restyle (2026-11-08)

Done after `/replica-design`. Pattern and layout follow `replica/design/components.md`; colours, sizes and radii come from `web/src/map-tokens.css`. No raw hex in the new components.

| ID | status | what is missing | notes |
| --- | --- | --- | --- |
| S02 | partial | coordinate grid labels (GridLabel), basemap switcher | White floating search pill, chip row, round toolbar at bottom centre, zoom pill at bottom right. The unit pill ("arsgis" plus unit select) stays above the search because arsgis needs it; the reference has no such element. |
| S12 | partial | only one chip exists | `LayerChips` reads `layer_catalog.chip`. New migration `0009_catalog_chip.sql` turns the chip on for `cakupan-spm`. NEW tag shows until `baru_sampai`. |
| S05 | done for the slice | opacity and ordering | Active layers section inside the Layer panel. |
| S06 | partial | catalog has one layer, so the long-list case (windowing) is untested | Docked panel with search, two filters, count (`aria-live`) and kind badges. |
| S09 | partial | measure, basemap, coordinate and geoprocessing sections | Accordion with "Gambar" and "Hasil dan ekspor". |
| S08, S10, S11, S13 | not touched | | Login screen, legend panel as a separate screen, PDF/image export, GeoNusa. |

Changed: `App.tsx` (chip row, `has-panel` class), `MapView.tsx` (zoom control to bottom right), `MapTools.tsx` (rewritten), `tools.css` (rewritten). New: `LayerChips.tsx`, `useLayerToggle.ts`, `map-tokens.css`, `map-ui.css`, migration `0009`. `styles.css` was not edited; `map-ui.css` overrides it with a `.stage` prefix and loads after it.

Checked: `bun run typecheck` clean; `bun run test` 12 tests pass (filtering, chip order, NEW tag dates, badges). In the browser at 1280px and 390px, against a stub API (not the real backend): layer toggle from chip and panel, search filter with no-result text, minimise and restore, accordion open and close, rectangle draw, clear all, Esc closes the panel, bottom sheet on phone width. Console errors came only from the missing basemap file in the stub.

Not checked: keyboard-only walk through everything, a screen reader, the long layer list, and the page against the real API and database (migration 0009 has not been applied anywhere yet).

Screenshots: `replica/clone-screens/S02-map-restyled.jpg`, `S06-layer-table.jpg`, `S09-draw-accordion.jpg`, `S06-mobile-sheet.jpg`.

Harder than it looked: the old "dynamic island" search is styled in `styles.css` with dark colours and a width animation. It is overridden in CSS only, so the search logic did not change, but the override depends on load order and the `.stage` prefix. On phone width the sign-in pill collided with the zoom control and then with the unit pill; both were moved (account pill to the top right, search column down one row).
