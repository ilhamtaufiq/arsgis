/** Nama berkas unduhan gambar, mis. gambar-20261108-1530.geojson (waktu lokal). */
export function exportFileName(now: Date = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `gambar-${now.getFullYear()}${p(now.getMonth() + 1)}${p(now.getDate())}-${p(now.getHours())}${p(now.getMinutes())}.geojson`
}
