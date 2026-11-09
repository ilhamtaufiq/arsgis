import { describe, expect, it } from 'vitest'
import type { FeatureProperties, FeatureType } from './types'
import {
  ATRIBUT_KAPASITAS,
  JENIS_SEMUA,
  LABEL_JENIS,
  NILAI_KOSONG,
  bacaAngka,
  buatPayload,
  fieldUntuk,
  geometriUntuk,
  nilaiDari,
  pindahPayload,
  selisihAtribut,
  ubahPayload,
  validasiAset,
  wajibUntuk,
  type NilaiForm,
} from './asetForm'

const TAHUN = 2026
const PIPA_VALID: NilaiForm = { ...NILAI_KOSONG, diameter_mm: '110', material: 'HDPE' }
const TITIK_VALID = (jenis: FeatureType): NilaiForm => ({
  ...NILAI_KOSONG,
  nama: 'Tandon A',
  ...(ATRIBUT_KAPASITAS[jenis] ? { [ATRIBUT_KAPASITAS[jenis]!]: '10' } : {}),
})

describe('aturan jenis', () => {
  it('ada tiga belas jenis dan pipa satu-satunya garis', () => {
    expect(JENIS_SEMUA).toHaveLength(13)
    const garis = JENIS_SEMUA.filter((j) => geometriUntuk(j) === 'LineString')
    expect(garis).toEqual(['pipa'])
  })

  it('tapping adalah titik, wajib nama, dan tidak punya kapasitas atau debit', () => {
    expect(JENIS_SEMUA).toContain('tapping')
    expect(geometriUntuk('tapping')).toBe('Point')
    expect(wajibUntuk('tapping')).toEqual(['nama'])
    expect(ATRIBUT_KAPASITAS.tapping).toBeUndefined()
    expect(validasiAset('tapping', { ...NILAI_KOSONG }, TAHUN).nama).toBeTruthy()
    expect(validasiAset('tapping', TITIK_VALID('tapping'), TAHUN)).toEqual({})
  })

  it('mata air adalah titik, wajib nama, dan tidak punya kapasitas atau debit', () => {
    expect(JENIS_SEMUA).toContain('mata_air')
    expect(LABEL_JENIS.mata_air).toBe('Mata air')
    expect(geometriUntuk('mata_air')).toBe('Point')
    expect(wajibUntuk('mata_air')).toEqual(['nama'])
    expect(ATRIBUT_KAPASITAS.mata_air).toBeUndefined()
    expect(validasiAset('mata_air', { ...NILAI_KOSONG }, TAHUN).nama).toBeTruthy()
    expect(validasiAset('mata_air', TITIK_VALID('mata_air'), TAHUN)).toEqual({})
  })

  it('pipa wajib diameter dan material, bukan nama', () => {
    expect(wajibUntuk('pipa')).toEqual(['diameter_mm', 'material'])
    expect(fieldUntuk('pipa')).not.toContain('kapasitas_m3')
  })

  it('reservoir dan menara air wajib kapasitas, intake dan pompa wajib debit', () => {
    expect(wajibUntuk('reservoir')).toEqual(['nama', 'kapasitas_m3'])
    expect(wajibUntuk('menara_air')).toEqual(['nama', 'kapasitas_m3'])
    expect(wajibUntuk('intake')).toEqual(['nama', 'debit_lps'])
    expect(wajibUntuk('pompa')).toEqual(['nama', 'debit_lps'])
  })

  it('valve, sr, dan hidran hanya wajib nama', () => {
    for (const j of ['valve', 'sr', 'hidran_umum'] as const) {
      expect(wajibUntuk(j)).toEqual(['nama'])
    }
  })
})

describe('validasiAset', () => {
  it('pipa tanpa diameter dan material ditolak', () => {
    const galat = validasiAset('pipa', NILAI_KOSONG, TAHUN)
    expect(galat.diameter_mm).toBeTruthy()
    expect(galat.material).toBeTruthy()
    expect(galat.nama).toBeUndefined()
  })

  it('pipa dengan diameter dan material valid lolos', () => {
    expect(validasiAset('pipa', PIPA_VALID, TAHUN)).toEqual({})
  })

  it('diameter harus bilangan bulat positif', () => {
    expect(validasiAset('pipa', { ...PIPA_VALID, diameter_mm: '110,5' }, TAHUN).diameter_mm).toBeTruthy()
    expect(validasiAset('pipa', { ...PIPA_VALID, diameter_mm: '0' }, TAHUN).diameter_mm).toBeTruthy()
    expect(validasiAset('pipa', { ...PIPA_VALID, diameter_mm: 'abc' }, TAHUN).diameter_mm).toBeTruthy()
    expect(validasiAset('pipa', { ...PIPA_VALID, diameter_mm: '3001' }, TAHUN).diameter_mm).toBeTruthy()
  })

  it('material di luar daftar ditolak', () => {
    expect(validasiAset('pipa', { ...PIPA_VALID, material: 'Kayu' }, TAHUN).material).toBeTruthy()
  })

  it('aset lain tanpa nama ditolak', () => {
    const galat = validasiAset('valve', NILAI_KOSONG, TAHUN)
    expect(galat.nama).toBeTruthy()
  })

  it('reservoir tanpa kapasitas ditolak, dengan kapasitas lolos', () => {
    expect(validasiAset('reservoir', { ...NILAI_KOSONG, nama: 'R1' }, TAHUN).kapasitas_m3).toBeTruthy()
    expect(validasiAset('reservoir', TITIK_VALID('reservoir'), TAHUN)).toEqual({})
  })

  it('kapasitas dan debit tidak boleh negatif', () => {
    expect(validasiAset('reservoir', { ...TITIK_VALID('reservoir'), kapasitas_m3: '-5' }, TAHUN).kapasitas_m3).toBeTruthy()
    expect(validasiAset('pompa', { ...TITIK_VALID('pompa'), debit_lps: '-0,1' }, TAHUN).debit_lps).toBeTruthy()
  })

  it('angka dengan koma desimal diterima', () => {
    expect(validasiAset('reservoir', { ...TITIK_VALID('reservoir'), kapasitas_m3: '12,5' }, TAHUN)).toEqual({})
  })

  it('tahun di luar rentang ditolak, termasuk tahun depan', () => {
    expect(validasiAset('valve', { ...TITIK_VALID('valve'), tahun_pasang: '1899' }, TAHUN).tahun_pasang).toBeTruthy()
    expect(validasiAset('valve', { ...TITIK_VALID('valve'), tahun_pasang: '2027' }, TAHUN).tahun_pasang).toBeTruthy()
    expect(validasiAset('valve', { ...TITIK_VALID('valve'), tahun_bangun: '2026' }, TAHUN)).toEqual({})
  })

  it('elevasi boleh negatif, tetapi tetap harus angka', () => {
    expect(validasiAset('intake', { ...TITIK_VALID('intake'), elevasi_m: '-12,3' }, TAHUN)).toEqual({})
    expect(validasiAset('intake', { ...TITIK_VALID('intake'), elevasi_m: 'tinggi' }, TAHUN).elevasi_m).toBeTruthy()
  })

  it('batas panjang nama dan sumber dana', () => {
    expect(validasiAset('valve', { ...TITIK_VALID('valve'), nama: 'x'.repeat(101) }, TAHUN).nama).toBeTruthy()
    expect(validasiAset('valve', { ...TITIK_VALID('valve'), sumber_dana: 'x'.repeat(101) }, TAHUN).sumber_dana).toBeTruthy()
  })
})

describe('bacaAngka', () => {
  it('kosong menjadi null, bukan galat', () => {
    expect(bacaAngka('   ')).toEqual({ ok: true, nilai: null })
  })
  it('menerima koma sebagai desimal', () => {
    expect(bacaAngka('3,25')).toEqual({ ok: true, nilai: 3.25 })
  })
  it('menolak teks campuran', () => {
    expect(bacaAngka('12 m')).toEqual({ ok: false })
  })
})

describe('buatPayload', () => {
  it('pipa: garis, diameter sebagai angka, isian kosong tidak ikut', () => {
    const body = buatPayload(
      'pipa',
      { type: 'LineString', coordinates: [[107.1, -6.8], [107.2, -6.8]] },
      { ...PIPA_VALID, nama: '  Jalur utama ', tahun_pasang: '2015' },
      7,
    )
    expect(body).toEqual({
      network_id: 7,
      feature_type: 'pipa',
      geometry: { type: 'LineString', coordinates: [[107.1, -6.8], [107.2, -6.8]] },
      nama: 'Jalur utama',
      diameter_mm: 110,
      material: 'HDPE',
      tahun_pasang: 2015,
    })
  })

  it('titik: hanya atribut yang tampil untuk jenis itu yang dikirim', () => {
    const body = buatPayload(
      'reservoir',
      { type: 'Point', coordinates: [107.14, -6.82] },
      { ...TITIK_VALID('reservoir'), diameter_mm: '200', debit_lps: '5', elevasi_m: '' },
      3,
    )
    expect(body).toEqual({
      network_id: 3,
      feature_type: 'reservoir',
      geometry: { type: 'Point', coordinates: [107.14, -6.82] },
      nama: 'Tandon A',
      kapasitas_m3: 10,
    })
  })

  it('elevasi negatif dan koma desimal dikonversi ke angka', () => {
    const body = buatPayload('intake', { type: 'Point', coordinates: [0, 0] }, { ...TITIK_VALID('intake'), debit_lps: '2,5', elevasi_m: '-3,5' }, 1)
    expect(body.debit_lps).toBe(2.5)
    expect(body.elevasi_m).toBe(-3.5)
  })
})

describe('ubahPayload', () => {
  const awal = nilaiDari({
    network_id: 1,
    feature_type: 'reservoir',
    nama: 'Tandon A',
    diameter_mm: null,
    material: null,
    tahun_pasang: 2010,
    kondisi: 'baik',
    status: 'aktif',
    fungsi: null,
    kapasitas_m3: 10,
    debit_lps: null,
    elevasi_m: null,
    tahun_bangun: null,
    sumber_dana: null,
  } as FeatureProperties)

  it('hanya isian yang berubah yang dikirim', () => {
    const akhir = { ...awal, nama: 'Tandon B' }
    expect(ubahPayload('reservoir', awal, akhir)).toEqual({ nama: 'Tandon B' })
  })

  it('isian yang dikosongkan dikirim sebagai null', () => {
    const akhir = { ...awal, sumber_dana: '', tahun_pasang: '' }
    expect(ubahPayload('reservoir', awal, akhir)).toEqual({ tahun_pasang: null })
  })

  it('tanpa perubahan menghasilkan objek kosong', () => {
    expect(ubahPayload('reservoir', awal, { ...awal })).toEqual({})
  })

  it('perbedaan penulisan angka tidak dihitung sebagai perubahan', () => {
    const akhir = { ...awal, kapasitas_m3: '10,0' }
    expect(ubahPayload('reservoir', awal, akhir)).toEqual({})
  })

  it('isian yang tidak tampil untuk jenis itu diabaikan', () => {
    const akhir = { ...awal, diameter_mm: '50' }
    expect(ubahPayload('reservoir', awal, akhir)).toEqual({})
  })
})

describe('pindahPayload', () => {
  it('hanya membawa geometri', () => {
    expect(pindahPayload({ type: 'Point', coordinates: [1, 2] })).toEqual({ geometry: { type: 'Point', coordinates: [1, 2] } })
  })
})

describe('selisihAtribut', () => {
  it('menampilkan kolom yang berbeda dengan label Indonesia', () => {
    const hasil = selisihAtribut({ nama: 'A', kondisi: 'baik', elevasi_m: null }, { nama: 'A', kondisi: 'rusak_ringan', elevasi_m: 12 })
    expect(hasil).toEqual([
      { kolom: 'Kondisi', dari: 'baik', ke: 'rusak_ringan' },
      { kolom: 'Elevasi (m)', dari: '–', ke: '12' },
    ])
  })

  it('geometri ditampilkan sebagai lokasi, dan keadaan kosong menghasilkan daftar kosong', () => {
    const hasil = selisihAtribut({ geometry: { type: 'Point', coordinates: [0, 0] } }, { geometry: { type: 'Point', coordinates: [1, 1] } })
    expect(hasil).toEqual([{ kolom: 'Lokasi', dari: 'lokasi', ke: 'lokasi' }])
    expect(selisihAtribut(null, null)).toEqual([])
  })
})
