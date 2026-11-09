// Klien API operasi dan pemeliharaan (Tahap C). Semua rute ada di /api/v1/om/... kecuali penulisan
// perbaikan, jadwal, dan rehabilitasi yang bernaung di bawah /api/v1/om/aset/{id}.

export type JenisGangguan = 'pipa_bocor' | 'air_mati' | 'kualitas_keruh' | 'lainnya'
export type StatusGangguan = 'dilaporkan' | 'ditangani' | 'selesai'
export type StatusRehab = 'tidak_ada' | 'perlu' | 'prioritas' | 'sedang_dikerjakan'
export type StatusRehabTampil = StatusRehab | 'belum_ditandai'
export type StatusJatuhTempo = 'terlambat' | 'hari_ini' | 'segera' | 'nanti'

export type FotoGangguan = {
  id: number
  gangguan_id: number
  mime: string
  ukuran_byte: number
  diunggah_pada: string
}

/** `pelapor` tidak ada untuk viewer. Kepala desa dan publik tidak melihat nama pelapor. */
export type Gangguan = {
  id: number
  network_id: number
  feature_id: number | null
  jenis: JenisGangguan
  lon: number | null
  lat: number | null
  deskripsi: string
  pelapor?: string
  status: StatusGangguan
  dilaporkan_pada: string
  diperbarui_pada: string
  diperbarui_oleh: string
  foto: FotoGangguan[]
}

export type Perbaikan = {
  id: number
  feature_id: number
  gangguan_id: number | null
  tanggal: string
  tindakan: string
  teknisi: string
  biaya: number | null
  catatan: string | null
  dicatat_oleh: string
  dicatat_pada: string
}

export type JadwalPerawatan = {
  id: number
  feature_id: number
  jenis: string
  interval_hari: number
  terakhir_dilakukan: string | null
  tanggal_berikutnya: string
  aktif: boolean
  diubah_oleh: string
  diubah_pada: string
}

export type JatuhTempoBaris = {
  jadwal_id: number
  feature_id: number
  feature_type: string
  nama_aset: string | null
  jenis: string
  interval_hari: number
  terakhir_dilakukan: string | null
  tanggal_berikutnya: string
  selisih_hari: number
  status: StatusJatuhTempo
}

export type BarisPrioritas = {
  feature_id: number
  feature_type: string
  nama_aset: string | null
  kondisi: string | null
  status_rehab: StatusRehabTampil
  prioritas: number | null
  catatan: string | null
}

export type AsetRehab = {
  feature_id: number
  status_rehab: StatusRehab
  prioritas: number | null
  catatan: string | null
  diubah_oleh: string
  diubah_pada: string
}

export type RiwayatOm = {
  sumber: string
  sumber_id: number
  aksi: string
  diubah_oleh: string
  diubah_pada: string
  sebelum: Record<string, unknown> | null
  sesudah: Record<string, unknown> | null
}

export type OmAset = {
  feature_id: number
  perbaikan: Perbaikan[]
  jadwal: JadwalPerawatan[]
  rehab: AsetRehab | null
  gangguan: Gangguan[]
  riwayat: RiwayatOm[]
}

export type BuatGangguan = {
  network_id: number
  feature_id?: number | null
  lon?: number | null
  lat?: number | null
  jenis: JenisGangguan
  deskripsi: string
  pelapor?: string
}

export type BuatPerbaikan = {
  tanggal: string
  tindakan: string
  teknisi: string
  biaya?: number | null
  catatan?: string | null
  gangguan_id?: number | null
}

export type BuatJadwal = {
  jenis: string
  interval_hari: number
  tanggal_berikutnya?: string | null
}

export type SimpanRehab = {
  status_rehab: StatusRehab
  prioritas?: number | null
  catatan?: string | null
}

/** Galat dari API O&M. Pesan diambil dari respons agar pengguna melihat alasan yang sebenarnya. */
export class OmApiError extends Error {
  readonly status: number
  constructor(status: number, message: string) {
    super(message)
    this.name = 'OmApiError'
    this.status = status
  }
}

const PESAN_STATUS: Record<number, string> = {
  400: 'Isian tidak valid.',
  401: 'Silakan masuk terlebih dahulu.',
  403: 'Peran Anda belum cukup untuk aksi ini. Butuh editor atau admin.',
  404: 'Data tidak ditemukan.',
  413: 'Berkas terlalu besar.',
  422: 'Aturan data tidak terpenuhi.',
}

async function galat(res: Response): Promise<OmApiError> {
  const body = (await res.json().catch(() => ({}))) as { error?: string }
  return new OmApiError(res.status, body.error ?? PESAN_STATUS[res.status] ?? `Permintaan gagal (${res.status}).`)
}

async function ambil<T>(url: string): Promise<T> {
  const res = await fetch(url)
  if (!res.ok) throw await galat(res)
  return ((await res.json()) as { data: T }).data
}

async function kirim<T>(url: string, metode: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method: metode,
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (!res.ok) throw await galat(res)
  return ((await res.json()) as { data: T }).data
}

export function daftarGangguan(filter: { networkId?: number; featureId?: number; status?: StatusGangguan }) {
  const q = new URLSearchParams()
  if (filter.networkId !== undefined) q.set('network_id', String(filter.networkId))
  if (filter.featureId !== undefined) q.set('feature_id', String(filter.featureId))
  if (filter.status) q.set('status', filter.status)
  return ambil<Gangguan[]>(`/api/v1/om/gangguan?${q}`)
}

export function ambilGangguan(id: number) {
  return ambil<Gangguan>(`/api/v1/om/gangguan/${id}`)
}

export function buatGangguan(payload: BuatGangguan) {
  return kirim<Gangguan>('/api/v1/om/gangguan', 'POST', payload)
}

export function ubahStatusGangguan(id: number, status: StatusGangguan) {
  return kirim<Gangguan>(`/api/v1/om/gangguan/${id}`, 'PATCH', { status })
}

/** Unggah foto mentah. Server memeriksa tipe dari tanda awal berkas, bukan dari header ini. */
export async function unggahFoto(gangguanId: number, berkas: File): Promise<FotoGangguan> {
  const res = await fetch(`/api/v1/om/gangguan/${gangguanId}/foto`, {
    method: 'POST',
    headers: { 'content-type': berkas.type || 'application/octet-stream' },
    body: berkas,
  })
  if (!res.ok) throw await galat(res)
  return ((await res.json()) as { data: FotoGangguan }).data
}

export function urlFoto(id: number) {
  return `/api/v1/om/foto/${id}`
}

export function jatuhTempo(networkId: number, hari: number) {
  return ambil<JatuhTempoBaris[]>(`/api/v1/om/jatuh-tempo?network_id=${networkId}&hari=${hari}`)
}

export function rehabPrioritas(networkId: number) {
  return ambil<BarisPrioritas[]>(`/api/v1/om/rehab-prioritas?network_id=${networkId}`)
}

export function omAset(featureId: number) {
  return ambil<OmAset>(`/api/v1/om/aset/${featureId}`)
}

export function tambahPerbaikan(featureId: number, payload: BuatPerbaikan) {
  return kirim<Perbaikan>(`/api/v1/om/aset/${featureId}/perbaikan`, 'POST', payload)
}

export function tambahJadwal(featureId: number, payload: BuatJadwal) {
  return kirim<JadwalPerawatan>(`/api/v1/om/aset/${featureId}/jadwal`, 'POST', payload)
}

/** Catat perawatan selesai. Tanpa tanggal, server memakai hari ini. */
export function catatJadwalSelesai(jadwalId: number, tanggal?: string) {
  return kirim<JadwalPerawatan>(`/api/v1/om/jadwal/${jadwalId}/catat`, 'POST', tanggal ? { tanggal } : {})
}

export function simpanRehab(featureId: number, payload: SimpanRehab) {
  return kirim<AsetRehab>(`/api/v1/om/aset/${featureId}/rehab`, 'PUT', payload)
}
