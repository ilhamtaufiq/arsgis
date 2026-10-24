export type DashboardData = {
  jaringan: string
  bisa_melihat_sr: boolean
  fitur: { feature_type: string; jumlah: number }[]
  pipa: { fungsi: string; jumlah: number; km: number }[]
  impor_terakhir: { id: number; nama_file: string; status: string; dibuat_oleh: string; jumlah_fitur: number | null; waktu: string }[]
}

export async function fetchDashboard(networkId: number): Promise<DashboardData> {
  const res = await fetch(`/api/v1/networks/${networkId}/dashboard`)
  if (!res.ok) throw new Error(`Dashboard gagal dimuat (${res.status})`)
  return (await res.json()) as DashboardData
}
