import type { Feature, Geometry } from 'geojson'

export type FeatureType = 'pipa' | 'valve' | 'reservoir' | 'sr' | 'bronkaptering'

export type Network = {
  id: number
  nama: string
  unit_spam_id: number | null
  desa_id: number | null
  deskripsi: string | null
}

export type FeatureProperties = {
  network_id: number
  feature_type: FeatureType
  nama: string | null
  diameter_mm: number | null
  material: string | null
  tahun_pasang: number | null
  kondisi: string | null
  status: string
}

export type PipeFeature = Feature<Geometry, FeatureProperties> & { id: number }

export type Bbox = { west: number; south: number; east: number; north: number }
