import { useQuery } from '@tanstack/react-query'
import { AuthPanel } from './Auth'
import { canSeeCustomerData, fetchFeatures, fetchMe, fetchNetworks } from './api'
import { ImportPanel } from './Import'
import { InfoPanel } from './InfoPanel'
import { LayerSwitcher } from './LayerSwitcher'
import { MapView } from './MapView'
import { useMapStore } from './store'
import type { Bbox, FeatureType } from './types'
import { useMemo } from 'react'

export function App() {
  const networkId = useMapStore((s) => s.networkId)
  const bbox = useMapStore((s) => s.bbox)
  const layers = useMapStore((s) => s.layers)
  const pipeFunctions = useMapStore((s) => s.pipeFunctions)
  const selected = useMapStore((s) => s.selected)
  const setNetwork = useMapStore((s) => s.setNetwork)
  const setBbox = useMapStore((s) => s.setBbox)
  const toggleLayer = useMapStore((s) => s.toggleLayer)
  const togglePipeFunction = useMapStore((s) => s.togglePipeFunction)
  const select = useMapStore((s) => s.select)

  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const user = me.data ?? null
  const customerAllowed = canSeeCustomerData(user)

  const firstNetwork = networks.data?.[0]?.id
  if (networkId === null && firstNetwork !== undefined) setNetwork(firstNetwork)

  // Sr hanya ikut diminta bila peran mengizinkan, supaya data pelanggan tidak dikirim ke pengunjung.
  const activeLayers: FeatureType[] = layers.filter((t) => customerAllowed || t !== 'sr')
  // Pipa diminta dari server selama salah satu fungsinya aktif; pemisahan transmisi/distribusi dilakukan di klien.
  const requested: FeatureType[] = pipeFunctions.length ? ['pipa', ...activeLayers] : activeLayers
  const typesKey = [...requested].sort().join(',')

  const features = useQuery({
    queryKey: ['features', networkId, typesKey, bboxKey(bbox), user?.role ?? 'anon'],
    queryFn: ({ signal }) => fetchFeatures(networkId!, bbox!, typesKey ? typesKey.split(',') : [], signal),
    enabled: networkId !== null && bbox !== null && typesKey !== '' && !me.isPending,
    placeholderData: (prev) => prev,
  })

  // Fitur pipa tanpa fungsi dianggap distribusi sampai datanya diklasifikasikan.
  const visible = useMemo(
    () =>
      (features.data ?? []).filter(
        (f) => f.properties.feature_type !== 'pipa' || pipeFunctions.includes(f.properties.fungsi ?? 'distribusi'),
      ),
    [features.data, pipeFunctions],
  )

  const handleBbox = (b: Bbox) => setBbox(b)

  return (
    <div className="app">
      <main className="stage">
        <MapView features={visible} onBboxChange={handleBbox} />

        <div className="float-top">
          <div className="brand">
            <strong>arsgis</strong>
            <span>{networks.data?.[0]?.nama ?? 'memuat jaringan…'}</span>
          </div>
          <LayerSwitcher
            active={activeLayers}
            pipeFunctions={pipeFunctions}
            canSeeCustomerData={customerAllowed}
            onToggle={toggleLayer}
            onTogglePipe={togglePipeFunction}
          />
        </div>

        <div className="float-right">
          <AuthPanel user={user} />
          {customerAllowed && <ImportPanel networkId={networkId} />}
        </div>

        <div className="notices">
          {networks.isError && <div className="banner">Gagal menghubungi server data.</div>}
          {features.isError && <div className="banner">{(features.error as Error).message}</div>}
          {features.isFetching && <div className="status">Memuat fitur…</div>}
        </div>

        <InfoPanel feature={selected} onClose={() => select(null)} />
      </main>
    </div>
  )
}

function bboxKey(b: Bbox | null) {
  return b ? [b.west, b.south, b.east, b.north].map((v) => v.toFixed(4)).join(',') : null
}
