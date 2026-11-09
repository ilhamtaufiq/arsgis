import { create } from 'zustand'
import type { Geometry } from 'geojson'
import type { Bbox, FeatureType, PipeFeature, PipeFunction } from './types'
import type { DesaProps } from './cakupanApi'
import { ASET_LABEL } from './asetStyle'

export const LAYER_LABELS: Record<Exclude<FeatureType, 'pipa'>, string> = {
  tapping: 'Tapping pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
  mata_air: 'Mata air',
  ...ASET_LABEL,
}

export const PIPE_LABELS: Record<PipeFunction, string> = {
  transmisi: 'Pipa transmisi',
  distribusi: 'Pipa distribusi',
}

// Semua layer aktif. Sambungan rumah (sr) hanya ikut diminta untuk editor dan admin (lihat App).
// Tapping dan mata air bukan data pelanggan, jadi tampil untuk semua pengunjung seperti valve dan reservoir.
export const DEFAULT_LAYERS: FeatureType[] = ['tapping', 'mata_air', 'valve', 'reservoir', 'bronkaptering', 'sr']
export const DEFAULT_PIPE_FUNCTIONS: PipeFunction[] = ['transmisi', 'distribusi']

/**
 * Mode edit di peta. Hanya dipakai editor dan admin.
 * - tambah: menunggu klik di peta. Titik langsung membuka form; pipa mengumpulkan `titik` (lng, lat) dulu.
 * - pindah: aset asli disembunyikan dan digantikan objek yang bisa diseret.
 */
export type ModeEdit =
  | { kind: 'tambah'; jenis: FeatureType; titik: [number, number][] }
  | { kind: 'pindah'; feature: PipeFeature }
  | null

/** Form atribut yang terbuka: fitur baru dengan geometri yang sudah ditentukan, atau fitur yang diubah. */
export type FormulirAset =
  | { kind: 'buat'; jenis: FeatureType; geometry: Geometry }
  | { kind: 'ubah'; feature: PipeFeature }
  | null

type MapState = {
  networkId: number | null
  bbox: Bbox | null
  selected: PipeFeature | null
  /** Desa yang dipilih di peta cakupan. Tidak bersamaan dengan `selected`. */
  desaSelected: DesaProps | null
  cakupanOpen: boolean
  /** Fitur yang peta harus pusatkan. Berbeda dari `selected`, karena klik di peta tidak memindahkan kamera. */
  focus: PipeFeature | null
  /** Panel kiri yang terbuka: daftar layer, alat gambar, edit aset, atau tidak ada. */
  panel: 'layer' | 'alat' | 'edit' | null
  /** Alat gambar yang sedang menunggu klik di peta. */
  drawMode: 'rectangle' | null
  /** Jumlah bentuk yang sudah digambar pengguna. */
  drawnCount: number
  editor: ModeEdit
  formulir: FormulirAset
  setPanel: (panel: 'layer' | 'alat' | 'edit' | null) => void
  setDrawMode: (mode: 'rectangle' | null) => void
  setDrawnCount: (n: number) => void
  /** Unit SPAM yang dipilih untuk filter. Null berarti semua unit. */
  setNetwork: (id: number | null) => void
  setBbox: (bbox: Bbox) => void
  select: (feature: PipeFeature | null) => void
  setFocus: (feature: PipeFeature | null) => void
  selectDesa: (desa: DesaProps | null) => void
  toggleCakupan: () => void
  setEditor: (editor: ModeEdit) => void
  setFormulir: (formulir: FormulirAset) => void
}

export const useMapStore = create<MapState>((set) => ({
  networkId: null,
  bbox: null,
  selected: null,
  desaSelected: null,
  cakupanOpen: true,
  focus: null,
  panel: null,
  drawMode: null,
  drawnCount: 0,
  editor: null,
  formulir: null,
  setPanel: (panel) => set((s) => ({ panel: s.panel === panel ? null : panel, drawMode: null })),
  setDrawMode: (mode) => set({ drawMode: mode }),
  setDrawnCount: (n) => set({ drawnCount: n }),
  setNetwork: (id) => set({ networkId: id, selected: null, focus: null, editor: null, formulir: null }),
  setBbox: (bbox) => set({ bbox }),
  select: (feature) => set({ selected: feature, desaSelected: null }),
  setFocus: (feature) => set({ focus: feature }),
  selectDesa: (desa) => set({ desaSelected: desa, selected: null }),
  toggleCakupan: () => set((s) => ({ cakupanOpen: !s.cakupanOpen })),
  setEditor: (editor) => set({ editor }),
  setFormulir: (formulir) => set({ formulir }),
}))
