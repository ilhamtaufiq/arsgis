/**
 * Pesan sukses singkat untuk toast, satu per aksi. Teks berbahasa Indonesia dan spesifik pada objek yang diubah.
 * Pesan gagal tidak lewat sini: memakai pesan error dari server atau dari lapisan API.
 */
export const PESAN_SUKSES = {
  'pelanggan.simpan': 'Pelanggan tersimpan',
  'dusun.simpan': 'Dusun tersimpan',
  'sr.pasang': 'Titik SR dipasang ke dusun',
  'gangguan.buat': 'Laporan gangguan tersimpan',
  'gangguan.status': 'Status gangguan diubah',
  'gangguan.foto': 'Foto ditambahkan ke laporan',
  'rehab.simpan': 'Prioritas rehabilitasi tersimpan',
  'perbaikan.catat': 'Perbaikan dicatat',
  'jadwal.tambah': 'Jadwal perawatan dicatat',
  'jadwal.selesai': 'Jadwal perawatan ditandai selesai',
  'aset.tambah': 'Aset ditambahkan',
  'aset.ubah': 'Perubahan aset tersimpan',
  'aset.pindah': 'Posisi aset dipindah',
  'aset.hapus': 'Aset dihapus. Bisa dipulihkan dari daftar aset terhapus',
} as const

export type AksiSukses = keyof typeof PESAN_SUKSES

export function pesanSukses(aksi: AksiSukses): string {
  return PESAN_SUKSES[aksi]
}
