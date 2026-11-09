import { useState } from 'react'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { fetchAsetTerhapus, pulihkanAset, type AsetTerhapus } from './asetApi'
import { LABEL_JENIS } from './asetForm'
import { formatWaktu } from './importLogic'
import { KonfirmasiDialog } from './KonfirmasiDialog'
import type { FeatureType } from './types'

const PER_HALAMAN = 20

/** Panel aset terhapus (soft delete). Aset di sini bisa dipulihkan dan kembali tampil di peta. */
export function PanelAsetTerhapus() {
  const qc = useQueryClient()
  const [halaman, setHalaman] = useState(1)
  const [target, setTarget] = useState<AsetTerhapus | null>(null)
  const [pesan, setPesan] = useState<string | null>(null)
  const [galat, setGalat] = useState<string | null>(null)

  const data = useQuery({
    queryKey: ['aset-terhapus', halaman],
    queryFn: () => fetchAsetTerhapus(PER_HALAMAN, halaman),
    placeholderData: keepPreviousData,
  })

  const pulih = useMutation({
    mutationFn: (id: number) => pulihkanAset(id),
    onSuccess: () => {
      setPesan('Aset dipulihkan dan kembali tampil di peta.')
      setGalat(null)
      setTarget(null)
      qc.invalidateQueries({ queryKey: ['aset-terhapus'] })
      qc.invalidateQueries({ queryKey: ['features'] })
    },
    onError: (err) => {
      setPesan(null)
      setGalat((err as Error).message)
      setTarget(null)
    },
  })

  const meta = data.data?.meta
  const baris = data.data?.data ?? []

  return (
    <div className="impor-card">
      <h2 className="impor-judul">Aset terhapus</h2>
      <p className="impor-muted">Aset yang dihapus, termasuk karena pembatalan impor, bisa dipulihkan di sini.</p>
      {data.isPending && <p className="impor-muted">Memuat aset terhapus…</p>}
      {data.isError && <p role="alert" className="panel-error">{(data.error as Error).message}</p>}
      {data.data && baris.length === 0 && <p className="impor-muted">Tidak ada aset terhapus.</p>}
      {baris.length > 0 && (
        <div className="impor-tabel-wrap">
          <table className="impor-tabel">
            <thead>
              <tr>
                <th scope="col">Kode aset</th>
                <th scope="col">Nama</th>
                <th scope="col">Jenis</th>
                <th scope="col">Dihapus</th>
                <th scope="col"><span className="sr-only">Aksi</span></th>
              </tr>
            </thead>
            <tbody>
              {baris.map((a) => (
                <tr key={a.id}>
                  <td>{a.kode_aset ?? '–'}</td>
                  <td className="impor-nama">{a.nama ?? '–'}</td>
                  <td>{LABEL_JENIS[a.jenis as FeatureType] ?? a.jenis}</td>
                  <td>{formatWaktu(a.deleted_at)}</td>
                  <td>
                    <button type="button" className="impor-btn" onClick={() => setTarget(a)} disabled={pulih.isPending}>Pulihkan</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {meta && meta.last_page > 1 && (
        <div className="impor-aksi">
          <button type="button" className="impor-btn" onClick={() => setHalaman((h) => h - 1)} disabled={halaman <= 1 || data.isFetching}>Sebelumnya</button>
          <span className="impor-muted">Halaman {meta.page} dari {meta.last_page}</span>
          <button type="button" className="impor-btn" onClick={() => setHalaman((h) => h + 1)} disabled={halaman >= meta.last_page || data.isFetching}>Berikutnya</button>
        </div>
      )}
      {pesan && <p role="status">{pesan}</p>}
      {galat && <p role="alert" className="panel-error">{galat}</p>}

      {target && (
        <KonfirmasiDialog
          judul="Pulihkan aset?"
          pesan={`Pulihkan "${target.nama ?? target.kode_aset ?? 'aset ini'}"? Aset akan kembali tampil di peta.`}
          labelAksi="Ya, pulihkan"
          nada="utama"
          sibuk={pulih.isPending}
          onBatal={() => setTarget(null)}
          onKonfirmasi={() => pulih.mutate(target.id)}
        />
      )}
    </div>
  )
}
