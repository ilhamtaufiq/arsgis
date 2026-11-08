import { describe, expect, it } from 'vitest'
import {
  chipLayers,
  filterLayers,
  isNew,
  kindBadge,
  todayIso,
  toCatalogLayer,
  type CatalogLayer,
  type CatalogRow,
} from './catalogApi'
import { exportFileName } from './fileName'

const row = (over: Partial<CatalogRow> = {}): CatalogRow => ({
  slug: 'cakupan-spm',
  nama: 'Cakupan SPM per desa',
  kategori: 'SPM air minum',
  sumber: 'Arumanis (sinkron)',
  kind: 'internal',
  url: null,
  params: { internalKey: 'cakupan-spm' },
  min_role: 'anon',
  chip: false,
  chip_urut: null,
  baru_sampai: null,
  ...over,
})

const layer = (over: Partial<CatalogLayer> = {}): CatalogLayer => ({ ...toCatalogLayer(row()), ...over })

describe('toCatalogLayer', () => {
  it('mengambil kunci internal dari params', () => {
    expect(toCatalogLayer(row()).internalKey).toBe('cakupan-spm')
  })
  it('mengabaikan kunci yang tidak dikenal', () => {
    expect(toCatalogLayer(row({ params: { internalKey: 'lain' } })).internalKey).toBeUndefined()
  })
  it('tahan terhadap params kosong', () => {
    expect(toCatalogLayer(row({ params: null })).internalKey).toBeUndefined()
  })
  it('membawa min_role, chip, dan tanggal BARU', () => {
    const l = toCatalogLayer(row({ min_role: 'editor', chip: true, chip_urut: 2, baru_sampai: '2026-12-31' }))
    expect([l.min_role, l.chip, l.chip_urut, l.baru_sampai]).toEqual(['editor', true, 2, '2026-12-31'])
  })
})

describe('chip dan tag BARU', () => {
  it('chipLayers hanya mengambil layer chip, urut menurut chip_urut', () => {
    const list = [
      layer({ slug: 'c', chip: true, chip_urut: 3 }),
      layer({ slug: 'x', chip: false }),
      layer({ slug: 'a', chip: true, chip_urut: 1 }),
    ]
    expect(chipLayers(list).map((l) => l.slug)).toEqual(['a', 'c'])
  })
  it('tag BARU tampil sampai tanggalnya, termasuk hari itu', () => {
    expect(isNew({ baru_sampai: '2026-11-08' }, '2026-11-08')).toBe(true)
    expect(isNew({ baru_sampai: '2026-11-09' }, '2026-11-08')).toBe(true)
    expect(isNew({ baru_sampai: '2026-11-07' }, '2026-11-08')).toBe(false)
    expect(isNew({ baru_sampai: null }, '2026-11-08')).toBe(false)
  })
  it('todayIso memakai tanggal lokal dua digit', () => {
    expect(todayIso(new Date(2026, 0, 5))).toBe('2026-01-05')
  })
})

describe('filterLayers', () => {
  const list = [
    layer({ slug: 'a', nama: 'Cakupan SPM', kategori: 'SPM', sumber: 'Arumanis', kind: 'internal' }),
    layer({ slug: 'b', nama: 'Batas provinsi', kategori: 'Administrasi', sumber: 'BIG', kind: 'wms' }),
    layer({ slug: 'c', nama: 'Gempa dirasakan', kategori: 'Bencana', sumber: 'BMKG', kind: 'feature' }),
  ]
  const none = { q: '', kategori: '', kind: '' }

  it('tanpa filter mengembalikan semuanya', () => {
    expect(filterLayers(list, none)).toHaveLength(3)
  })
  it('kata kunci mencari di nama, sumber, dan kategori tanpa peduli huruf besar', () => {
    expect(filterLayers(list, { ...none, q: ' batas ' }).map((l) => l.slug)).toEqual(['b'])
    expect(filterLayers(list, { ...none, q: 'bmkg' }).map((l) => l.slug)).toEqual(['c'])
    expect(filterLayers(list, { ...none, q: 'administrasi' }).map((l) => l.slug)).toEqual(['b'])
  })
  it('kategori dan jenis menyaring tepat, dan digabung dengan kata kunci', () => {
    expect(filterLayers(list, { ...none, kategori: 'Bencana' }).map((l) => l.slug)).toEqual(['c'])
    expect(filterLayers(list, { ...none, kind: 'wms' }).map((l) => l.slug)).toEqual(['b'])
    expect(filterLayers(list, { q: 'gempa', kategori: 'SPM', kind: '' })).toEqual([])
  })
})

describe('kindBadge', () => {
  it('setiap jenis punya teks sendiri, bukan hanya warna', () => {
    const kinds = ['internal', 'feature', 'arcgis', 'wms', 'vector', 'pmtiles'] as const
    const labels = kinds.map((k) => kindBadge(k).label)
    expect(new Set(labels).size).toBe(kinds.length)
  })
})

describe('exportFileName', () => {
  it('memakai tanggal dan jam lokal dengan angka dua digit', () => {
    expect(exportFileName(new Date(2026, 10, 8, 5, 3))).toBe('gambar-20261108-0503.geojson')
  })
})
