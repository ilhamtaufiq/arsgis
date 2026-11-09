// Akses API untuk pelanggan, dusun, cakupan, dan titik SR. Semua permintaan memakai cookie sesi.

export const GOLONGAN: { value: string; label: string }[] = [
  { value: 'sosial', label: 'Sosial' },
  { value: 'rumah_tangga', label: 'Rumah tangga' },
  { value: 'niaga', label: 'Niaga' },
  { value: 'industri', label: 'Industri' },
  { value: 'khusus', label: 'Khusus' },
]

export type StatusPelanggan = 'aktif' | 'putus'

export type Pelanggan = {
  id: number
  nama: string
  alamat: string | null
  nomor_meter: string | null
  golongan_tarif: string
  status: StatusPelanggan
  sr_feature_id: number
  kode_aset_sr: string
  dusun_id: number | null
  dusun_nama: string | null
  desa_id: number | null
  created_at: string
}

export type PelangganInput = {
  nama: string
  alamat: string | null
  nomor_meter: string | null
  golongan_tarif: string
  status: StatusPelanggan
  sr_feature_id: number
}

export type Dusun = {
  id: number
  desa_id: number
  nama: string
  jumlah_rumah: number
  jumlah_jiwa: number
  diperbarui_pada: string
}

export type DusunInput = { nama: string; jumlah_rumah: number; jumlah_jiwa: number }

export type SrPilihan = {
  id: number
  kode_aset: string
  nama: string | null
  dusun_id: number | null
  dusun_nama: string | null
  desa_id: number | null
}

export type Cakupan = {
  jumlah_sr: number
  sr_terlayani: number
  sr_belum: number
  persen_cakupan: number | null
  persen_sr_terlayani: number | null
  jiwa_terlayani_perkiraan: number | null
}

export type BarisCakupan = {
  dusun_id: number | null
  nama: string
  jumlah_rumah: number
  jumlah_jiwa: number
  cakupan: Cakupan
}

export type CakupanDesa = {
  desa_id: number
  per_dusun: BarisCakupan[]
  tanpa_dusun: BarisCakupan
  total: BarisCakupan
}

export type RiwayatItem = {
  aksi: 'tambah' | 'ubah' | 'hapus'
  diubah_oleh: string
  diubah_pada: string
  sebelum: Record<string, unknown> | null
  sesudah: Record<string, unknown> | null
}

export type DesaPilihan = { id: number; nama: string; kecamatan: string }

async function minta<T>(method: string, url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    headers: body !== undefined ? { 'content-type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) {
    const err = (await res.json().catch(() => ({}))) as { error?: string }
    throw new Error(err.error ?? `Permintaan gagal (${res.status})`)
  }
  return (await res.json()) as T
}

export function daftarDesa(): Promise<DesaPilihan[]> {
  return minta<{ data: DesaPilihan[] }>('GET', '/api/v1/desa').then((b) => b.data)
}

export function daftarPelanggan(q: {
  q?: string
  desaId?: number | null
  dusunId?: number | null
  status?: StatusPelanggan | ''
}): Promise<Pelanggan[]> {
  const p = new URLSearchParams()
  if (q.q) p.set('q', q.q)
  if (q.desaId != null) p.set('desa_id', String(q.desaId))
  if (q.dusunId != null) p.set('dusun_id', String(q.dusunId))
  if (q.status) p.set('status', q.status)
  return minta<{ data: Pelanggan[] }>('GET', `/api/v1/pelanggan?${p}`).then((b) => b.data)
}

export function simpanPelanggan(id: number | null, input: PelangganInput): Promise<Pelanggan> {
  const req = id === null ? minta<{ data: Pelanggan }>('POST', '/api/v1/pelanggan', input) : minta<{ data: Pelanggan }>('PUT', `/api/v1/pelanggan/${id}`, input)
  return req.then((b) => b.data)
}

export function riwayatPelanggan(id: number): Promise<RiwayatItem[]> {
  return minta<{ data: RiwayatItem[] }>('GET', `/api/v1/pelanggan/${id}/riwayat`).then((b) => b.data)
}

export function daftarSr(q: { desaId?: number | null; dusunId?: number | null; q?: string }): Promise<SrPilihan[]> {
  const p = new URLSearchParams()
  if (q.desaId != null) p.set('desa_id', String(q.desaId))
  if (q.dusunId != null) p.set('dusun_id', String(q.dusunId))
  if (q.q) p.set('q', q.q)
  return minta<{ data: SrPilihan[] }>('GET', `/api/v1/sr?${p}`).then((b) => b.data)
}

export function daftarDusun(desaId: number): Promise<Dusun[]> {
  return minta<{ data: Dusun[] }>('GET', `/api/v1/desa/${desaId}/dusun`).then((b) => b.data)
}

export function simpanDusun(desaId: number, dusunId: number | null, input: DusunInput): Promise<Dusun> {
  const req =
    dusunId === null
      ? minta<{ data: Dusun }>('POST', `/api/v1/desa/${desaId}/dusun`, input)
      : minta<{ data: Dusun }>('PUT', `/api/v1/dusun/${dusunId}`, input)
  return req.then((b) => b.data)
}

export function pasangSrKeDusun(featureId: number, dusunId: number | null): Promise<void> {
  return minta<unknown>('PUT', `/api/v1/features/${featureId}/dusun`, { dusun_id: dusunId }).then(() => undefined)
}

export function cakupanDesa(desaId: number): Promise<CakupanDesa> {
  return minta<{ data: CakupanDesa }>('GET', `/api/v1/desa/${desaId}/cakupan`).then((b) => b.data)
}
