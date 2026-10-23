import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchFeatures, fetchNetworks } from './api'
import { InfoPanel } from './InfoPanel'
import { LayerSwitcher } from './LayerSwitcher'
import { MapView } from './MapView'
import { useMapStore } from './store'
import type { Bbox } from './types'

export function App() {
  const networkId = useMapStore((s) => s.networkId)
  const bbox = useMapStore((s) => s.bbox)
  const layers = useMapStore((s) => s.layers)
  const selected = useMapStore((s) => s.selected)
  const setNetwork = useMapStore((s) => s.setNetwork)
  const setBbox = useMapStore((s) => s.setBbox)
  const toggleLayer = useMapStore((s) => s.toggleLayer)
  const select = useMapStore((s) => s.select)

  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })

  const firstNetwork = networks.data?.[0]?.id
  if (networkId === null && firstNetwork !== undefined) setNetwork(firstNetwork)

  // Jenis yang dikirim ke server: hanya layer yang aktif, supaya data sr tidak ikut bila nonaktif.
  const typesKey = useMemo(() => [...layers].sort().join(','), [layers])

  const features = useQuery({
    queryKey: ['features', networkId, typesKey, bboxKey(bbox)],
    queryFn: ({ signal }) => fetchFeatures(networkId!, bbox!, typesKey ? typesKey.split(',') : [], signal),
    enabled: networkId !== null && bbox !== null && typesKey !== '',
    placeholderData: (prev) => prev,
  })

  const handleBbox = (b: Bbox) => setBbox(b)

  return (
    <div className="app">
      <header className="top">
        <div>
          <h1>arsgis</h1>
          <p>WebGIS perpipaan SPAM · {networks.data?.[0]?.nama ?? 'memuat jaringan…'}</p>
        </div>
        <LayerSwitcher active={layers} onToggle={toggleLayer} />
      </header>

      <main className="stage">
        <MapView features={features.data ?? []} onBboxChange={handleBbox} />
        {networks.isError && <div className="banner">Gagal menghubungi server data.</div>}
        {features.isError && <div className="banner">{(features.error as Error).message}</div>}
        {features.isFetching && <div className="status">Memuat fitur…</div>}
        <InfoPanel feature={selected} onClose={() => select(null)} />
      </main>
    </div>
  )
}

function bboxKey(b: Bbox | null) {
  return b ? [b.west, b.south, b.east, b.north].map((v) => v.toFixed(4)).join(',') : null
}
