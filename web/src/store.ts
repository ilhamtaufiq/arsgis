import { create } from 'zustand'
import type { Bbox, FeatureType, PipeFeature, PipeFunction } from './types'
import type { DesaProps } from './cakupanApi'

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
  /** Desa yang dipilih di peta cakupan. Tidak bersamaan dengan `selected`. */
  desaSelected: DesaProps | null
  cakupanOpen: boolean
  /** Fitur yang peta harus pusatkan. Berbeda dari `selected`, karena klik di peta tidak memindahkan kamera. */
  focus: PipeFeature | null
  importOpen: boolean
  /** Unit SPAM yang dipilih untuk filter. Null berarti semua unit. */
  setNetwork: (id: number | null) => void
  setBbox: (bbox: Bbox) => void
  select: (feature: PipeFeature | null) => void
  setFocus: (feature: PipeFeature | null) => void
  selectDesa: (desa: DesaProps | null) => void
  toggleCakupan: () => void
  toggleImport: () => void
}

export const useMapStore = create<MapState>((set) => ({
  networkId: null,
  bbox: null,
  selected: null,
  desaSelected: null,
  cakupanOpen: true,
  focus: null,
  importOpen: false,
  setNetwork: (id) => set({ networkId: id, selected: null, focus: null }),
  setBbox: (bbox) => set({ bbox }),
  select: (feature) => set({ selected: feature, desaSelected: null }),
  setFocus: (feature) => set({ focus: feature }),
  selectDesa: (desa) => set({ desaSelected: desa, selected: null }),
  toggleCakupan: () => set((s) => ({ cakupanOpen: !s.cakupanOpen })),
  toggleImport: () => set((s) => ({ importOpen: !s.importOpen })),
}))
