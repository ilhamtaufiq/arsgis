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

// Sambungan rumah (sr) dinonaktifkan secara default: data pelanggan belum punya auth.
export const DEFAULT_LAYERS: FeatureType[] = ['valve', 'reservoir', 'bronkaptering']
export const DEFAULT_PIPE_FUNCTIONS: PipeFunction[] = ['transmisi', 'distribusi']

type MapState = {
  networkId: number | null
  bbox: Bbox | null
  layers: FeatureType[]
  pipeFunctions: PipeFunction[]
  selected: PipeFeature | null
  setNetwork: (id: number) => void
  setBbox: (bbox: Bbox) => void
  toggleLayer: (type: FeatureType) => void
  togglePipeFunction: (fungsi: PipeFunction) => void
  select: (feature: PipeFeature | null) => void
}

const toggle = <T,>(list: T[], item: T): T[] =>
  list.includes(item) ? list.filter((x) => x !== item) : [...list, item]

export const useMapStore = create<MapState>((set) => ({
  networkId: null,
  bbox: null,
  layers: DEFAULT_LAYERS,
  pipeFunctions: DEFAULT_PIPE_FUNCTIONS,
  selected: null,
  setNetwork: (id) => set({ networkId: id, selected: null }),
  setBbox: (bbox) => set({ bbox }),
  toggleLayer: (type) => set((s) => ({ layers: toggle(s.layers, type) })),
  togglePipeFunction: (fungsi) => set((s) => ({ pipeFunctions: toggle(s.pipeFunctions, fungsi) })),
  select: (feature) => set({ selected: feature }),
}))
