import { describe, expect, it } from 'vitest'
import { ASET_BARU, ASET_COLOR, ASET_LABEL, isAsetBaru } from './asetStyle'

// Warna jenis yang sudah ada di peta. Aset baru tidak boleh memakai warna ini.
const WARNA_LAMA = ['#1d4ed8', '#111111', '#0f766e', '#a16207', '#0891b2']

describe('asetStyle', () => {
  it('setiap jenis aset baru punya label dan warna', () => {
    for (const k of ASET_BARU) {
      expect(ASET_LABEL[k]).toBeTruthy()
      expect(ASET_COLOR[k]).toMatch(/^#[0-9a-f]{6}$/)
    }
    expect(ASET_BARU).toHaveLength(6)
  })

  it('warna aset baru berbeda dari warna jenis lama dan satu sama lain', () => {
    const warna = ASET_BARU.map((k) => ASET_COLOR[k])
    expect(new Set(warna).size).toBe(warna.length)
    for (const c of warna) expect(WARNA_LAMA).not.toContain(c)
  })

  it('isAsetBaru hanya mengenali jenis baru', () => {
    expect(isAsetBaru('menara_air')).toBe(true)
    expect(isAsetBaru('meter_induk')).toBe(true)
    expect(isAsetBaru('pipa')).toBe(false)
    expect(isAsetBaru('sr')).toBe(false)
  })
})
