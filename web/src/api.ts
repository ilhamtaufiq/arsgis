import type { FeatureCollection } from 'geojson'
import type { Bbox, Network, PipeFeature } from './types'

export type Role = 'viewer' | 'editor' | 'admin'

export type User = { username: string; role: Role }

/** Editor dan admin boleh melihat data pelanggan (sr). */
export function canSeeCustomerData(user: User | null): boolean {
  return user?.role === 'editor' || user?.role === 'admin'
}

export async function fetchMe(): Promise<User | null> {
  const res = await fetch('/api/v1/auth/me')
  if (res.status === 401) return null
  if (!res.ok) throw new Error(`Status sesi gagal dimuat (${res.status})`)
  return (await res.json()) as User
}

export async function login(username: string, password: string): Promise<User> {
  const res = await fetch('/api/v1/auth/login', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })
  if (res.status === 401) throw new Error('Username atau password salah.')
  if (!res.ok) throw new Error(`Login gagal (${res.status})`)
  return (await res.json()) as User
}

export async function logout(): Promise<void> {
  await fetch('/api/v1/auth/logout', { method: 'POST' })
}

export async function fetchNetworks(): Promise<Network[]> {
  const res = await fetch('/api/v1/networks')
  if (!res.ok) throw new Error(`Daftar jaringan gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: Network[] }
  return body.data
}

/** Fitur dalam bbox. Tanpa `networkId`, fitur dari semua unit SPAM ikut diambil. */
export async function fetchFeatures(networkId: number | null, bbox: Bbox, types: string[], signal?: AbortSignal) {
  const params = new URLSearchParams({
    bbox: [bbox.west, bbox.south, bbox.east, bbox.north].map((v) => v.toFixed(6)).join(','),
  })
  if (types.length) params.set('types', types.join(','))
  const base = networkId === null ? '/api/v1/features/geojson' : `/api/v1/networks/${networkId}/geojson`
  const res = await fetch(`${base}?${params}`, { signal })
  if (res.status === 403) throw new Error('Akses ke data ini butuh peran editor atau admin.')
  if (res.status === 422) throw new Error('Area terlalu banyak fitur. Perkecil area peta.')
  if (!res.ok) throw new Error(`Data fitur gagal dimuat (${res.status})`)
  const body = (await res.json()) as FeatureCollection
  return body.features as PipeFeature[]
}
