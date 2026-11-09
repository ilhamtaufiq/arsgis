import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { AsetFormPanel, AsetModeBar } from './AsetEditor'
import { canSeeCustomerData, fetchFeatures, fetchMe, fetchNetworks } from './api'
import { DesaInfo, InfoPanel } from './InfoPanel'
import { fetchDesaGeo } from './cakupanApi'
import { Legend } from './Legend'
import { FeatureLegend } from './FeatureLegend'
import { MapView } from './MapView'
import { LayerChips } from './LayerChips'
import { ToolButtons, ToolPanel } from './MapTools'
import { AuthPanel } from './Auth'
import { SearchBox } from './Search'
import { DEFAULT_LAYERS, DEFAULT_PIPE_FUNCTIONS, useMapStore } from './store'
import type { Bbox, FeatureType } from './types'

/** Sama dengan `Bbox::MAX_SPAN_DEG` di backend (pipa-core). */
const MAX_BBOX_DEG = 2

export function App() {
  const networkId = useMapStore((s) => s.networkId)
  const bbox = useMapStore((s) => s.bbox)
  const selected = useMapStore((s) => s.selected)
  const panel = useMapStore((s) => s.panel)
  const cakupanOpen = useMapStore((s) => s.cakupanOpen)
  const desaSelected = useMapStore((s) => s.desaSelected)
  const toggleCakupan = useMapStore((s) => s.toggleCakupan)
  const selectDesa = useMapStore((s) => s.selectDesa)
  const setNetwork = useMapStore((s) => s.setNetwork)
  const setBbox = useMapStore((s) => s.setBbox)
  const select = useMapStore((s) => s.select)
  const editor = useMapStore((s) => s.editor)

  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const desaGeo = useQuery({ queryKey: ['desa-geo'], queryFn: fetchDesaGeo, staleTime: 5 * 60_000 })
  const user = me.data ?? null
  const customerAllowed = canSeeCustomerData(user)

  // Peta dimulai tanpa unit. Jaringan (unit SPAM) dipilih pengguna; cakupan desa tampil tanpa pilihan.
  const units = (networks.data ?? []).filter((n) => n.unit_spam_id !== null)

  // Sr hanya ikut diminta bila peran mengizinkan, supaya data pelanggan tidak dikirim ke pengunjung.
  const activeLayers: FeatureType[] = DEFAULT_LAYERS.filter((t) => customerAllowed || t !== 'sr')
  const pipeFunctions = DEFAULT_PIPE_FUNCTIONS
  const requested: FeatureType[] = ['pipa', ...activeLayers]
  const typesKey = [...requested].sort().join(',')

  // Backend menolak bbox yang lebar lebih dari MAX_BBOX_DEG (400). Saat peta di-zoom out sejauh itu, fitur tidak diminta.
  const bboxTerlaluLebar =
    bbox !== null && (bbox.east - bbox.west > MAX_BBOX_DEG || bbox.north - bbox.south > MAX_BBOX_DEG)

  // Semua unit diambil sekali per bbox; pilihan unit hanya menyaring di klien.
  const features = useQuery({
    queryKey: ['features', typesKey, bboxKey(bbox), user?.role ?? 'anon'],
    queryFn: ({ signal }) => fetchFeatures(null, bbox!, typesKey.split(','), signal),
    enabled: bbox !== null && !me.isPending && !bboxTerlaluLebar,
    placeholderData: (prev) => prev,
  })

  // Fitur pipa tanpa fungsi dianggap distribusi sampai datanya diklasifikasikan.
  const visible = useMemo(
    () =>
      (features.data ?? []).filter(
        (f) =>
          // Aset yang sedang dipindah digantikan objek yang bisa diseret.
          !(editor?.kind === 'pindah' && editor.feature.id === f.id) &&
          (networkId === null || f.properties.network_id === networkId) &&
          (f.properties.feature_type !== 'pipa' || pipeFunctions.includes(f.properties.fungsi ?? 'distribusi')),
      ),
    [features.data, pipeFunctions, networkId, editor],
  )

  const handleBbox = (b: Bbox) => setBbox(b)

  return (
    <div className="app">
      <main className={`stage${panel ? ' has-panel' : ''}`}>
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
                onChange={(e) => setNetwork(e.target.value === '' ? null : Number(e.target.value))}
              >
                <option value="">Semua unit SPAM</option>
                {units.map((n) => (
                  <option key={n.id} value={n.id}>{n.nama}</option>
                ))}
              </select>
            ) : (
              <span>{networks.isPending ? 'memuat unit…' : 'belum ada unit SPAM'}</span>
            )}
          </div>
          <SearchBox features={visible} />
          <LayerChips />
        </div>

        <div className="float-right">
          <AuthPanel user={user} />
        </div>

        <div className="toolbar">
          <button type="button" className="round" aria-label="Cakupan SPM" aria-pressed={cakupanOpen} onClick={toggleCakupan}>
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2z M9 4v14 M15 6v14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
            </svg>
          </button>
          <ToolButtons />
          <a className="round" href="/dashboard" aria-label="Dashboard" title="Dashboard">
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
              <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </a>
        </div>

        <div className="notices">
          {networks.isError && <div className="banner">Gagal menghubungi server data.</div>}
          {features.isError && <div className="banner">{(features.error as Error).message}</div>}
          {features.isFetching && <div className="status">Memuat fitur…</div>}
        </div>

        <ToolPanel />
        {customerAllowed && <AsetFormPanel />}
        {customerAllowed && <AsetModeBar />}
        <FeatureLegend />
        {cakupanOpen && <Legend />}
        {bboxTerlaluLebar && (
          <div
            role="status"
            style={{
              position: 'absolute',
              bottom: 16,
              left: '50%',
              transform: 'translateX(-50%)',
              zIndex: 1000,
              background: '#fff',
              padding: '6px 12px',
              borderRadius: 8,
              boxShadow: '0 1px 4px rgba(0,0,0,.2)',
              fontSize: 13,
            }}
          >
            Perbesar peta untuk melihat pipa dan titik.
          </div>
        )}
        {desaSelected ? (
          <DesaInfo desa={desaSelected} onClose={() => selectDesa(null)} />
        ) : (
          <InfoPanel feature={selected} onClose={() => select(null)} bolehEdit={customerAllowed} />
        )}
      </main>
    </div>
  )
}

function bboxKey(b: Bbox | null) {
  return b ? [b.west, b.south, b.east, b.north].map((v) => v.toFixed(4)).join(',') : null
}
