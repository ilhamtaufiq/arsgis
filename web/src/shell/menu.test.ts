import { describe, expect, it } from 'vitest'
import type { Role, User } from '../api'
import { itemAktif, menuUntukUser, normalisasiPath } from './menu'

const pengguna = (role: Role): User => ({ username: `u-${role}`, role })
const idMenu = (role: Role | null) => menuUntukUser(role === null ? null : pengguna(role)).map((i) => i.id)

describe('menuUntukUser', () => {
  it('tanpa login tidak menampilkan item', () => {
    expect(menuUntukUser(null)).toEqual([])
  })

  it('viewer dan kepala desa hanya melihat Peta, Dashboard, dan O&M', () => {
    expect(idMenu('viewer')).toEqual(['peta', 'dashboard', 'om'])
    expect(idMenu('kepala_desa')).toEqual(['peta', 'dashboard', 'om'])
  })

  it('bendahara melihat Pelanggan tanpa Impor', () => {
    expect(idMenu('bendahara')).toEqual(['peta', 'dashboard', 'pelanggan', 'om'])
  })

  it('editor melihat Pelanggan dan Impor, tanpa Admin', () => {
    expect(idMenu('editor')).toEqual(['peta', 'dashboard', 'pelanggan', 'om', 'impor'])
  })

  it('admin melihat semua item', () => {
    expect(idMenu('admin')).toEqual(['peta', 'dashboard', 'pelanggan', 'om', 'impor', 'admin'])
  })

  it('label Impor memakai "Impor data"', () => {
    const impor = menuUntukUser(pengguna('admin')).find((i) => i.id === 'impor')
    expect(impor?.judul).toBe('Impor data')
    expect(impor?.href).toBe('/impor')
  })
})

describe('itemAktif', () => {
  it('menandai item sesuai path', () => {
    expect(itemAktif('/peta')).toBe('peta')
    expect(itemAktif('/dashboard')).toBe('dashboard')
    expect(itemAktif('/pelanggan')).toBe('pelanggan')
    expect(itemAktif('/om')).toBe('om')
    expect(itemAktif('/impor')).toBe('impor')
    expect(itemAktif('/admin')).toBe('admin')
  })

  it('mengabaikan garis miring di akhir', () => {
    expect(itemAktif('/om/')).toBe('om')
  })

  it('detail desa dianggap bagian dari peta', () => {
    expect(itemAktif('/desa/42')).toBe('peta')
  })

  it('path lain tidak menandai item apa pun', () => {
    expect(itemAktif('/')).toBeNull()
    expect(itemAktif('/masuk')).toBeNull()
    expect(itemAktif('/tidak-ada')).toBeNull()
    expect(itemAktif('/omah')).toBeNull()
  })
})

describe('normalisasiPath', () => {
  it('path root tetap "/"', () => {
    expect(normalisasiPath('/')).toBe('/')
    expect(normalisasiPath('')).toBe('/')
    expect(normalisasiPath('///')).toBe('/')
  })
})
