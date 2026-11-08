import type { CatalogLayer } from './catalogApi'
import { useMapStore } from './store'

/**
 * Status nyala dan sakelar untuk satu layer katalog. Saat ini hanya layer internal yang dikenali;
 * layer lain tidak ditampilkan (lihat fetchCatalog), jadi cabang terakhir hanya pengaman.
 */
export function useLayerToggle(layer: CatalogLayer) {
  const cakupanOpen = useMapStore((s) => s.cakupanOpen)
  const toggleCakupan = useMapStore((s) => s.toggleCakupan)
  if (layer.internalKey === 'cakupan-spm') return { on: cakupanOpen, toggle: toggleCakupan }
  return { on: false, toggle: () => {} }
}
