/** Paket pekerjaan (kontrak API: GET /api/v1/pekerjaan). */
export type PaketPekerjaan = { id: number; nama_paket: string; tahun: number }

export async function fetchPekerjaan(): Promise<PaketPekerjaan[]> {
  const res = await fetch('/api/v1/pekerjaan')
  if (!res.ok) throw new Error(`Daftar paket pekerjaan gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: PaketPekerjaan[] }
  return body.data
}
