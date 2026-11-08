# syntax=docker/dockerfile:1.7
# arsgis dalam satu image: API Rust yang juga menyajikan frontend hasil build dan berkas basemap.
# Basemap (74 MB, tidak ada di git) dipasang lewat volume di /data/basemap, bukan di dalam image.

# --- 1. Frontend ---
FROM oven/bun:1 AS web
WORKDIR /src/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

# --- 2. Backend ---
FROM rust:1-bookworm AS api
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# Migrasi di crates/pipa-store/migrations ditanam ke binari saat kompilasi (sqlx::migrate!).
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p pipa-api \
 && cp target/release/pipa-api /usr/local/bin/pipa-api

# --- 3. Runtime ---
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /app app \
 && mkdir -p /app/web /app/data /data/basemap \
 && chown app /data/basemap
WORKDIR /app
COPY --from=api /usr/local/bin/pipa-api /app/pipa-api
COPY --from=web /src/web/dist /app/web/dist
# Dipakai sekali lewat: /app/pipa-api load-batas-desa /app/data/batas-desa-cianjur.geojson
COPY data/batas-desa-cianjur.geojson /app/data/batas-desa-cianjur.geojson

ENV PORT=8080 \
    WEB_DIST=/app/web/dist \
    BASEMAP_DIR=/data/basemap \
    COOKIE_SECURE=true \
    RUST_LOG=pipa_api=info,tower_http=info
USER app
EXPOSE 8080
# /healthz juga memeriksa koneksi database.
HEALTHCHECK --interval=30s --timeout=5s --start-period=40s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/healthz" || exit 1
CMD ["/app/pipa-api"]
