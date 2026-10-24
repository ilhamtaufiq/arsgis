import { useQuery } from '@tanstack/react-query'
import { fetchDashboard } from './dashboardApi'

const JENIS: Record<string, { label: string; color: string }> = {
  pipa: { label: 'Pipa', color: '#1d4ed8' },
  valve: { label: 'Valve', color: '#111111' },
  reservoir: { label: 'Reservoir', color: '#0f766e' },
  sr: { label: 'Sambungan rumah', color: '#a16207' },
  bronkaptering: { label: 'Bronkaptering', color: '#0891b2' },
}

const FUNGSI: Record<string, string> = { transmisi: 'Transmisi', distribusi: 'Distribusi' }

type Props = {
  networkId: number | null
  open: boolean
  onClose: () => void
  role: string | null
}

/** Dashboard jaringan: jumlah fitur, panjang pipa, dan riwayat impor (editor dan admin). */
export function Dashboard({ networkId, open, onClose, role }: Props) {
  const data = useQuery({
    queryKey: ['dashboard', networkId, role ?? 'anon'],
    queryFn: () => fetchDashboard(networkId!),
    enabled: open && networkId !== null,
  })

  if (!open) return null

  const fitur = data.data?.fitur ?? []
  const total = fitur.reduce((sum, f) => sum + f.jumlah, 0)
  const pipa = data.data?.pipa ?? []
  const totalKm = pipa.reduce((sum, p) => sum + p.km, 0)

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <section className="dash" role="dialog" aria-modal="true" aria-labelledby="dash-title" onMouseDown={(e) => e.stopPropagation()}>
        <header className="dash-head">
          <div>
            <h2 id="dash-title">Dashboard</h2>
            <p>{data.data?.jaringan ?? 'Memuat jaringan…'}</p>
          </div>
          <button type="button" className="modal-close" aria-label="Tutup" onClick={onClose}>×</button>
        </header>

        {data.isError && <p className="modal-error">{(data.error as Error).message}</p>}

        <div className="dash-cards">
          <div className="dash-card dash-card-total">
            <span>Total fitur</span>
            <strong>{total.toLocaleString('id-ID')}</strong>
          </div>
          <div className="dash-card">
            <span>Panjang pipa</span>
            <strong>{totalKm.toLocaleString('id-ID', { maximumFractionDigits: 2 })} km</strong>
            {pipa.map((p) => (
              <small key={p.fungsi}>
                {FUNGSI[p.fungsi] ?? p.fungsi}: {p.km.toLocaleString('id-ID', { maximumFractionDigits: 2 })} km ({p.jumlah} ruas)
              </small>
            ))}
          </div>
        </div>

        <h3 className="dash-section">Fitur per jenis</h3>
        <ul className="dash-list">
          {fitur.map((f) => (
            <li key={f.feature_type}>
              <span className="dash-dot" style={{ background: JENIS[f.feature_type]?.color ?? '#6b7280' }} />
              <span>{JENIS[f.feature_type]?.label ?? f.feature_type}</span>
              <strong>{f.jumlah.toLocaleString('id-ID')}</strong>
            </li>
          ))}
          {data.data && !data.data.bisa_melihat_sr && (
            <li className="dash-note">Sambungan rumah hanya terlihat untuk editor dan admin.</li>
          )}
        </ul>

        {data.data?.bisa_melihat_sr && (
          <>
            <h3 className="dash-section">Impor terakhir</h3>
            {data.data.impor_terakhir.length === 0 ? (
              <p className="dash-note">Belum ada impor.</p>
            ) : (
              <table className="dash-table">
                <thead>
                  <tr>
                    <th>Berkas</th>
                    <th>Oleh</th>
                    <th>Fitur</th>
                    <th>Status</th>
                    <th>Waktu</th>
                  </tr>
                </thead>
                <tbody>
                  {data.data.impor_terakhir.map((i) => (
                    <tr key={i.id}>
                      <td>{i.nama_file}</td>
                      <td>{i.dibuat_oleh}</td>
                      <td>{i.jumlah_fitur ?? '–'}</td>
                      <td>{i.status === 'committed' ? 'Disimpan' : 'Menunggu'}</td>
                      <td>{i.waktu}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}
      </section>
    </div>
  )
}
