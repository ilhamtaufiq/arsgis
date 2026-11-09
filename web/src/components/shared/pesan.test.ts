import { describe, expect, it } from 'vitest'
import { PESAN_SUKSES, pesanSukses, type AksiSukses } from './pesan'

describe('pesanSukses', () => {
  it('memetakan aksi simpan pelanggan ke pesan spesifik', () => {
    expect(pesanSukses('pelanggan.simpan')).toBe('Pelanggan tersimpan')
  })

  it('jadwal perawatan memakai kata "dicatat" dan "ditandai selesai"', () => {
    expect(pesanSukses('jadwal.tambah')).toBe('Jadwal perawatan dicatat')
    expect(pesanSukses('jadwal.selesai')).toBe('Jadwal perawatan ditandai selesai')
  })

  it('hapus aset menyebut cara memulihkan', () => {
    expect(pesanSukses('aset.hapus')).toBe('Aset dihapus. Bisa dipulihkan dari daftar aset terhapus')
  })

  it('setiap aksi punya pesan tidak kosong dan tanpa titik di akhir', () => {
    for (const aksi of Object.keys(PESAN_SUKSES) as AksiSukses[]) {
      const teks = pesanSukses(aksi)
      expect(teks.trim().length, aksi).toBeGreaterThan(0)
      expect(teks, aksi).not.toMatch(/\.$/)
    }
  })

  it('semua aksi yang didaftarkan lengkap untuk halaman yang dipasang toast', () => {
    expect(Object.keys(PESAN_SUKSES).sort()).toEqual(
      [
        'aset.hapus',
        'aset.pindah',
        'aset.tambah',
        'aset.ubah',
        'dusun.simpan',
        'gangguan.buat',
        'gangguan.foto',
        'gangguan.status',
        'jadwal.selesai',
        'jadwal.tambah',
        'pelanggan.simpan',
        'perbaikan.catat',
        'rehab.simpan',
        'sr.pasang',
      ].sort(),
    )
  })
})
