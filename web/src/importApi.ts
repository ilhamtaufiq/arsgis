export type ImportReport = {
  /** Pemeriksaan batas desa unit. Null bila batch dibuat sebelum fitur ini ada. */
  dalam_batas?: number
  luar_batas?: number
  tanpa_batas?: number
  jumlah_fitur: number
  per_jenis: Record<string, number>
  dilewati: { nama: string | null; alasan: string }[]
  bukan_fitur: number
}

/** Batch yang sudah pernah diimpor dari berkas dengan isi sama. Null bila berkas baru. */
export type DuplikatImpor = {
  batch_id: number
  nama_file: string
  created_at: string
}

export type StagedImport = {
  id: number
  status: string
  nama_file: string
  laporan: ImportReport
  /** Opsional agar respons backend yang belum memuat field ini tetap bisa dibaca. */
  duplikat?: DuplikatImpor | null
}

export type StatusBatch = 'staged' | 'committed' | 'dibatalkan'

/** Satu baris daftar batch impor. `pekerjaan_nama` null bila impor tidak ditautkan ke paket. */
export type BatchImpor = {
  id: number
  nama_file: string
  status: StatusBatch
  created_at: string
  dibuat_oleh: string
  pekerjaan_nama: string | null
}

/** Galat dari API impor. `status` dipakai UI untuk memilih pesan yang tepat, misalnya 409 pada commit ganda. */
export class ImportApiError extends Error {
  readonly status: number
  constructor(status: number, message: string) {
    super(message)
    this.name = 'ImportApiError'
    this.status = status
  }
}

async function galat(res: Response, fallback: string): Promise<ImportApiError> {
  const body = (await res.json().catch(() => ({}))) as { error?: string }
  return new ImportApiError(res.status, body.error ?? `${fallback} (${res.status})`)
}

/** Unggah KML atau KMZ ke staging. Belum ada data yang masuk ke jaringan sampai `commitImport`. */
export async function uploadKml(file: File, titik: string, networkId: number, pekerjaanId?: number | null): Promise<StagedImport> {
  const params = new URLSearchParams({ nama_file: file.name, titik, network_id: String(networkId) })
  if (pekerjaanId != null) params.set('pekerjaan_id', String(pekerjaanId))
  const res = await fetch(`/api/v1/imports?${params}`, {
    method: 'POST',
    headers: { 'content-type': 'application/octet-stream' },
    body: file,
  })
  if (!res.ok) throw await galat(res, 'Impor gagal')
  return (await res.json()) as StagedImport
}

/**
 * Simpan batch ke jaringan. Bila berkas duplikat dan `izinkanGanda` false, backend menjawab 409.
 * Respons memakai `ditulis` dan `dilewati`. Nama lama `fitur_*` masih dibaca selama backend belum berubah.
 */
export async function commitImport(
  id: number,
  networkId: number,
  izinkanGanda: boolean,
): Promise<{ ditulis: number; dilewati: number }> {
  const params = new URLSearchParams({ network_id: String(networkId), izinkan_ganda: String(izinkanGanda) })
  const res = await fetch(`/api/v1/imports/${id}/commit?${params}`, { method: 'POST' })
  if (!res.ok) throw await galat(res, 'Simpan gagal')
  const body = (await res.json()) as { ditulis?: number; dilewati?: number; fitur_ditulis?: number; fitur_dilewati?: number }
  return { ditulis: body.ditulis ?? body.fitur_ditulis ?? 0, dilewati: body.dilewati ?? body.fitur_dilewati ?? 0 }
}

/** Batalkan batch: semua aset dari batch di-soft-delete dan bisa dipulihkan. Respons berisi jumlah aset yang dibatalkan. */
export async function batalkanImport(id: number): Promise<number> {
  const res = await fetch(`/api/v1/imports/${id}/batal`, { method: 'POST' })
  if (!res.ok) throw await galat(res, 'Pembatalan gagal')
  const body = (await res.json()) as { data: { dibatalkan: number } }
  return body.data.dibatalkan
}

/** Daftar batch impor untuk editor dan admin, terbaru di atas. */
export async function fetchImports(): Promise<BatchImpor[]> {
  const res = await fetch('/api/v1/imports')
  if (!res.ok) throw await galat(res, 'Daftar impor gagal dimuat')
  const body = (await res.json()) as { data: BatchImpor[] }
  return body.data
}
