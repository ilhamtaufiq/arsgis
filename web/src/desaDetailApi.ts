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
  /** `override` bila Pokmas dikoreksi di arsgis, `arumanis` bila masih data hasil sinkron. */
  pokmas_sumber: SumberPokmas
}

export type SumberPokmas = 'override' | 'arumanis'

/** Jejak koreksi Pokmas. Hanya dikirim untuk editor dan admin. */
export type KoreksiPokmas = {
  catatan: string | null
  diperbarui_oleh: string
  diperbarui_pada: string
}

export type PokmasEfektif = {
  sumber: SumberPokmas
  pokmas: PokmasDesa | null
  koreksi: KoreksiPokmas | null
}

/** Isian form edit Pokmas. Teks kosong dibaca server sebagai tidak diisi. */
export type IsianPokmas = {
  nama: string
  perdes: string
  kepala: string
  bendahara: string
  sekretaris: string
  catatan: string
}

export async function fetchDesaDetail(id: number): Promise<DesaDetail> {
  const res = await fetch(`/api/v1/desa/${id}`)
  if (res.status === 404) throw new Error('Desa tidak ditemukan.')
  if (!res.ok) throw new Error(`Detail desa gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: DesaDetail }
  return body.data
}

export async function fetchDesaPokmas(id: number): Promise<PokmasEfektif> {
  const res = await fetch(`/api/v1/desa/${id}/pokmas`)
  if (res.status === 404) throw new Error('Desa tidak ditemukan.')
  if (!res.ok) throw new Error(`Data Pokmas gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: PokmasEfektif }
  return body.data
}

export function simpanDesaPokmas(id: number, isian: IsianPokmas): Promise<void> {
  return kirimPokmas(`/api/v1/desa/${id}/pokmas`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(isian),
  })
}

/** Kembalikan Pokmas ke data Arumanis dengan menghapus koreksi. */
export function hapusDesaPokmas(id: number): Promise<void> {
  return kirimPokmas(`/api/v1/desa/${id}/pokmas`, { method: 'DELETE' })
}

async function kirimPokmas(url: string, init: RequestInit): Promise<void> {
  const res = await fetch(url, init)
  if (res.ok) return
  let pesan = `Penyimpanan gagal (${res.status})`
  try {
    const body = (await res.json()) as { error?: string }
    if (body.error) pesan = body.error
  } catch {
    // Badan respons bukan JSON: pakai pesan status saja.
  }
  throw new Error(pesan)
}
