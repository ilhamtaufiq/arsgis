import { describe, expect, it } from 'vitest'
import { ImportApiError, type BatchImpor } from './importApi'
import {
  bisaDibatalkan,
  daftarBisaDibatalkan,
  formatWaktu,
  labelStatusBatch,
  pesanBatalSukses,
  pesanDuplikat,
  pesanGalatBatal,
  pesanGalatSimpan,
  pesanKonfirmasiBatal,
} from './importLogic'

const ISO = '2026-03-14T08:30:00Z'

describe('pesan duplikat', () => {
  it('menyebut nama berkas dan tanggal dalam format Indonesia', () => {
    const pesan = pesanDuplikat({ batch_id: 7, nama_file: 'sr-desa-a.kml', created_at: ISO })
    expect(pesan.startsWith('Berkas ini sudah pernah diimpor: sr-desa-a.kml pada ')).toBe(true)
    expect(pesan.endsWith(formatWaktu(ISO))).toBe(true)
  })

  it('tanggal yang tidak valid dikembalikan apa adanya', () => {
    expect(formatWaktu('bukan-tanggal')).toBe('bukan-tanggal')
  })
})

describe('label status batch', () => {
  it('memetakan tiga status ke label Indonesia', () => {
    expect(labelStatusBatch('staged')).toBe('Menunggu disimpan')
    expect(labelStatusBatch('committed')).toBe('Disimpan')
    expect(labelStatusBatch('dibatalkan')).toBe('Dibatalkan')
  })

  it('status yang tidak dikenal ditampilkan apa adanya', () => {
    expect(labelStatusBatch('entah')).toBe('entah')
  })
})

describe('penyaringan batch yang bisa dibatalkan', () => {
  const batch = (id: number, status: BatchImpor['status']): BatchImpor => ({
    id,
    nama_file: `f${id}.kml`,
    status,
    created_at: ISO,
    dibuat_oleh: 'op',
    pekerjaan_nama: null,
  })

  it('batch yang sudah dibatalkan tidak bisa dibatalkan lagi, status lain bisa', () => {
    expect(bisaDibatalkan({ status: 'staged' })).toBe(true)
    expect(bisaDibatalkan({ status: 'committed' })).toBe(true)
    expect(bisaDibatalkan({ status: 'dibatalkan' })).toBe(false)
  })

  it('daftarBisaDibatalkan hanya menyisakan batch yang belum dibatalkan dan mempertahankan urutan', () => {
    const hasil = daftarBisaDibatalkan([batch(3, 'committed'), batch(2, 'dibatalkan'), batch(1, 'staged')])
    expect(hasil.map((b) => b.id)).toEqual([3, 1])
  })
})

describe('pesan konfirmasi dan hasil pembatalan', () => {
  it('konfirmasi menyebut bahwa aset dihapus dan bisa dipulihkan', () => {
    const pesan = pesanKonfirmasiBatal({ nama_file: 'desa.kml' })
    expect(pesan).toContain('desa.kml')
    expect(pesan).toContain('Semua aset dari impor ini akan dihapus')
    expect(pesan).toContain('Aset terhapus')
  })

  it('pesan sukses menyebut jumlah aset', () => {
    expect(pesanBatalSukses(12)).toBe('Impor dibatalkan. 12 aset dihapus dan bisa dipulihkan dari daftar Aset terhapus.')
  })
})

describe('pesan galat', () => {
  it('409 saat simpan dijelaskan sebagai berkas ganda, dengan petunjuk checkbox', () => {
    const pesan = pesanGalatSimpan(new ImportApiError(409, 'duplikat'))
    expect(pesan).toContain('sudah pernah diimpor')
    expect(pesan).toContain('Impor lagi walaupun ganda')
  })

  it('galat lain memakai pesan dari server', () => {
    expect(pesanGalatSimpan(new ImportApiError(500, 'basis data mati'))).toBe('basis data mati')
    expect(pesanGalatSimpan('aneh')).toBe('Simpan gagal.')
  })

  it('409 saat batal berarti sudah dibatalkan sebelumnya', () => {
    expect(pesanGalatBatal(new ImportApiError(409, 'x'))).toBe('Impor ini sudah dibatalkan sebelumnya.')
  })
})
