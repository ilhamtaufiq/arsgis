import { useQuery } from '@tanstack/react-query'
import { rehabPrioritas } from './omApi'
import { LABEL_KONDISI, LABEL_STATUS_REHAB } from './omLogic'
import './om.css'

/** Daftar prioritas rehabilitasi satu jaringan. Penandaan dilakukan dari panel detail aset di peta. */
export function RehabPanel({ networkId }: { networkId: number }) {
  const daftar = useQuery({
    queryKey: ['om', 'rehab-prioritas', networkId],
    queryFn: () => rehabPrioritas(networkId),
  })

  return (
    <div className="om-panel">
      <p className="om-muted">
        Diurutkan dari angka prioritas terkecil, lalu kondisi paling parah. Aset rusak yang belum ditandai juga muncul.
        Penandaan dilakukan dari detail aset di peta.
      </p>
      {daftar.isPending && <p className="om-muted">Memuat daftar…</p>}
      {daftar.isError && <p role="alert" className="om-error">{(daftar.error as Error).message}</p>}
      {daftar.data && daftar.data.length === 0 && <p className="om-muted">Tidak ada aset yang perlu rehabilitasi.</p>}
      {daftar.data && daftar.data.length > 0 && (
        <table className="om-tabel">
          <thead>
            <tr>
              <th>No</th>
              <th>Aset</th>
              <th>Jenis</th>
              <th>Kondisi</th>
              <th>Status</th>
              <th>Prioritas</th>
            </tr>
          </thead>
          <tbody>
            {daftar.data.map((b, i) => (
              <tr key={b.feature_id} data-status={b.status_rehab}>
                <td>{i + 1}</td>
                <td>{b.nama_aset ?? `Aset #${b.feature_id}`}</td>
                <td>{b.feature_type}</td>
                <td>{b.kondisi ? (LABEL_KONDISI[b.kondisi] ?? b.kondisi) : 'Belum dicatat'}</td>
                <td>{LABEL_STATUS_REHAB[b.status_rehab]}</td>
                <td>{b.prioritas ?? '–'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}
