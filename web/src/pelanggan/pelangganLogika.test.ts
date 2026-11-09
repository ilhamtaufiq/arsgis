import { describe, expect, it } from 'vitest'
import { fieldBerubah, kePayload, validasiFormPelanggan, type FormPelanggan } from './pelangganLogika'

const dasar: FormPelanggan = {
  nama: 'Siti Aminah',
  alamat: '',
  nomorMeter: 'MTR-0042/A',
  golongan: 'rumah_tangga',
  status: 'aktif',
  srId: 17,
}

describe('validasiFormPelanggan', () => {
  it('menerima isian normal', () => {
    expect(validasiFormPelanggan(dasar)).toBeNull()
  })

  it('menolak nama terlalu pendek dan SR kosong', () => {
    expect(validasiFormPelanggan({ ...dasar, nama: ' A ' })).not.toBeNull()
    expect(validasiFormPelanggan({ ...dasar, srId: null })).toMatch(/titik SR/)
  })

  it('menolak golongan di luar daftar dan nomor meter dengan spasi', () => {
    expect(validasiFormPelanggan({ ...dasar, golongan: 'Rumah Tangga' })).toMatch(/golongan/)
    expect(validasiFormPelanggan({ ...dasar, nomorMeter: '12 34' })).toMatch(/Nomor meter/)
  })

  it('nomor meter kosong diperbolehkan', () => {
    expect(validasiFormPelanggan({ ...dasar, nomorMeter: '   ' })).toBeNull()
  })
})

describe('kePayload', () => {
  it('merapikan teks kosong menjadi null', () => {
    const p = kePayload({ ...dasar, nama: '  Budi  ', alamat: '  ', nomorMeter: '' })
    expect(p).toEqual({
      nama: 'Budi',
      alamat: null,
      nomor_meter: null,
      golongan_tarif: 'rumah_tangga',
      status: 'aktif',
      sr_feature_id: 17,
    })
  })
})

describe('fieldBerubah', () => {
  it('hanya menyebut field yang benar-benar berubah', () => {
    const item = {
      aksi: 'ubah' as const,
      diubah_oleh: 'op1',
      diubah_pada: '2026-11-09T10:00:00+07:00',
      sebelum: { nama: 'Siti', status: 'aktif', alamat: null },
      sesudah: { nama: 'Siti', status: 'putus', alamat: null },
    }
    expect(fieldBerubah(item)).toEqual(['status'])
  })

  it('riwayat tambah tidak punya selisih', () => {
    expect(
      fieldBerubah({ aksi: 'tambah', diubah_oleh: 'op1', diubah_pada: 'x', sebelum: null, sesudah: { nama: 'A' } }),
    ).toEqual([])
  })
})
