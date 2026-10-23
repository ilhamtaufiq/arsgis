import type { FeatureCollection } from 'geojson'
import type { Bbox, Network, PipeFeature } from './types'

export async function fetchNetworks(): Promise<Network[]> {
  const res = await fetch('/api/v1/networks')
  if (!res.ok) throw new Error(`Daftar jaringan gagal dimuat (${res.status})`)
  const body = (await res.json()) as { data: Network[] }
  return body.data
}

export async function fetchFeatures(networkId: number, bbox: Bbox, types: string[], signal?: AbortSignal) {
  const params = new URLSearchParams({
    bbox: [bbox.west, bbox.south, bbox.east, bbox.north].map((v) => v.toFixed(6)).join(','),
  })
  if (types.length) params.set('types', types.join(','))
  const res = await fetch(`/api/v1/networks/${networkId}/geojson?${params}`, { signal })
  if (res.status === 422) throw new Error('Area terlalu banyak fitur. Perkecil area peta.')
  if (!res.ok) throw new Error(`Data fitur gagal dimuat (${res.status})`)
  const body = (await res.json()) as FeatureCollection
  return body.features as PipeFeature[]
}
