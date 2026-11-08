import { useQuery } from '@tanstack/react-query'
import { Map as MapIcon } from 'lucide-react'
import { chipLayers, fetchCatalog, isNew, todayIso, type CatalogLayer } from './catalogApi'
import { useLayerToggle } from './useLayerToggle'

function Chip({ layer }: { layer: CatalogLayer }) {
  const { on, toggle } = useLayerToggle(layer)
  const baru = isNew(layer, todayIso())
  return (
    <button type="button" className="layer-chip" aria-pressed={on} onClick={toggle}>
      <MapIcon size={14} aria-hidden="true" />
      <span>{layer.nama}</span>
      {baru && (
        <span className="chip-new">
          <span aria-hidden="true">BARU</span>
          <span className="sr-only">baru</span>
        </span>
      )}
    </button>
  )
}

/** Baris chip layer cepat di bawah kapsul pencarian. Kosong atau gagal dimuat berarti baris tidak tampil. */
export function LayerChips() {
  const catalog = useQuery({ queryKey: ['catalog'], queryFn: fetchCatalog, staleTime: 5 * 60_000 })
  const chips = chipLayers(catalog.data ?? [])
  if (chips.length === 0) return null
  return (
    <div className="chip-row" role="group" aria-label="Layer cepat">
      {chips.map((l) => (
        <Chip key={l.slug} layer={l} />
      ))}
    </div>
  )
}
