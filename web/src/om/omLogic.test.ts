import { describe, expect, it } from 'vitest'
import {
  bolehCatat,
  formatTanggal,
  MAKS_FOTO_BYTES,
  statusBerikut,
  teksJatuhTempo,
  validasiFoto,
  validasiLaporan,
  validasiRehab,
  type IsianLaporan,
} from './omLogic'

const laporanDasar: IsianLaporan = {
  jenis: 'pipa_bocor',
  deskripsi: 'Air memancar dari sambungan',
  pelapor: 'Siti',
  featureId: null,
  lon: 107.1,
  lat: -6.8,
}

describe('transisi status laporan', () => {
  it('hanya menawarkan status yang lebih maju', () => {
    expect(statusBerikut('dilaporkan')).toEqual(['ditangani', 'selesai'])
    expect(statusBerikut('ditangani')).toEqual(['selesai'])
    expect(statusBerikut('selesai')).toEqual([])
  })
})

describe('validasi laporan gangguan', () => {
  it('menerima laporan dengan titik koordinat', () => {
    expect(validasiLaporan(laporanDasar)).toEqual([])
  })

  it('menerima laporan dengan aset saja', () => {
    expect(validasiLaporan({ ...laporanDasar, lon: null, lat: null, featureId: 42 })).toEqual([])
  })

  it('menolak deskripsi kosong dan terlalu panjang', () => {
    expect(validasiLaporan({ ...laporanDasar, deskripsi: '   ' })).toContain('Deskripsi wajib diisi.')
    const panjang = validasiLaporan({ ...laporanDasar, deskripsi: 'x'.repeat(2001) })
    expect(panjang.some((g) => g.includes('maksimal 2000'))).toBe(true)
  })

  it('menolak lokasi kosong dan koordinat yang hanya separuh', () => {
    const tanpaLokasi = validasiLaporan({ ...laporanDasar, lon: null, lat: null })
    expect(tanpaLokasi.some((g) => g.startsWith('Lokasi wajib'))).toBe(true)
    const separuh = validasiLaporan({ ...laporanDasar, lat: null })
    expect(separuh).toContain('Lon dan lat harus diisi berpasangan.')
  })

  it('menolak koordinat di luar rentang', () => {
    const galat = validasiLaporan({ ...laporanDasar, lon: 200 })
    expect(galat.some((g) => g.startsWith('Koordinat di luar rentang'))).toBe(true)
  })
})

describe('validasi foto', () => {
  it('menerima JPEG dan PNG dalam batas', () => {
    expect(validasiFoto({ size: 1000, type: 'image/jpeg' }, 0)).toBeNull()
    expect(validasiFoto({ size: 1000, type: 'image/png' }, 4)).toBeNull()
  })

  it('menolak tipe lain, berkas kosong, dan ukuran di atas 5 MB', () => {
    expect(validasiFoto({ size: 1000, type: 'image/gif' }, 0)).toMatch(/JPEG atau PNG/)
    expect(validasiFoto({ size: 0, type: 'image/png' }, 0)).toMatch(/kosong/)
    expect(validasiFoto({ size: MAKS_FOTO_BYTES + 1, type: 'image/png' }, 0)).toMatch(/5 MB/)
  })

  it('menolak foto ke-6 pada satu laporan', () => {
    expect(validasiFoto({ size: 1000, type: 'image/png' }, 5)).toMatch(/5 foto/)
  })
})

describe('aturan penandaan rehabilitasi', () => {
  it('kondisi baik atau belum dicatat tidak boleh ditandai', () => {
    expect(validasiRehab('perlu', null, 'baik')).not.toBeNull()
    expect(validasiRehab('perlu', null, null)).not.toBeNull()
    expect(validasiRehab('perlu', null, 'perlu cek')).not.toBeNull()
  })

  it('kondisi rusak boleh ditandai', () => {
    expect(validasiRehab('perlu', null, 'rusak_ringan')).toBeNull()
    expect(validasiRehab('prioritas', 1, 'tidak_berfungsi')).toBeNull()
  })

  it('prioritas dan sedang dikerjakan wajib angka prioritas', () => {
    expect(validasiRehab('prioritas', null, 'rusak_berat')).toMatch(/wajib/)
    expect(validasiRehab('sedang_dikerjakan', null, 'rusak_berat')).toMatch(/wajib/)
    expect(validasiRehab('perlu', null, 'rusak_berat')).toBeNull()
  })

  it('angka prioritas dibatasi 1 sampai 10', () => {
    expect(validasiRehab('prioritas', 0, 'rusak_berat')).not.toBeNull()
    expect(validasiRehab('prioritas', 11, 'rusak_berat')).not.toBeNull()
    expect(validasiRehab('prioritas', 10, 'rusak_berat')).toBeNull()
  })

  it('tidak_ada tidak boleh membawa angka prioritas', () => {
    expect(validasiRehab('tidak_ada', 2, 'rusak_berat')).not.toBeNull()
    expect(validasiRehab('tidak_ada', null, 'baik')).toBeNull()
  })
})

describe('tampilan jatuh tempo dan tanggal', () => {
  it('menuliskan selisih hari dengan benar', () => {
    expect(teksJatuhTempo('terlambat', -5)).toBe('Terlambat 5 hari')
    expect(teksJatuhTempo('hari_ini', 0)).toBe('Jatuh tempo hari ini')
    expect(teksJatuhTempo('segera', 3)).toBe('Jatuh tempo dalam 3 hari')
  })

  it('memformat tanggal kalender tanpa menggeser hari', () => {
    expect(formatTanggal('2026-11-09')).toBe('9 Nov 2026')
    expect(formatTanggal('2026-01-01')).toBe('1 Jan 2026')
    expect(formatTanggal('bukan-tanggal')).toBe('bukan-tanggal')
  })
})

describe('hak tulis O&M', () => {
  it('hanya editor dan admin yang boleh mencatat', () => {
    expect(bolehCatat('editor')).toBe(true)
    expect(bolehCatat('admin')).toBe(true)
    expect(bolehCatat('viewer')).toBe(false)
    expect(bolehCatat(undefined)).toBe(false)
  })
})
