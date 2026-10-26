import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { AuthPanel } from './Auth'
import { canSeeCustomerData, fetchFeatures, fetchMe, fetchNetworks } from './api'
import { ImportPanel } from './Import'
import { DesaInfo, InfoPanel } from './InfoPanel'
import { fetchDesaGeo } from './cakupanApi'
import { Legend } from './Legend'
import { MapView } from './MapView'
import { SearchBox } from './Search'
import { DEFAULT_LAYERS, DEFAULT_PIPE_FUNCTIONS, useMapStore } from './store'
import type { Bbox, FeatureType } from './types'

export function App() {
  const networkId = useMapStore((s) => s.networkId)
  const bbox = useMapStore((s) => s.bbox)
  const selected = useMapStore((s) => s.selected)
  const importOpen = useMapStore((s) => s.importOpen)
  const cakupanOpen = useMapStore((s) => s.cakupanOpen)
  const desaSelected = useMapStore((s) => s.desaSelected)
  const toggleCakupan = useMapStore((s) => s.toggleCakupan)
  const selectDesa = useMapStore((s) => s.selectDesa)
  const setNetwork = useMapStore((s) => s.setNetwork)
  const setBbox = useMapStore((s) => s.setBbox)
  const select = useMapStore((s) => s.select)
  const toggleImport = useMapStore((s) => s.toggleImport)

  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const desaGeo = useQuery({ queryKey: ['desa-geo'], queryFn: fetchDesaGeo, staleTime: 5 * 60_000 })
  const user = me.data ?? null
  const customerAllowed = canSeeCustomerData(user)

  // Peta dimulai tanpa unit. Jaringan (unit SPAM) dipilih pengguna; cakupan desa tampil tanpa pilihan.
  const units = (networks.data ?? []).filter((n) => n.unit_spam_id !== null)
  const unitName = units.find((n) => n.id === networkId)?.nama

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
        <MapView
          features={visible}
          desa={desaGeo.data}
          showCakupan={cakupanOpen}
          focusDesaId={units.find((n) => n.id === networkId)?.desa_id ?? null}
          onBboxChange={handleBbox}
        />

        <div className="float-top">
          <div className="brand">
            <strong>arsgis</strong>
            {units.length > 0 ? (
              <select
                className="network-select"
                aria-label="Pilih unit SPAM"
                value={networkId ?? ''}
                onChange={(e) => setNetwork(Number(e.target.value))}
              >
                <option value="" disabled>
                  Pilih unit SPAM…
                </option>
                {units.map((n) => (
                  <option key={n.id} value={n.id}>{n.nama}</option>
                ))}
              </select>
            ) : (
              <span>{networks.isPending ? 'memuat unit…' : 'belum ada unit SPAM'}</span>
            )}
          </div>
          <SearchBox features={visible} />
        </div>

        <div className="float-right">
          <AuthPanel user={user} />
          {customerAllowed && <ImportPanel networkId={networkId} open={importOpen} onClose={toggleImport} />}
        </div>

        <div className="toolbar">
          <button type="button" className="round" aria-label="Cakupan SPM" aria-pressed={cakupanOpen} onClick={toggleCakupan}>
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2z M9 4v14 M15 6v14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
            </svg>
          </button>
          <a className="round" href="/dashboard" aria-label="Dashboard" title="Dashboard">
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </a>
          {customerAllowed && (
            <button type="button" className="round" aria-label="Impor KML" aria-pressed={importOpen} onClick={toggleImport}>
              <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
                <path d="M4 20l4-1 11-11-3-3L5 16l-1 4z" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
              </svg>
            </button>
          )}
        </div>

        <div className="notices">
          {networks.isError && <div className="banner">Gagal menghubungi server data.</div>}
          {features.isError && <div className="banner">{(features.error as Error).message}</div>}
          {networkId === null && units.length > 0 && <div className="banner">Pilih unit SPAM untuk melihat jaringan pipa dan sambungan rumah.</div>}
          {networkId !== null && features.isFetching && <div className="status">Memuat fitur {unitName ? `· ${unitName}` : ''}…</div>}
        </div>

        {cakupanOpen && <Legend />}
        {desaSelected ? (
          <DesaInfo desa={desaSelected} onClose={() => selectDesa(null)} />
        ) : (
          <InfoPanel feature={selected} onClose={() => select(null)} />
        )}
      </main>
    </div>
  )
}

function bboxKey(b: Bbox | null) {
  return b ? [b.west, b.south, b.east, b.north].map((v) => v.toFixed(4)).join(',') : null
}
