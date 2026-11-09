import { toast } from 'sonner'
import { pesanSukses, type AksiSukses } from './pesan'

/** Toast sukses untuk aksi yang sudah tersimpan di server. */
export function toastSukses(aksi: AksiSukses) {
  toast.success(pesanSukses(aksi))
}

/** Toast gagal dengan pesan error yang sudah ada. Error tanpa pesan memakai teks cadangan. */
export function toastGalat(err: unknown, cadangan = 'Permintaan gagal. Coba lagi.') {
  toast.error(err instanceof Error && err.message ? err.message : cadangan)
}
