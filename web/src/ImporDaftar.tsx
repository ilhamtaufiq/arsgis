import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { batalkanImport, fetchImports, type BatchImpor } from './importApi'
import {
  bisaDibatalkan,
  formatWaktu,
  labelStatusBatch,
  pesanBatalSukses,
  pesanGalatBatal,
  pesanKonfirmasiBatal,
} from './importLogic'
import { KonfirmasiDialog } from './KonfirmasiDialog'

/** Daftar batch impor. Batch yang dibatalkan tampil redup dan tidak bisa dibatalkan lagi. */
export function ImporDaftar() {
  const qc = useQueryClient()
  const daftar = useQuery({ queryKey: ['impor-daftar'], queryFn: fetchImports })
  const [target, setTarget] = useState<BatchImpor | null>(null)
  const [pesan, setPesan] = useState<string | null>(null)
  const [galat, setGalat] = useState<string | null>(null)

  const batal = useMutation({
    mutationFn: (id: number) => batalkanImport(id),
    onSuccess: (jumlah) => {
      setPesan(pesanBatalSukses(jumlah))
      setGalat(null)
      setTarget(null)
      qc.invalidateQueries({ queryKey: ['impor-daftar'] })
      qc.invalidateQueries({ queryKey: ['features'] })
      qc.invalidateQueries({ queryKey: ['aset-terhapus'] })
    },
    onError: (err) => {
      setPesan(null)
      setGalat(pesanGalatBatal(err))
      setTarget(null)
      qc.invalidateQueries({ queryKey: ['impor-daftar'] })
    },
  })

  return (
    <div className="impor-card">
      <h2 className="impor-judul">Riwayat impor</h2>
      {daftar.isPending && <p className="impor-muted">Memuat daftar impor…</p>}
      {daftar.isError && <p role="alert" className="panel-error">{(daftar.error as Error).message}</p>}
      {daftar.data && daftar.data.length === 0 && <p className="impor-muted">Belum ada impor.</p>}
      {daftar.data && daftar.data.length > 0 && (
        <div className="impor-tabel-wrap">
          <table className="impor-tabel">
            <thead>
              <tr>
                <th scope="col">Berkas</th>
                <th scope="col">Paket</th>
                <th scope="col">Oleh</th>
                <th scope="col">Waktu</th>
                <th scope="col">Status</th>
                <th scope="col"><span className="sr-only">Aksi</span></th>
              </tr>
            </thead>
            <tbody>
              {daftar.data.map((b) => (
                <tr key={b.id} className={b.status === 'dibatalkan' ? 'impor-redup' : undefined}>
                  <td className="impor-nama">{b.nama_file}</td>
                  <td>{b.pekerjaan_nama ?? '–'}</td>
                  <td>{b.dibuat_oleh}</td>
                  <td>{formatWaktu(b.created_at)}</td>
                  <td>{labelStatusBatch(b.status)}</td>
                  <td>
                    {bisaDibatalkan(b) && (
                      <button type="button" className="impor-btn danger" onClick={() => setTarget(b)} disabled={batal.isPending}>
                        Batalkan impor
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {pesan && <p role="status">{pesan}</p>}
      {galat && <p role="alert" className="panel-error">{galat}</p>}

      {target && (
        <KonfirmasiDialog
          judul="Batalkan impor?"
          pesan={pesanKonfirmasiBatal(target)}
          labelAksi="Ya, batalkan impor"
          nada="bahaya"
          sibuk={batal.isPending}
          onBatal={() => setTarget(null)}
          onKonfirmasi={() => batal.mutate(target.id)}
        />
      )}
    </div>
  )
}
