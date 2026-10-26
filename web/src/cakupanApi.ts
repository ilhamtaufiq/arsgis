import type { FeatureCollection, Geometry } from 'geojson'

/** Properti satu desa dari `GET /api/v1/desa/geojson`. Nama pengurus Pokmas tidak pernah ikut. */
export type DesaProps = {
  id: number
  nama: string
  kecamatan: string
  kode_bps: string | null
  punya_batas: boolean
  target_kk: number
  jp_kk: number
  bjp_kk: number
  sr: number
  jiwa: number
  unit_count: number
  /** (JP + BJP) / target × 100. Null bila target 0. */
  cakupan_persen: number | null
  punya_pokmas: boolean
  pokmas_konflik: boolean
}

export type DesaGeo = FeatureCollection<Geometry, DesaProps>

export async function fetchDesaGeo(): Promise<DesaGeo> {
  const res = await fetch('/api/v1/desa/geojson')
  if (!res.ok) throw new Error(`Data cakupan desa gagal dimuat (${res.status})`)
  return (await res.json()) as DesaGeo
}

/** Skala warna tangerine untuk cakupan SPM. Desa tanpa target abu-abu. */
export const CAKUPAN_BINS = [
  { max: 25, color: '#FDE3D8', label: '< 25%' },
  { max: 50, color: '#F7B99C', label: '25 – 50%' },
  { max: 75, color: '#EE8A63', label: '50 – 75%' },
  { max: Infinity, color: '#C2461F', label: '≥ 75%' },
] as const

export const CAKUPAN_TANPA_TARGET = { color: '#E5E7EB', label: 'Tanpa target' }

export function cakupanColor(persen: number | null): string {
  if (persen === null) return CAKUPAN_TANPA_TARGET.color
  return CAKUPAN_BINS.find((b) => persen < b.max)!.color
}
