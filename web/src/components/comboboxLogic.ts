export type OpsiCombo = { value: string; label: string; hint?: string }

/** Jumlah hasil yang ditampilkan sebelum pengguna mengetik. */
export const BATAS_TAMPIL_DEFAULT = 10

/**
 * Memfilter opsi dengan kata kunci. Setiap kata dalam kata kunci harus cocok (tanpa memperhatikan huruf besar),
 * di label atau petunjuk. Mengembalikan hasil yang tampil dan total semua yang cocok.
 */
export function filterOpsi(
  options: OpsiCombo[],
  query: string,
  limit = BATAS_TAMPIL_DEFAULT,
): { items: OpsiCombo[]; total: number } {
  const kata = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
  const cocok = kata.length === 0
    ? options
    : options.filter((o) => {
        const teks = `${o.label} ${o.hint ?? ''}`.toLowerCase()
        return kata.every((k) => teks.includes(k))
      })
  return { items: cocok.slice(0, limit), total: cocok.length }
}
