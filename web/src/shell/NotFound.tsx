import { Button } from '@/components/ui/button'
import { Tautan } from './navigasi'

/** Ditampilkan untuk path yang tidak dikenal. Tautan kembali membawa ke peta. */
export function HalamanTidakDitemukan() {
  return (
    <div className="gate">
      <div className="gate-card">
        <p className="gate-title">Halaman tidak ditemukan</p>
        <p className="gate-sub">Alamat yang Anda buka tidak ada di arsgis. Periksa kembali alamatnya.</p>
        <Button asChild className="justify-self-start">
          <Tautan href="/peta">Kembali ke peta</Tautan>
        </Button>
      </div>
    </div>
  )
}
