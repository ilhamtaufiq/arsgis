export type UnitDesa = {
  id: number
  nama: string | null
  sistem_layanan: string | null
  sumber_dana: string | null
  is_simspam: boolean
  sr: number
  kk: number
  jiwa: number
  bjp_kk: number
}

export type PokmasDesa = {
  nama: string | null
  perdes: string | null
  konflik: boolean
  /** Hanya ada untuk editor dan admin. */
  kepala?: string | null
  bendahara?: string | null
  sekretaris?: string | null
  sumber_unit_ids: number[]
}

export type DesaDetail = {
  summary: {
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
    cakupan_persen: number | null
    punya_pokmas: boolean
    pokmas_konflik: boolean
  }
  units: UnitDesa[]
  pokmas: PokmasDesa | null
}

export async function fetchDesaDetail(id: number): Promise<DesaDetail> {
  const res = await fetch(`/api/v1/desa/${id}`)
  if (res.status === 404) throw new Error('Desa tidak ditemukan.')
  if (!res.ok) throw new Error(`Detail desa gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: DesaDetail }
  return body.data
}
