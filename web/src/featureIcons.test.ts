import { describe, expect, it } from 'vitest'
import { FEATURE_META, ikonHtml, warnaFitur } from './featureIcons'
import type { FeatureType } from './types'

const SEMUA: FeatureType[] = [
  'pipa', 'valve', 'reservoir', 'sr', 'bronkaptering', 'bak_penahan_tekanan',
  'menara_air', 'hidran_umum', 'intake', 'pompa', 'meter_induk', 'tapping',
]

describe('ikon jenis fitur', () => {
  it('setiap jenis punya label, warna, dan isi ikon', () => {
    for (const t of SEMUA) {
      expect(FEATURE_META[t].label.length).toBeGreaterThan(0)
      expect(FEATURE_META[t].color).toMatch(/^#[0-9a-f]{6}$/i)
      expect(FEATURE_META[t].icon.length).toBeGreaterThan(0)
    }
  })

  it('sambungan rumah berwarna hijau bila terlayani dan kuning bila belum', () => {
    expect(warnaFitur('sr', true)).toBe('#15803d')
    expect(warnaFitur('sr', false)).toBe('#ca8a04')
    expect(warnaFitur('sr', null)).toBe(FEATURE_META.sr.color)
  })

  it('tapping punya ikon dan warna sendiri, bukan warna sambungan rumah', () => {
    expect(FEATURE_META.tapping.label).toBe('Tapping pipa')
    expect(FEATURE_META.tapping.color).toMatch(/^#[0-9a-f]{6}$/i)
    expect(FEATURE_META.tapping.color).not.toBe(FEATURE_META.sr.color)
    expect(FEATURE_META.tapping.color).not.toBe(FEATURE_META.reservoir.color)
    expect(FEATURE_META.tapping.icon).toContain('<circle')
    // Tapping tidak ikut status cakupan karena bukan sambungan rumah.
    expect(warnaFitur('tapping', false)).toBe(FEATURE_META.tapping.color)
    expect(ikonHtml('tapping', null, 20)).toContain(`background:${FEATURE_META.tapping.color}`)
  })

  it('jenis lain tidak terpengaruh status cakupan', () => {
    expect(warnaFitur('valve', true)).toBe(FEATURE_META.valve.color)
  })

  it('markup ikon memuat svg dengan ukuran yang diminta', () => {
    const html = ikonHtml('sr', true, 26)
    expect(html).toContain('<svg')
    expect(html).toContain('width:26px')
    expect(html).toContain('background:#15803d')
    expect(html).toContain('aria-hidden="true"')
  })
})
