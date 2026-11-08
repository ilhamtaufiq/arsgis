import type { Role } from './api'

export type CatalogKind = 'internal' | 'feature' | 'arcgis' | 'vector' | 'wms' | 'pmtiles'
export type CatalogRole = 'anon' | Role

/** Baris dari GET /api/v1/catalog (tabel `layer_catalog`). Server sudah menyaring menurut peran. */
export type CatalogRow = {
  slug: string
  nama: string
  kategori: string
  sumber: string
  kind: CatalogKind
  url: string | null
  params: Record<string, unknown> | null
  min_role: CatalogRole
  chip: boolean
  chip_urut: number | null
  /** Tanggal ISO sampai kapan tag BARU tampil. */
  baru_sampai: string | null
}

/** Layer yang bisa ditampilkan panel Layer. */
export type CatalogLayer = {
  slug: string
  nama: string
  kategori: string
  sumber: string
  kind: CatalogKind
  min_role: CatalogRole
  chip: boolean
  chip_urut: number | null
  baru_sampai: string | null
  /** Hanya untuk kind 'internal': kunci yang dikenali MapTools. */
  internalKey?: 'cakupan-spm'
}

export function toCatalogLayer(row: CatalogRow): CatalogLayer {
  const key = row.params?.internalKey
  return {
    slug: row.slug,
    nama: row.nama,
    kategori: row.kategori,
    sumber: row.sumber,
    kind: row.kind,
    min_role: row.min_role,
    chip: row.chip ?? false,
    chip_urut: row.chip_urut ?? null,
    baru_sampai: row.baru_sampai ?? null,
    internalKey: key === 'cakupan-spm' ? key : undefined,
  }
}

/** Daftar layer untuk peran pengguna saat ini. Peran ditentukan server dari cookie sesi. */
export async function fetchCatalog(): Promise<CatalogLayer[]> {
  const res = await fetch('/api/v1/catalog')
  if (!res.ok) throw new Error(`Daftar layer gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: CatalogRow[] }
  // Hanya layer internal yang dikenali yang bisa dinyalakan sekarang. Jenis lain (WMS, ArcGIS, ...) menyusul,
  // dan tidak ditampilkan dulu supaya tidak ada kotak centang yang tidak berfungsi.
  return body.data.map(toCatalogLayer).filter((l) => l.kind === 'internal' && l.internalKey !== undefined)
}

/** Tag BARU tampil sampai (dan termasuk) tanggal `baru_sampai`. Tanggal ISO bisa dibandingkan sebagai teks. */
export function isNew(layer: Pick<CatalogLayer, 'baru_sampai'>, today: string): boolean {
  return layer.baru_sampai !== null && layer.baru_sampai >= today
}

/** Layer untuk baris chip, urut menurut `chip_urut`. */
export function chipLayers(layers: CatalogLayer[]): CatalogLayer[] {
  return layers.filter((l) => l.chip).sort((a, b) => (a.chip_urut ?? 0) - (b.chip_urut ?? 0))
}

export type LayerFilter = { q: string; kategori: string; kind: string }

/** Menyaring daftar layer menurut kata kunci (nama, sumber, kategori), kategori, dan jenis. Kosong berarti semua. */
export function filterLayers(layers: CatalogLayer[], f: LayerFilter): CatalogLayer[] {
  const term = f.q.trim().toLowerCase()
  return layers.filter(
    (l) =>
      (f.kategori === '' || l.kategori === f.kategori) &&
      (f.kind === '' || l.kind === f.kind) &&
      (term === '' || [l.nama, l.sumber, l.kategori].some((t) => t.toLowerCase().includes(term))),
  )
}

/** Tanggal lokal hari ini sebagai YYYY-MM-DD. */
export function todayIso(now: Date = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}`
}

/** Teks dan gaya lencana jenis layer. Teks selalu ada, supaya jenis tidak hanya dibedakan lewat warna. */
export function kindBadge(kind: CatalogKind): { label: string; tone: 'feature' | 'service' | 'vector' } {
  switch (kind) {
    case 'internal':
      return { label: 'Internal', tone: 'feature' }
    case 'feature':
      return { label: 'Feature', tone: 'feature' }
    case 'arcgis':
      return { label: 'ArcGIS', tone: 'service' }
    case 'wms':
      return { label: 'WMS', tone: 'service' }
    case 'vector':
      return { label: 'Vektor', tone: 'vector' }
    case 'pmtiles':
      return { label: 'PMTiles', tone: 'vector' }
  }
}
