import type { ReactNode } from 'react'

export type StatusTipe = 'kosong' | 'memuat' | 'gagal'

export interface StatusStateProps {
  /** kosong: belum ada data. memuat: data sedang diambil. gagal: permintaan gagal. */
  tipe: StatusTipe
  judul: string
  deskripsi?: string
  /** Tombol atau tautan lanjutan, misalnya "Coba lagi". */
  aksi?: ReactNode
}

/**
 * Keadaan kosong, memuat, dan gagal dalam satu bentuk. Keadaan memuat memakai teks, bukan spinner saja.
 * Keadaan gagal memakai role="alert" agar pembaca layar langsung mengumumkannya.
 */
export function StatusState({ tipe, judul, deskripsi, aksi }: StatusStateProps) {
  const role = tipe === 'gagal' ? 'alert' : tipe === 'memuat' ? 'status' : undefined

  return (
    <div className={`status-state status-state-${tipe}`} role={role} data-tipe={tipe}>
      <p className="status-state-judul">{judul}</p>
      {deskripsi && <p className="status-state-deskripsi">{deskripsi}</p>}
      {aksi && <div className="status-state-aksi">{aksi}</div>}
    </div>
  )
}
