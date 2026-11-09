import type { Feature, Geometry } from 'geojson'

export type FeatureType =
  | 'pipa'
  | 'valve'
  | 'reservoir'
  | 'sr'
  | 'bronkaptering'
  | 'bak_penahan_tekanan'
  | 'menara_air'
  | 'hidran_umum'
  | 'intake'
  | 'pompa'
  | 'meter_induk'
  | 'tapping'

export type Network = {
  id: number
  nama: string
  unit_spam_id: number | null
  desa_id: number | null
  deskripsi: string | null
}

export type PipeFunction = 'transmisi' | 'distribusi'

export type FeatureProperties = {
  network_id: number
  feature_type: FeatureType
  nama: string | null
  diameter_mm: number | null
  material: string | null
  tahun_pasang: number | null
  kondisi: string | null
  status: string
  fungsi: PipeFunction | null
  // Atribut tambahan dari migrasi 0010. Opsional agar respons lama tetap valid.
  kapasitas_m3?: number | null
  debit_lps?: number | null
  elevasi_m?: number | null
  tahun_bangun?: number | null
  sumber_dana?: string | null
  // Dari migrasi 0013-0014. `terlayani` hanya ada untuk jenis sr dan hanya untuk peran internal.
  kode_aset?: string | null
  dusun_id?: number | null
  terlayani?: boolean | null
  // Paket pekerjaan asal aset. Null bila aset tidak terkait paket.
  pekerjaan_id?: number | null
  pekerjaan_nama?: string | null
}

export type PipeFeature = Feature<Geometry, FeatureProperties> & { id: number }

export type Bbox = { west: number; south: number; east: number; north: number }
