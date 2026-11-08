import type { FeatureType } from './types'

/** Jenis aset yang ditambahkan di model aset (migrasi 0010). Semuanya titik, bukan garis. */
export const ASET_BARU = [
  'bak_penahan_tekanan',
  'menara_air',
  'hidran_umum',
  'intake',
  'pompa',
  'meter_induk',
] as const

export type AsetBaru = (typeof ASET_BARU)[number]

/** Label dan warna jenis aset baru. Dipakai bersama oleh peta, legenda, pencarian, dan panel info. */
export const ASET_LABEL: Record<AsetBaru, string> = {
  bak_penahan_tekanan: 'Bak penahan tekanan',
  menara_air: 'Menara air',
  hidran_umum: 'Hidran umum',
  intake: 'Intake (sumur/mata air)',
  pompa: 'Pompa',
  meter_induk: 'Meter induk',
}

/** Warna berbeda dari valve, reservoir, bronkaptering, dan sr. Pipa tetap biru. */
export const ASET_COLOR: Record<AsetBaru, string> = {
  bak_penahan_tekanan: '#4f46e5',
  menara_air: '#7c3aed',
  hidran_umum: '#dc2626',
  intake: '#15803d',
  pompa: '#ea580c',
  meter_induk: '#db2777',
}

export function isAsetBaru(t: FeatureType): t is AsetBaru {
  return (ASET_BARU as readonly string[]).includes(t)
}
