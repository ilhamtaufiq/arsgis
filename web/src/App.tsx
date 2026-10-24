import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { AuthPanel } from './Auth'
import { canSeeCustomerData, fetchFeatures, fetchMe, fetchNetworks } from './api'
import { Dashboard } from './Dashboard'
import { ImportPanel } from './Import'
import { InfoPanel } from './InfoPanel'
import { MapView } from './MapView'
import { SearchBox } from './Search'
import { DEFAULT_LAYERS, DEFAULT_PIPE_FUNCTIONS, useMapStore } from './store'
import type { Bbox, FeatureType } from './types'

export function App() {
  const networkId = useMapStore((s) => s.networkId)
  const bbox = useMapStore((s) => s.bbox)
  const selected = useMapStore((s) => s.selected)
  const importOpen = useMapStore((s) => s.importOpen)
  const setNetwork = useMapStore((s) => s.setNetwork)
  const setBbox = useMapStore((s) => s.setBbox)
  const select = useMapStore((s) => s.select)
  const toggleImport = useMapStore((s) => s.toggleImport)
  const dashboardOpen = useMapStore((s) => s.dashboardOpen)
  const toggleDashboard = useMapStore((s) => s.toggleDashboard)

  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const user = me.data ?? null
  const customerAllowed = canSeeCustomerData(user)

  const firstNetwork = networks.data?.[0]?.id
  if (networkId === null && firstNetwork !== undefined) setNetwork(firstNetwork)

  // Sr hanya ikut diminta bila peran mengizinkan, supaya data pelanggan tidak dikirim ke pengunjung.
  const activeLayers: FeatureType[] = DEFAULT_LAYERS.filter((t) => customerAllowed || t !== 'sr')
  const pipeFunctions = DEFAULT_PIPE_FUNCTIONS
  const requested: FeatureType[] = ['pipa', ...activeLayers]
  const typesKey = [...requested].sort().join(',')

  const features = useQuery({
    queryKey: ['features', networkId, typesKey, bboxKey(bbox), user?.role ?? 'anon'],
    queryFn: ({ signal }) => fetchFeatures(networkId!, bbox!, typesKey.split(','), signal),
    enabled: networkId !== null && bbox !== null && !me.isPending,
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
            {networks.data && networks.data.length > 1 ? (
              <select
                className="network-select"
                aria-label="Pilih jaringan"
                value={networkId ?? ''}
                onChange={(e) => setNetwork(Number(e.target.value))}
              >
                {networks.data.map((n) => (
                  <option key={n.id} value={n.id}>{n.nama}</option>
                ))}
              </select>
            ) : (
              <span>{networks.data?.[0]?.nama ?? 'memuat jaringan…'}</span>
            )}
          </div>
          <SearchBox features={visible} />
        </div>

        <div className="float-right">
          <AuthPanel user={user} />
          {customerAllowed && <ImportPanel networkId={networkId} open={importOpen} onClose={toggleImport} />}
        </div>

        <div className="toolbar">
          <button type="button" className="round" aria-label="Dashboard" aria-pressed={dashboardOpen} onClick={toggleDashboard}>
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
          {customerAllowed && (
            <button type="button" className="round" aria-label="Impor KML" aria-pressed={importOpen} onClick={toggleImport}>
              <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
                <path d="M4 20l4-1 11-11-3-3L5 16l-1 4z" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
              </svg>
            </button>
          )}
        </div>

        <Dashboard networkId={networkId} open={dashboardOpen} onClose={toggleDashboard} role={user?.role ?? null} />

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
