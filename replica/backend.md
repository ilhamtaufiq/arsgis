# Backend: status and checklist

Date: 2026-11-08. Scope: the vertical slice only. arsgis already had login, roles, PostGIS and sessions, so most of the usual backend list does not apply.

## Done

- **Migration** `crates/pipa-store/migrations/0008_layer_catalog.sql`: table `layer_catalog` (role-gated rows, `aktif` flag, chip fields), an `updated_at` trigger, and one seed row `cakupan-spm` (public). Applied automatically at startup by the existing `Store::migrate`. **Not run yet**, no database was available here.
- **Rule in one place** `pipa_core::catalog_roles(role)`: which `min_role` values a caller may see. Visitors see `anon` only; viewer adds `viewer`; editor adds `editor`; admin sees all.
- **Store** `Store::list_catalog(role)`: the role filter is in the SQL (`min_role = ANY(...)`, `aktif` only), so rows for higher roles never leave the database.
- **Route** `GET /api/v1/catalog` returns `{ "data": [...] }`. Public, role-filtered from the session cookie. Sent with `Cache-Control: private, max-age=60` and `Vary: Cookie` so a shared proxy cannot serve one role's list to another.
- **Front end** `fetchCatalog()` now calls the real route. Layer kinds the panel cannot toggle yet (WMS, ArcGIS, ...) are filtered out in the client.
- **Migration 0009** `0009_catalog_chip.sql`: turns on the quick-chip flag for `cakupan-spm` so the new chip row has something to show. Also added `crates/pipa-store/build.rs` (one line, `rerun-if-changed=migrations`): without it Cargo does not rebuild when a migration file is added, and a new migration can be silently left out of the binary. Both migrations were confirmed present in the built test binary; neither has been applied to a database yet.
- **Admin routes** (admin role only, same `Admin` check as the other `/api/v1/admin/*` routes):
  - `GET /api/v1/admin/catalog`: every row, including inactive ones and all roles, with `id` and `aktif`.
  - `POST /api/v1/admin/catalog`: add a layer. 201 with `{id, slug}`; 400 with a plain message for bad input; 409 for a duplicate slug.
  - `PATCH /api/v1/admin/catalog/{id}`: change the fields you send. `slug` and `kind` cannot be changed, because clients use them as keys. Values cannot be cleared through PATCH. 404 for an unknown id; 400 when the combination breaks a rule (a chip layer needs `chip_urut`, a non-internal layer needs `url`).
  - `DELETE /api/v1/admin/catalog/{id}`: 204, or 404. Setting `aktif: false` through PATCH is the gentler option.
  - Validation lives in `crates/pipa-store/src/catalog_input.rs`: slug format, https-only URLs, known kinds and roles, JSON size limits, real calendar dates, unknown fields rejected. Each change is logged with the admin's username.

## Not needed for this slice

- **Payments, email, background jobs, third-party integrations:** none in `architecture.md`. Not built.
- **Sign-up, password reset, OAuth:** arsgis creates accounts by CLI and admin only, on purpose. Left as is.
- **New env vars:** none. `.env.example` is unchanged.

## Not done yet (from `architecture.md`)

- A screen for the admin routes. They exist only as API calls; the Admin page does not use them yet.
- Tables `user_layers`, `user_features` and `ref_wilayah`, with their routes (saved drawings, region search).
- The optional allow-listed layer proxy.
- A seed script with more layers. Only the one row from the migration exists.

## Checks run

- `cargo test -p pipa-core -p pipa-api -p pipa-store`: pass (26 tests, 2 ignored). That includes `catalog_roles_never_expose_higher_roles` and 12 validation tests for the admin input (bad slugs, `http://` and `javascript:` URLs, unknown fields, impossible dates, empty PATCH, and so on).
- `bun run typecheck` and `bun run test` in `web/`: pass.
- **Written but not run** (both in `pipa-store`, marked `#[ignore]`, need PostgreSQL with PostGIS; run with `cargo test -p pipa-store -- --ignored` once `DATABASE_URL` points at a server):
  - `katalog_tidak_membocorkan_layer_peran_lebih_tinggi`: visitor, viewer, editor and admin each see only their layers, and inactive layers stay hidden.
  - `admin_katalog_tambah_ubah_hapus`: create, duplicate slug, visibility by role, patch, constraint error, delete.
- **Not checked at all:** the SQL of the new insert and update queries, the migration against a real database, the HTTP status codes of the admin routes, and the panel against the real route. No Postgres was listening here, and the 401/403 behaviour of the new routes relies on the existing `Admin` extractor without a new test.

## Security checklist

- [x] secrets only in env vars, `.env` is in `.gitignore` (`git check-ignore` confirms), `.env.example` has names only
- [x] input validated on the server: `GET /api/v1/catalog` takes none; the admin routes validate every field (unit-tested)
- [~] authorisation tested with another role: rule unit-tested and DB test written, DB test not run
- [ ] rate limits on login and other routes: none found in the API (no limiter dependency in `pipa-api`). Existing gap, not touched
- [x] no webhooks in this slice
- [x] no uploads added
- [x] no user data in URLs or logs added
- [ ] dependency audit (`cargo audit`, `bun audit`): not run
- [ ] privacy policy: not checked

## Before this ships

1. Start a Postgres with PostGIS, run the API once so migration 0008 applies, and open the Layer panel as a visitor and as a logged-in user.
2. Run the ignored database test.
3. Decide on rate limiting for login before the app goes on a public domain (the README already says not to deploy publicly yet).
