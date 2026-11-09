import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { catatJadwalSelesai, jatuhTempo, OmApiError } from './omApi'
import { formatTanggal, LABEL_JENIS_PERAWATAN, teksJatuhTempo } from './omLogic'
import './om.css'

const PILIHAN_JENDELA = [7, 14, 30, 90]

type Props = {
  networkId: number
  bisaCatat: boolean
  /** Mode ringkas untuk kartu dashboard: tanpa pemilih jendela dan tombol, maksimal 10 baris. */
  ringkas?: boolean
}

/** Daftar jadwal perawatan yang sudah lewat atau jatuh tempo dalam N hari. */
export function JatuhTempoPanel({ networkId, bisaCatat, ringkas = false }: Props) {
  const [hari, setHari] = useState(30)
  const qc = useQueryClient()
  const jendela = ringkas ? 30 : hari
  const daftar = useQuery({
    queryKey: ['om', 'jatuh-tempo', networkId, jendela],
    queryFn: () => jatuhTempo(networkId, jendela),
  })
  const catat = useMutation({
    mutationFn: (jadwalId: number) => catatJadwalSelesai(jadwalId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['om'] }),
  })

  const baris = daftar.data ?? []
  const tampil = ringkas ? baris.slice(0, 10) : baris

  return (
    <div className="om-panel">
      {!ringkas && (
        <label className="om-field om-inline">
          Jatuh tempo dalam
          <select value={hari} onChange={(e) => setHari(Number(e.target.value))}>
            {PILIHAN_JENDELA.map((n) => (
              <option key={n} value={n}>
                {n} hari
              </option>
            ))}
          </select>
        </label>
      )}
      {daftar.isPending && <p className="om-muted">Memuat jadwal…</p>}
      {daftar.isError && <p role="alert" className="om-error">{(daftar.error as Error).message}</p>}
      {catat.isError && <p role="alert" className="om-error">{(catat.error as OmApiError).message}</p>}
      {daftar.data && tampil.length === 0 && <p className="om-muted">Tidak ada perawatan yang jatuh tempo.</p>}
      {tampil.length > 0 && (
        <table className="om-tabel">
          <thead>
            <tr>
              <th>Aset</th>
              <th>Perawatan</th>
              <th>Tanggal</th>
              <th>Status</th>
              {bisaCatat && !ringkas && <th aria-label="Aksi" />}
            </tr>
          </thead>
          <tbody>
            {tampil.map((b) => (
              <tr key={b.jadwal_id} data-status={b.status}>
                <td>{b.nama_aset ?? `Aset #${b.feature_id}`}</td>
                <td>{LABEL_JENIS_PERAWATAN[b.jenis] ?? b.jenis} · tiap {b.interval_hari} hari</td>
                <td>{formatTanggal(b.tanggal_berikutnya)}</td>
                <td>
                  <span className={`om-badge om-badge-${b.status}`}>{teksJatuhTempo(b.status, b.selisih_hari)}</span>
                </td>
                {bisaCatat && !ringkas && (
                  <td>
                    <button type="button" onClick={() => catat.mutate(b.jadwal_id)} disabled={catat.isPending}>
                      Catat selesai
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}

/** Kartu jatuh tempo untuk dashboard unit SPAM. */
export function JatuhTempoCard({ networkId, bisaCatat }: { networkId: number; bisaCatat: boolean }) {
  return (
    <Card className="min-w-0">
      <CardHeader>
        <CardTitle>Perawatan jatuh tempo</CardTitle>
        <CardDescription>Jadwal yang sudah lewat atau jatuh tempo dalam 30 hari</CardDescription>
      </CardHeader>
      <CardContent>
        <JatuhTempoPanel networkId={networkId} bisaCatat={bisaCatat} ringkas />
        <a className="om-link" href={`/om?unit=${networkId}`}>
          Buka operasi dan pemeliharaan →
        </a>
      </CardContent>
    </Card>
  )
}
