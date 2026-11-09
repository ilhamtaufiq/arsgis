import { useId } from 'react'

type Props = {
  judul: string
  pesan: string
  labelAksi: string
  nada: 'bahaya' | 'utama'
  sibuk?: boolean
  onBatal: () => void
  onKonfirmasi: () => void
}

/** Dialog konfirmasi modal untuk aksi yang mengubah data. Tombol "Kembali" mendapat fokus dan bisa ditutup dengan Escape. */
export function KonfirmasiDialog({ judul, pesan, labelAksi, nada, sibuk = false, onBatal, onKonfirmasi }: Props) {
  const idJudul = useId()
  const idPesan = useId()
  return (
    <div className="impor-overlay" onKeyDown={(e) => { if (e.key === 'Escape') onBatal() }}>
      <div role="alertdialog" aria-modal="true" aria-labelledby={idJudul} aria-describedby={idPesan} className="impor-dialog">
        <h2 id={idJudul}>{judul}</h2>
        <p id={idPesan}>{pesan}</p>
        <div className="impor-aksi">
          <button type="button" className="impor-btn" onClick={onBatal} disabled={sibuk} autoFocus>Kembali</button>
          <button type="button" className={`impor-btn ${nada}`} onClick={onKonfirmasi} disabled={sibuk}>{labelAksi}</button>
        </div>
      </div>
    </div>
  )
}
