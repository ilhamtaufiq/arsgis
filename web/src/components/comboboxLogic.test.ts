import { describe, expect, it } from 'vitest'
import { filterOpsi, type OpsiCombo } from './comboboxLogic'

const DAFTAR: OpsiCombo[] = Array.from({ length: 25 }, (_, i) => ({
  value: String(i + 1),
  label: `Paket ${i + 1} ${i % 2 === 0 ? 'pipa transmisi' : 'sumur bor'}`,
  hint: `2026`,
}))

describe('filterOpsi', () => {
  it('tanpa kata kunci menampilkan 10 pertama dan total semua', () => {
    const hasil = filterOpsi(DAFTAR, '')
    expect(hasil.items).toHaveLength(10)
    expect(hasil.total).toBe(25)
    expect(hasil.items[0].value).toBe('1')
  })

  it('kata kunci harus cocok semua katanya, tanpa peduli huruf besar', () => {
    const hasil = filterOpsi(DAFTAR, 'SUMUR bor', 50)
    expect(hasil.items.length).toBeGreaterThan(0)
    expect(hasil.items.every((o) => o.label.toLowerCase().includes('sumur bor'))).toBe(true)
    expect(filterOpsi(DAFTAR, 'pipa sumur', 50).total).toBe(0)
  })

  it('mencocokkan petunjuk juga', () => {
    const hasil = filterOpsi(DAFTAR, '2026', 3)
    expect(hasil.items).toHaveLength(3)
    expect(hasil.total).toBe(25)
  })

  it('batas tampil bisa diubah', () => {
    expect(filterOpsi(DAFTAR, '', 4).items).toHaveLength(4)
  })
})
