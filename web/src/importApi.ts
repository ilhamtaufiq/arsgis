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

export type StagedImport = {
  id: number
  status: string
  nama_file: string
  laporan: ImportReport
}

async function errorMessage(res: Response, fallback: string): Promise<string> {
  const body = (await res.json().catch(() => ({}))) as { error?: string }
  return body.error ?? `${fallback} (${res.status})`
}

/** Unggah KML ke staging. Belum ada data yang masuk ke jaringan sampai `commitImport`. */
export async function uploadKml(file: File, titik: string, networkId: number): Promise<StagedImport> {
  const params = new URLSearchParams({ nama_file: file.name, titik, network_id: String(networkId) })
  const res = await fetch(`/api/v1/imports?${params}`, {
    method: 'POST',
    headers: { 'content-type': 'application/octet-stream' },
    body: file,
  })
  if (!res.ok) throw new Error(await errorMessage(res, 'Impor gagal'))
  return (await res.json()) as StagedImport
}

export async function commitImport(id: number, networkId: number): Promise<{ ditulis: number; dilewati: number }> {
  const params = new URLSearchParams({ network_id: String(networkId) })
  const res = await fetch(`/api/v1/imports/${id}/commit?${params}`, { method: 'POST' })
  if (!res.ok) throw new Error(await errorMessage(res, 'Simpan gagal'))
  const body = (await res.json()) as { fitur_ditulis: number; fitur_dilewati: number }
  return { ditulis: body.fitur_ditulis, dilewati: body.fitur_dilewati }
}
