import type { DesaProps } from './cakupanApi'
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

const nf = (n: number) => n.toLocaleString('id-ID')

/** Kartu capaian SPM satu desa. Data dari endpoint publik, tanpa nama pengurus. */
export function DesaInfo({ desa, onClose }: { desa: DesaProps; onClose: () => void }) {
  const pokmas = desa.pokmas_konflik
    ? 'Ada perbedaan antar unit'
    : desa.punya_pokmas
      ? 'Terisi'
      : 'Belum terisi'
  const rows: [string, string][] = [
    ['Kecamatan', desa.kecamatan],
    ['Target', `${nf(desa.target_kk)} KK`],
    ['JP (perpipaan)', `${nf(desa.jp_kk)} KK`],
    ['BJP', `${nf(desa.bjp_kk)} KK`],
    ['Sambungan rumah', nf(desa.sr)],
    ['Jiwa terlayani', nf(desa.jiwa)],
    ['Unit SPAM', nf(desa.unit_count)],
    ['Profil Pokmas', pokmas],
    ['Batas desa', desa.punya_batas ? 'Ada' : 'Belum ada'],
  ]
  return (
    <aside className="info">
      <div className="info-head">
        <div className="info-title">
          <strong>{desa.nama}</strong>
          <span className="info-sub">
            {desa.cakupan_persen === null ? 'tanpa target' : `cakupan SPM ${desa.cakupan_persen}%`}
          </span>
        </div>
        <button type="button" onClick={onClose} aria-label="Tutup">×</button>
      </div>
      <dl>
        {rows.map(([k, v]) => (
          <div key={k}>
            <dt>{k}</dt>
            <dd>{v}</dd>
          </div>
        ))}
      </dl>
    </aside>
  )
}
