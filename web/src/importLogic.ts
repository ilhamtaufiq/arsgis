import { ImportApiError, type BatchImpor, type DuplikatImpor, type StatusBatch } from './importApi'

/** Label status batch untuk tampilan. */
export const LABEL_STATUS_BATCH: Record<StatusBatch, string> = {
  staged: 'Menunggu disimpan',
  committed: 'Disimpan',
  dibatalkan: 'Dibatalkan',
}

export function labelStatusBatch(status: string): string {
  return LABEL_STATUS_BATCH[status as StatusBatch] ?? status
}

/** Tanggal dan jam dalam format Indonesia. Bila tanggal tidak valid, nilai asli dikembalikan apa adanya. */
export function formatWaktu(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString('id-ID', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}

/** Peringatan saat berkas yang diunggah sudah pernah diimpor. */
export function pesanDuplikat(d: DuplikatImpor): string {
  return `Berkas ini sudah pernah diimpor: ${d.nama_file} pada ${formatWaktu(d.created_at)}`
}

/** Batch yang belum dibatalkan masih bisa dibatalkan, termasuk yang sudah disimpan. */
export function bisaDibatalkan(batch: Pick<BatchImpor, 'status'>): boolean {
  return batch.status !== 'dibatalkan'
}

export function daftarBisaDibatalkan<T extends Pick<BatchImpor, 'status'>>(batch: T[]): T[] {
  return batch.filter(bisaDibatalkan)
}

export function pesanKonfirmasiBatal(batch: Pick<BatchImpor, 'nama_file'>): string {
  return `Batalkan impor "${batch.nama_file}"? Semua aset dari impor ini akan dihapus. Aset tersebut bisa dipulihkan dari daftar Aset terhapus.`
}

export function pesanBatalSukses(jumlah: number): string {
  return `Impor dibatalkan. ${jumlah} aset dihapus dan bisa dipulihkan dari daftar Aset terhapus.`
}

/** Pesan galat saat menyimpan batch. 409 berarti berkas ganda dan pengguna belum mengizinkannya. */
export function pesanGalatSimpan(err: unknown): string {
  if (err instanceof ImportApiError && err.status === 409) {
    return 'Berkas ini sudah pernah diimpor. Centang "Impor lagi walaupun ganda" bila tetap ingin menyimpannya.'
  }
  return err instanceof Error ? err.message : 'Simpan gagal.'
}

/** Pesan galat saat membatalkan batch. 409 berarti batch sudah dibatalkan sebelumnya. */
export function pesanGalatBatal(err: unknown): string {
  if (err instanceof ImportApiError && err.status === 409) return 'Impor ini sudah dibatalkan sebelumnya.'
  return err instanceof Error ? err.message : 'Pembatalan gagal.'
}
