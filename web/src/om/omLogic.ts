// Logika murni operasi dan pemeliharaan. Aturan di sini mencerminkan pipa-core (gangguan.rs, rehab.rs),
// supaya pengguna mendapat umpan balik sebelum mengirim. Server tetap memeriksa ulang semuanya.

import type { JenisGangguan, StatusGangguan, StatusJatuhTempo, StatusRehab, StatusRehabTampil } from './omApi'

export const LABEL_JENIS_GANGGUAN: Record<JenisGangguan, string> = {
  pipa_bocor: 'Pipa bocor',
  air_mati: 'Air mati',
  kualitas_keruh: 'Kualitas air keruh',
  lainnya: 'Lainnya',
}

export const LABEL_STATUS_GANGGUAN: Record<StatusGangguan, string> = {
  dilaporkan: 'Dilaporkan',
  ditangani: 'Ditangani',
  selesai: 'Selesai',
}

export const LABEL_STATUS_REHAB: Record<StatusRehabTampil, string> = {
  belum_ditandai: 'Belum ditandai',
  tidak_ada: 'Tidak perlu',
  perlu: 'Perlu rehabilitasi',
  prioritas: 'Prioritas',
  sedang_dikerjakan: 'Sedang dikerjakan',
}

export const LABEL_JENIS_PERAWATAN: Record<string, string> = {
  pembersihan_bak: 'Pembersihan bak',
  cek_valve: 'Cek valve',
  kalibrasi_meter: 'Kalibrasi meter',
  lainnya: 'Lainnya',
}

export const LABEL_KONDISI: Record<string, string> = {
  baik: 'Baik',
  rusak_ringan: 'Rusak ringan',
  rusak_berat: 'Rusak berat',
  tidak_berfungsi: 'Tidak berfungsi',
}

export const LABEL_JATUH_TEMPO: Record<StatusJatuhTempo, string> = {
  terlambat: 'Terlambat',
  hari_ini: 'Hari ini',
  segera: 'Segera',
  nanti: 'Nanti',
}

/** Batas karakter dan ukuran, sama dengan pipa-core. */
export const MAKS_DESKRIPSI = 2000
export const MAKS_PELAPOR = 100
export const MAKS_FOTO_BYTES = 5 * 1024 * 1024
export const MAKS_FOTO_PER_LAPORAN = 5
export const JENIS_FOTO_DITERIMA = ['image/jpeg', 'image/png'] as const

export const KONDISI_RUSAK = ['rusak_ringan', 'rusak_berat', 'tidak_berfungsi'] as const

/** Status laporan yang boleh dituju dari status sekarang. Status hanya maju. */
export function statusBerikut(status: StatusGangguan): StatusGangguan[] {
  switch (status) {
    case 'dilaporkan':
      return ['ditangani', 'selesai']
    case 'ditangani':
      return ['selesai']
    default:
      return []
  }
}

export type IsianLaporan = {
  jenis: string
  deskripsi: string
  pelapor: string
  featureId: number | null
  lon: number | null
  lat: number | null
}

/** Mengembalikan daftar galat. Daftar kosong berarti isian boleh dikirim. */
export function validasiLaporan(isian: IsianLaporan): string[] {
  const galat: string[] = []
  const deskripsi = isian.deskripsi.trim()
  if (!deskripsi) galat.push('Deskripsi wajib diisi.')
  else if (deskripsi.length > MAKS_DESKRIPSI) galat.push(`Deskripsi maksimal ${MAKS_DESKRIPSI} karakter.`)
  if (isian.pelapor.trim().length > MAKS_PELAPOR) galat.push(`Nama pelapor maksimal ${MAKS_PELAPOR} karakter.`)

  const adaTitik = isian.lon !== null || isian.lat !== null
  if (adaTitik && (isian.lon === null || isian.lat === null)) {
    galat.push('Lon dan lat harus diisi berpasangan.')
  } else if (adaTitik && !lokasiValid(isian.lon as number, isian.lat as number)) {
    galat.push('Koordinat di luar rentang (lon -180 sampai 180, lat -90 sampai 90).')
  }
  if (isian.featureId === null && !adaTitik) {
    galat.push('Lokasi wajib: isi titik koordinat atau ID aset.')
  }
  return galat
}

export function lokasiValid(lon: number, lat: number): boolean {
  return Number.isFinite(lon) && Number.isFinite(lat) && lon >= -180 && lon <= 180 && lat >= -90 && lat <= 90
}

/** Pemeriksaan awal foto di sisi klien. Tanda awal berkas tetap diperiksa server. */
export function validasiFoto(berkas: { size: number; type: string }, jumlahSekarang: number): string | null {
  if (berkas.size === 0) return 'Berkas foto kosong.'
  if (berkas.size > MAKS_FOTO_BYTES) return 'Ukuran foto maksimal 5 MB.'
  if (!(JENIS_FOTO_DITERIMA as readonly string[]).includes(berkas.type)) return 'Foto harus berformat JPEG atau PNG.'
  if (jumlahSekarang >= MAKS_FOTO_PER_LAPORAN) return `Laporan sudah punya ${MAKS_FOTO_PER_LAPORAN} foto.`
  return null
}

/** Aturan penandaan rehabilitasi, sama dengan `periksa_rehab` di pipa-core. Mengembalikan pesan galat atau null. */
export function validasiRehab(status: StatusRehab, prioritas: number | null, kondisi: string | null): string | null {
  if (prioritas !== null && (!Number.isInteger(prioritas) || prioritas < 1 || prioritas > 10)) {
    return 'Prioritas harus angka 1 sampai 10.'
  }
  if (status === 'tidak_ada') {
    return prioritas !== null ? 'Aset tanpa kebutuhan rehabilitasi tidak boleh punya angka prioritas.' : null
  }
  const rusak = KONDISI_RUSAK.includes((kondisi ?? '') as (typeof KONDISI_RUSAK)[number])
  if (!rusak) {
    return `Aset dengan kondisi ${kondisi ? (LABEL_KONDISI[kondisi] ?? kondisi).toLowerCase() : 'belum dicatat'} tidak bisa ditandai rehabilitasi. Ubah kondisi dulu atau pilih "Tidak perlu".`
  }
  if ((status === 'prioritas' || status === 'sedang_dikerjakan') && prioritas === null) {
    return `Status ${LABEL_STATUS_REHAB[status].toLowerCase()} wajib disertai angka prioritas.`
  }
  return null
}

/** Status rehabilitasi yang perlu dipilih untuk isian formulir (tanpa "belum ditandai"). */
export const PILIHAN_REHAB: StatusRehab[] = ['tidak_ada', 'perlu', 'prioritas', 'sedang_dikerjakan']

/** Bobot kondisi untuk tampilan, sama urutannya dengan `bobot_kondisi` di pipa-core. */
export function bobotKondisi(kondisi: string | null): number {
  switch (kondisi) {
    case 'tidak_berfungsi':
      return 3
    case 'rusak_berat':
      return 2
    case 'rusak_ringan':
      return 1
    default:
      return 0
  }
}

/** Teks jatuh tempo untuk pengguna, mis. "Terlambat 5 hari" atau "Jatuh tempo dalam 3 hari". */
export function teksJatuhTempo(status: StatusJatuhTempo, selisihHari: number): string {
  if (status === 'terlambat') return `Terlambat ${Math.abs(selisihHari)} hari`
  if (status === 'hari_ini') return 'Jatuh tempo hari ini'
  return `Jatuh tempo dalam ${selisihHari} hari`
}

const BULAN = ['Jan', 'Feb', 'Mar', 'Apr', 'Mei', 'Jun', 'Jul', 'Agu', 'Sep', 'Okt', 'Nov', 'Des']

/** `YYYY-MM-DD` menjadi "9 Nov 2026". Tanggal kalender diformat tanpa zona waktu supaya tidak bergeser. */
export function formatTanggal(teks: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(teks)
  if (!m) return teks
  const bulan = Number(m[2])
  return `${Number(m[3])} ${BULAN[bulan - 1] ?? ''} ${m[1]}`
}

export function formatRupiah(n: number | null): string {
  if (n === null) return '–'
  return `Rp ${n.toLocaleString('id-ID', { maximumFractionDigits: 0 })}`
}

/** Editor dan admin boleh mencatat dan memperbarui. Viewer dan kepala desa hanya membaca. */
export function bolehCatat(role: string | undefined): boolean {
  return role === 'editor' || role === 'admin'
}
