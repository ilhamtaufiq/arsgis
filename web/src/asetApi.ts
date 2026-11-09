import type { Geometry } from 'geojson'
import type { PipeFeature } from './types'
import type { PayloadBuat, PayloadUbah } from './asetForm'

/** Galat dari API dengan status HTTP. Pesan diambil dari respons bila ada, supaya pengguna melihat alasan yang sebenarnya. */
export class AsetApiError extends Error {
  readonly status: number
  constructor(status: number, message: string) {
    super(message)
    this.name = 'AsetApiError'
    this.status = status
  }
}

const PESAN_STATUS: Record<number, string> = {
  400: 'Isian tidak valid.',
  403: 'Peran Anda belum cukup untuk mengubah aset. Butuh editor atau admin.',
  404: 'Aset tidak ditemukan. Mungkin sudah dihapus.',
  422: 'Aturan data tidak terpenuhi. Periksa lokasi dan atribut wajib.',
}

async function galat(res: Response): Promise<AsetApiError> {
  const body = (await res.json().catch(() => ({}))) as { error?: string; message?: string }
  const pesan = body.error ?? body.message ?? PESAN_STATUS[res.status] ?? `Permintaan gagal (${res.status}).`
  return new AsetApiError(res.status, pesan)
}

async function kirim<T>(url: string, init: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...init,
    headers: init.body ? { 'content-type': 'application/json', ...init.headers } : init.headers,
  })
  if (!res.ok) throw await galat(res)
  const body = (await res.json()) as { data: T }
  return body.data
}

export type PenambahanAset = PayloadBuat
export type PerubahanAset = PayloadUbah

/** POST /api/v1/features. Respons 201 berisi fitur baru di `data`. */
export function buatAset(payload: PenambahanAset): Promise<PipeFeature> {
  return kirim<PipeFeature>('/api/v1/features', { method: 'POST', body: JSON.stringify(payload) })
}

/** PATCH /api/v1/features/{id} dengan hanya isian yang berubah. */
export function ubahAset(id: number, payload: PerubahanAset): Promise<PipeFeature> {
  return kirim<PipeFeature>(`/api/v1/features/${id}`, { method: 'PATCH', body: JSON.stringify(payload) })
}

/** Pemindahan memakai PATCH dengan geometri saja. */
export function pindahAset(id: number, geometry: Geometry): Promise<PipeFeature> {
  return ubahAset(id, { geometry })
}

/** DELETE /api/v1/features/{id}. Backend melakukan soft delete. Respons 204. */
export async function hapusAset(id: number): Promise<void> {
  const res = await fetch(`/api/v1/features/${id}`, { method: 'DELETE' })
  if (!res.ok) throw await galat(res)
}

export type EntriRiwayat = {
  aksi: string
  diubah_oleh: string
  diubah_pada: string
  sebelum: Record<string, unknown> | null
  sesudah: Record<string, unknown> | null
}

/** GET /api/v1/features/{id}/riwayat. */
export async function fetchRiwayat(id: number): Promise<EntriRiwayat[]> {
  const res = await fetch(`/api/v1/features/${id}/riwayat`)
  if (!res.ok) throw await galat(res)
  const body = (await res.json()) as { data: EntriRiwayat[] }
  return body.data
}

/** Respons ubah atau buat berisi fitur lengkap. Bila bentuknya tidak dikenali, pemanggil cukup memuat ulang peta. */
export function adalahFitur(v: unknown): v is PipeFeature {
  return typeof v === 'object' && v !== null && 'id' in v && 'properties' in v
}
