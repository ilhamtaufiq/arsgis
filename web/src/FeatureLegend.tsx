import './feature-legend.css'
import { FEATURE_META, ikonHtml } from './featureIcons'
import type { FeatureType } from './types'

// Urutan tampil: jaringan dulu (pipa, valve, reservoir), lalu titik pelayanan, lalu aset baru.
const URUTAN: FeatureType[] = [
  'pipa',
  'tapping',
  'valve',
  'reservoir',
  'bronkaptering',
  'sr',
  'intake',
  'mata_air',
  'pompa',
  'bak_penahan_tekanan',
  'menara_air',
  'hidran_umum',
  'meter_induk',
]

/**
 * Legenda jenis fitur. Tertutup secara default supaya tidak menutupi peta; dibuka dari ringkasan.
 * Ikon memakai sumber yang sama dengan penanda di peta.
 */
export function FeatureLegend() {
  return (
    <details className="feature-legend">
      <summary>Jenis fitur</summary>
      <ul>
        {URUTAN.map((t) => (
          <li key={t}>
            {t === 'pipa' ? (
              <span className="feature-legend-garis" style={{ background: FEATURE_META.pipa.color }} aria-hidden="true" />
            ) : (
              <span
                className="feature-legend-ikon"
                aria-hidden="true"
                dangerouslySetInnerHTML={{ __html: ikonHtml(t, null, 20) }}
              />
            )}
            {FEATURE_META[t].label}
          </li>
        ))}
      </ul>
    </details>
  )
}
