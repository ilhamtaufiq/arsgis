import { create } from 'zustand'
import type { Bbox, FeatureType, PipeFeature } from './types'

export const LAYER_LABELS: Record<FeatureType, string> = {
  pipa: 'Pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
}

// Sambungan rumah (sr) dinonaktifkan secara default: data pelanggan belum punya auth.
export const DEFAULT_LAYERS: FeatureType[] = ['pipa', 'valve', 'reservoir', 'bronkaptering']

type MapState = {
  networkId: number | null
  bbox: Bbox | null
  layers: FeatureType[]
  selected: PipeFeature | null
  setNetwork: (id: number) => void
  setBbox: (bbox: Bbox) => void
  toggleLayer: (type: FeatureType) => void
  select: (feature: PipeFeature | null) => void
}

export const useMapStore = create<MapState>((set) => ({
  networkId: null,
  bbox: null,
  layers: DEFAULT_LAYERS,
  selected: null,
  setNetwork: (id) => set({ networkId: id, selected: null }),
  setBbox: (bbox) => set({ bbox }),
  toggleLayer: (type) =>
    set((s) => ({
      layers: s.layers.includes(type) ? s.layers.filter((t) => t !== type) : [...s.layers, type],
    })),
  select: (feature) => set({ selected: feature }),
}))
