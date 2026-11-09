import { useEffect, useId, useRef } from 'react'
import { Button } from '@/components/ui/button'

type Props = {
  open: boolean
  judul: string
  pesan: string
  /** Teks tombol aksi utama, misalnya "Ya, kembalikan". */
  labelAksi: string
  /** `bahaya` untuk aksi yang menghapus atau membatalkan data. */
  nada?: 'bahaya' | 'utama'
  sibuk?: boolean
  onBatal: () => void
  onKonfirmasi: () => void
}

/**
 * Dialog konfirmasi modal untuk aksi yang mengubah atau menghapus data. Fokus awal di "Batal",
 * Escape menutup dialog, dan fokus kembali ke elemen yang membukanya.
 */
export function ConfirmDialog({ open, judul, pesan, labelAksi, nada = 'utama', sibuk = false, onBatal, onKonfirmasi }: Props) {
  const idJudul = useId()
  const idPesan = useId()
  const batalRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    if (!open) return
    const pemicu = document.activeElement as HTMLElement | null
    batalRef.current?.focus()
    return () => pemicu?.focus?.()
  }, [open])

  useEffect(() => {
    if (!open) return
    function tekan(e: KeyboardEvent) {
      if (e.key === 'Escape' && !sibuk) onBatal()
    }
    window.addEventListener('keydown', tekan)
    return () => window.removeEventListener('keydown', tekan)
  }, [open, sibuk, onBatal])

  if (!open) return null

  return (
    <div className="fixed inset-0 z-[1000] grid place-items-center bg-black/40 p-4">
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={idJudul}
        aria-describedby={idPesan}
        className="grid w-[min(420px,100%)] gap-3 rounded-lg border bg-background p-5 text-sm text-foreground"
      >
        <h2 id={idJudul} className="text-base font-semibold">{judul}</h2>
        <p id={idPesan} className="text-muted-foreground">{pesan}</p>
        <div className="flex flex-wrap justify-end gap-2">
          <Button ref={batalRef} type="button" variant="outline" onClick={onBatal} disabled={sibuk}>
            Batal
          </Button>
          <Button
            type="button"
            variant={nada === 'bahaya' ? 'destructive' : 'default'}
            onClick={onKonfirmasi}
            disabled={sibuk}
          >
            {sibuk ? 'Memproses…' : labelAksi}
          </Button>
        </div>
      </div>
    </div>
  )
}
