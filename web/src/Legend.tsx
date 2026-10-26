import { CAKUPAN_BINS, CAKUPAN_TANPA_TARGET } from './cakupanApi'

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
