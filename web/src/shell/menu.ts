import { canReadPelanggan, canSeeCustomerData, type User } from '../api'

export type IdMenu = 'peta' | 'dashboard' | 'pelanggan' | 'om' | 'impor' | 'admin'

export type ItemMenu = {
  id: IdMenu
  judul: string
  href: string
}

/** Semua item menu, urutan tampil. Hak akses dicek di `menuUntukUser`. */
export const SEMUA_MENU: readonly ItemMenu[] = [
  { id: 'peta', judul: 'Peta', href: '/peta' },
  { id: 'dashboard', judul: 'Dashboard', href: '/dashboard' },
  { id: 'pelanggan', judul: 'Pelanggan', href: '/pelanggan' },
  { id: 'om', judul: 'O&M', href: '/om' },
  { id: 'impor', judul: 'Impor data', href: '/impor' },
  { id: 'admin', judul: 'Admin', href: '/admin' },
]

/**
 * Item menu yang boleh dilihat pengguna. Tanpa login tidak ada item, karena semua halaman
 * di balik gerbang login. Peran mengikuti aturan yang sama dengan halaman masing-masing.
 */
export function menuUntukUser(user: User | null): ItemMenu[] {
  if (!user) return []
  return SEMUA_MENU.filter((item) => {
    switch (item.id) {
      case 'pelanggan':
        return canReadPelanggan(user)
      case 'impor':
        return canSeeCustomerData(user)
      case 'admin':
        return user.role === 'admin'
      default:
        return true
    }
  })
}

/** Menghapus garis miring di akhir path. Path "/" tetap "/". */
export function normalisasiPath(pathname: string): string {
  const bersih = pathname.replace(/\/+$/, '')
  return bersih === '' ? '/' : bersih
}

/** Item menu yang aktif untuk path saat ini. Detail desa termasuk bagian dari peta. */
export function itemAktif(pathname: string): IdMenu | null {
  const path = normalisasiPath(pathname)
  if (/^\/desa\/\d+$/.test(path)) return 'peta'
  const cocok = SEMUA_MENU.find((item) => path === item.href || path.startsWith(`${item.href}/`))
  return cocok?.id ?? null
}
