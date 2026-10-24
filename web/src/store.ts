import { create } from 'zustand'
import type { Bbox, FeatureType, PipeFeature, PipeFunction } from './types'

export const LAYER_LABELS: Record<Exclude<FeatureType, 'pipa'>, string> = {
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
}

export const PIPE_LABELS: Record<PipeFunction, string> = {
  transmisi: 'Pipa transmisi',
  distribusi: 'Pipa distribusi',
}

// Semua layer aktif. Sambungan rumah (sr) hanya ikut diminta untuk editor dan admin (lihat App).
export const DEFAULT_LAYERS: FeatureType[] = ['valve', 'reservoir', 'bronkaptering', 'sr']
export const DEFAULT_PIPE_FUNCTIONS: PipeFunction[] = ['transmisi', 'distribusi']

type MapState = {
  networkId: number | null
  bbox: Bbox | null
  selected: PipeFeature | null
  /** Fitur yang peta harus pusatkan. Berbeda dari `selected`, karena klik di peta tidak memindahkan kamera. */
  focus: PipeFeature | null
  importOpen: boolean
  dashboardOpen: boolean
  setNetwork: (id: number) => void
  setBbox: (bbox: Bbox) => void
  select: (feature: PipeFeature | null) => void
  setFocus: (feature: PipeFeature | null) => void
  toggleImport: () => void
  toggleDashboard: () => void
}

export const useMapStore = create<MapState>((set) => ({
  networkId: null,
  bbox: null,
  selected: null,
  focus: null,
  importOpen: false,
  dashboardOpen: false,
  setNetwork: (id) => set({ networkId: id, selected: null, focus: null }),
  setBbox: (bbox) => set({ bbox }),
  select: (feature) => set({ selected: feature }),
  setFocus: (feature) => set({ focus: feature }),
  toggleImport: () => set((s) => ({ importOpen: !s.importOpen })),
  toggleDashboard: () => set((s) => ({ dashboardOpen: !s.dashboardOpen })),
}))
