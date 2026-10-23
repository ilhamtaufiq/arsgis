import type { PipeFeature } from './types'

const LABEL: Record<string, string> = {
  pipa: 'Pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
}

type Props = { feature: PipeFeature | null; onClose: () => void }

export function InfoPanel({ feature, onClose }: Props) {
  if (!feature) {
    return <aside className="info empty">Klik pipa atau titik di peta untuk melihat detailnya.</aside>
  }
  const p = feature.properties
  const rows: [string, string | number | null][] = [
    ['Nama', p.nama],
    ['Diameter (mm)', p.diameter_mm],
    ['Material', p.material],
    ['Tahun pasang', p.tahun_pasang],
    ['Kondisi', p.kondisi],
    ['Status', p.status],
  ]
  return (
    <aside className="info">
      <div className="info-head">
        <strong>{LABEL[p.feature_type] ?? p.feature_type}</strong>
        <button type="button" onClick={onClose} aria-label="Tutup">×</button>
      </div>
      <dl>
        {rows.map(([k, v]) => (
          <div key={k}>
            <dt>{k}</dt>
            <dd>{v ?? '–'}</dd>
          </div>
        ))}
      </dl>
    </aside>
  )
}
