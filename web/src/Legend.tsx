import { CAKUPAN_BINS, CAKUPAN_TANPA_TARGET } from './cakupanApi'
import { ASET_BARU, ASET_COLOR, ASET_LABEL } from './asetStyle'

/** Legenda titik aset baru. Belum dipasang di App karena belum ada kontrol layer untuk jenis ini. */
export function AsetLegend() {
  return (
    <div className="legend" aria-label="Legenda aset">
      <strong>Aset</strong>
      <ul>
        {ASET_BARU.map((k) => (
          <li key={k}>
            <span className="swatch" style={{ background: ASET_COLOR[k], borderRadius: '50%' }} />
            {ASET_LABEL[k]}
          </li>
        ))}
      </ul>
    </div>
  )
}

/** Legenda warna cakupan SPM per desa. */
export function Legend() {
  return (
    <div className="legend" aria-label="Legenda cakupan SPM">
      <strong>Cakupan SPM</strong>
      <ul>
        {CAKUPAN_BINS.map((b) => (
          <li key={b.label}>
            <span className="swatch" style={{ background: b.color }} />
            {b.label}
          </li>
        ))}
        <li>
          <span className="swatch" style={{ background: CAKUPAN_TANPA_TARGET.color }} />
          {CAKUPAN_TANPA_TARGET.label}
        </li>
      </ul>
    </div>
  )
}
