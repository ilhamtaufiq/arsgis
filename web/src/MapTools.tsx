import { useEffect, useId, useMemo, useRef, useState, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown, Download, Layers, Minus, Pencil, Search, SquareDashed, X } from 'lucide-react'
import { fetchCatalog, filterLayers, kindBadge, type CatalogLayer, type LayerFilter } from './catalogApi'
import { clearDrawn, downloadDrawn } from './drawing'
import { useMapStore } from './store'
import { useLayerToggle } from './useLayerToggle'
import './map-tokens.css'
import './map-ui.css'
import './tools.css'

/** Dua tombol bulat untuk deretan alat di bawah peta: daftar layer dan alat gambar. */
export function ToolButtons() {
  const panel = useMapStore((s) => s.panel)
  const setPanel = useMapStore((s) => s.setPanel)
  return (
    <>
      <button type="button" className="round" aria-label="Layer peta" title="Layer peta" aria-pressed={panel === 'layer'} onClick={() => setPanel('layer')}>
        <Layers size={18} aria-hidden="true" />
      </button>
      <button type="button" className="round" aria-label="Alat gambar" title="Alat gambar" aria-pressed={panel === 'alat'} onClick={() => setPanel('alat')}>
        <Pencil size={18} aria-hidden="true" />
      </button>
    </>
  )
}

/** Panel di tepi kiri: header (ikon, judul, minimalkan, tutup) lalu isi daftar layer atau alat gambar. */
export function ToolPanel() {
  const panel = useMapStore((s) => s.panel)
  const setPanel = useMapStore((s) => s.setPanel)
  const [minimized, setMinimized] = useState(false)
  const titleId = useId()
  const titleRef = useRef<HTMLHeadingElement>(null)

  // Panel baru dimulai terbuka penuh, dan fokus pindah ke judulnya supaya pembaca layar tahu panel muncul.
  useEffect(() => {
    setMinimized(false)
    if (panel) titleRef.current?.focus()
  }, [panel])

  if (!panel) return null
  const Icon = panel === 'layer' ? Layers : Pencil
  const title = panel === 'layer' ? 'Layer' : 'Gambar'

  return (
    <aside
      className="side-panel"
      aria-labelledby={titleId}
      onKeyDown={(e) => {
        // Esc yang sedang membatalkan gambar tidak ikut menutup panel.
        if (e.key === 'Escape' && !useMapStore.getState().drawMode) setPanel(null)
      }}
    >
      <header className="panel-head">
        <Icon size={16} aria-hidden="true" />
        <h2 id={titleId} ref={titleRef} tabIndex={-1}>{title}</h2>
        <button type="button" aria-label={minimized ? 'Buka panel' : 'Minimalkan panel'} aria-expanded={!minimized} onClick={() => setMinimized((m) => !m)}>
          {minimized ? <ChevronDown size={16} aria-hidden="true" /> : <Minus size={16} aria-hidden="true" />}
        </button>
        <button type="button" aria-label="Tutup panel" onClick={() => setPanel(null)}>
          <X size={16} aria-hidden="true" />
        </button>
      </header>
      {!minimized && <div className="panel-body">{panel === 'layer' ? <LayerPanel /> : <DrawPanel />}</div>}
    </aside>
  )
}

function AccordionSection({
  icon,
  title,
  subtitle,
  defaultOpen = false,
  children,
}: {
  icon: ReactNode
  title: string
  subtitle: string
  defaultOpen?: boolean
  children: ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen)
  const id = useId()
  return (
    <section className="accordion">
      <h3>
        <button type="button" className="accordion-head" aria-expanded={open} aria-controls={id} onClick={() => setOpen((o) => !o)}>
          <span className="acc-icon" aria-hidden="true">{icon}</span>
          <span className="acc-text">
            <span className="acc-title">{title}</span>
            <span className="acc-sub">{subtitle}</span>
          </span>
          <ChevronDown className="acc-chevron" size={16} aria-hidden="true" />
        </button>
      </h3>
      <div id={id} className="accordion-body" hidden={!open}>
        {children}
      </div>
    </section>
  )
}

function TypeBadge({ kind }: { kind: CatalogLayer['kind'] }) {
  const b = kindBadge(kind)
  return <span className={`type-badge tone-${b.tone}`}>{b.label}</span>
}

function LayerRow({ layer }: { layer: CatalogLayer }) {
  const { on, toggle } = useLayerToggle(layer)
  return (
    <li>
      <label className="layer-row">
        <input type="checkbox" checked={on} onChange={toggle} />
        <span className="layer-text">
          <span className="layer-name">{layer.nama}</span>
          <span className="layer-src">{layer.sumber}</span>
        </span>
        <TypeBadge kind={layer.kind} />
      </label>
    </li>
  )
}

function ActiveRow({ layer }: { layer: CatalogLayer }) {
  const { on, toggle } = useLayerToggle(layer)
  if (!on) return null
  return (
    <li className="layer-active">
      <span className="layer-name">{layer.nama}</span>
      <button type="button" onClick={toggle} aria-label={`Matikan ${layer.nama}`}>Matikan</button>
    </li>
  )
}

function LayerPanel() {
  const catalog = useQuery({ queryKey: ['catalog'], queryFn: fetchCatalog, staleTime: 5 * 60_000 })
  const cakupanOpen = useMapStore((s) => s.cakupanOpen)
  const [filter, setFilter] = useState<LayerFilter>({ q: '', kategori: '', kind: '' })
  const layers = useMemo(() => catalog.data ?? [], [catalog.data])
  const kategoriOptions = useMemo(() => [...new Set(layers.map((l) => l.kategori))].sort(), [layers])
  const kindOptions = useMemo(() => [...new Set(layers.map((l) => l.kind))].sort(), [layers])
  const shown = useMemo(() => filterLayers(layers, filter), [layers, filter])

  if (catalog.isPending) {
    return (
      <div aria-busy="true" className="stack">
        <div className="skeleton" /><div className="skeleton" /><div className="skeleton" />
      </div>
    )
  }
  if (catalog.isError) {
    return (
      <div className="stack">
        <p className="panel-error">Daftar layer gagal dimuat.</p>
        <button type="button" className="tool-btn" onClick={() => catalog.refetch()}>Coba lagi</button>
      </div>
    )
  }
  if (layers.length === 0) {
    return <p className="panel-muted">Belum ada layer yang bisa dilihat dengan akun ini.</p>
  }

  // Layer aktif dihitung dari layer internal yang dikenali; jenis lain ikut saat didukung.
  const anyActive = layers.some((l) => l.internalKey === 'cakupan-spm') && cakupanOpen
  return (
    <div className="stack">
      <label className="field-search">
        <Search size={14} aria-hidden="true" />
        <input
          aria-label="Cari layer"
          placeholder="Cari layer"
          value={filter.q}
          onChange={(e) => setFilter((f) => ({ ...f, q: e.target.value }))}
        />
      </label>
      <div className="filters">
        <select aria-label="Kategori" value={filter.kategori} onChange={(e) => setFilter((f) => ({ ...f, kategori: e.target.value }))}>
          <option value="">Semua kategori</option>
          {kategoriOptions.map((k) => <option key={k} value={k}>{k}</option>)}
        </select>
        <select aria-label="Jenis" value={filter.kind} onChange={(e) => setFilter((f) => ({ ...f, kind: e.target.value }))}>
          <option value="">Semua jenis</option>
          {kindOptions.map((k) => <option key={k} value={k}>{kindBadge(k).label}</option>)}
        </select>
      </div>
      <p className="panel-count" aria-live="polite">{shown.length} layer</p>
      {shown.length === 0 ? (
        <p className="panel-muted">Tidak ada layer yang cocok. Ubah kata kunci atau filter.</p>
      ) : (
        <ul className="layer-list">{shown.map((l) => <LayerRow key={l.slug} layer={l} />)}</ul>
      )}
      <h3 className="panel-subhead">Layer aktif</h3>
      {anyActive ? (
        <ul className="layer-list">{layers.map((l) => <ActiveRow key={l.slug} layer={l} />)}</ul>
      ) : (
        <p className="panel-muted">Belum ada layer aktif. Nyalakan salah satu dari daftar di atas.</p>
      )}
    </div>
  )
}

function DrawPanel() {
  const drawMode = useMapStore((s) => s.drawMode)
  const drawnCount = useMapStore((s) => s.drawnCount)
  const setDrawMode = useMapStore((s) => s.setDrawMode)
  const [message, setMessage] = useState<string | null>(null)

  function handleExport() {
    const n = downloadDrawn()
    setMessage(n === 0 ? 'Belum ada gambar untuk diunduh.' : `${n} bentuk diunduh sebagai GeoJSON.`)
  }
  function handleClear() {
    clearDrawn()
    setMessage(null)
  }

  return (
    <div className="stack">
      <AccordionSection icon={<Pencil size={14} />} title="Gambar" subtitle="Bentuk di atas peta" defaultOpen>
        <button
          type="button"
          className="tool-btn"
          aria-pressed={drawMode === 'rectangle'}
          onClick={() => setDrawMode(drawMode === 'rectangle' ? null : 'rectangle')}
        >
          <SquareDashed size={16} aria-hidden="true" /> Persegi panjang
        </button>
        {drawMode === 'rectangle' && <p className="panel-muted">Klik dua sudut di peta. Tekan Esc untuk batal.</p>}
      </AccordionSection>

      <AccordionSection icon={<Download size={14} />} title="Hasil dan ekspor" subtitle="Unduh atau hapus gambar" defaultOpen>
        <p className="panel-muted" role="status">
          {drawnCount === 0 ? 'Belum ada gambar.' : `${drawnCount} bentuk di peta.`}
        </p>
        <div className="btn-row">
          <button type="button" className="tool-btn primary" disabled={drawnCount === 0} onClick={handleExport}>Unduh GeoJSON</button>
          <button type="button" className="tool-btn" disabled={drawnCount === 0} onClick={handleClear}>Hapus semua</button>
        </div>
        {message && <p className="panel-muted" role="status">{message}</p>}
      </AccordionSection>
    </div>
  )
}
