import L from 'leaflet'
import '@geoman-io/leaflet-geoman-free'
import '@geoman-io/leaflet-geoman-free/dist/leaflet-geoman.css'
import type { FeatureCollection } from 'geojson'
import { exportFileName } from './fileName'
import { useMapStore } from './store'

// Gambar pengguna disimpan di satu grup. Satu peta aktif pada satu waktu, jadi cukup satu grup.
let drawn: L.FeatureGroup | null = null

const DRAW_STYLE: L.PathOptions = { color: '#DF5E3A', weight: 2, fillColor: '#DF5E3A', fillOpacity: 0.15 }

/** Pasang geoman dan grup gambar ke peta. Mengembalikan fungsi pelepas. */
export function attachDrawing(map: L.Map): () => void {
  const group = L.featureGroup().addTo(map)
  drawn = group
  map.pm.setGlobalOptions({ layerGroup: group, pathOptions: DRAW_STYLE, templineStyle: DRAW_STYLE, hintlineStyle: DRAW_STYLE })
  map.pm.setLang('id')

  const sync = () => useMapStore.getState().setDrawnCount(group.getLayers().length)
  const onCreate = () => {
    sync()
    // Selesai satu bentuk, kembali ke mode biasa supaya klik berikutnya tidak menggambar lagi.
    useMapStore.getState().setDrawMode(null)
  }
  map.on('pm:create', onCreate)
  group.on('layerremove', sync)

  // Esc membatalkan gambar yang belum selesai.
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && useMapStore.getState().drawMode) useMapStore.getState().setDrawMode(null)
  }
  document.addEventListener('keydown', onKey)

  // Mode gambar mengikuti store.
  const unsubscribe = useMapStore.subscribe((state, prev) => {
    if (state.drawMode === prev.drawMode) return
    if (state.drawMode === 'rectangle') map.pm.enableDraw('Rectangle')
    else map.pm.disableDraw()
  })

  return () => {
    unsubscribe()
    document.removeEventListener('keydown', onKey)
    map.off('pm:create', onCreate)
    map.pm.disableDraw()
    group.remove()
    drawn = null
  }
}

/** Semua gambar sebagai GeoJSON (EPSG:4326). Kosong bila belum ada gambar. */
export function drawnGeoJson(): FeatureCollection {
  return (drawn?.toGeoJSON() as FeatureCollection | undefined) ?? { type: 'FeatureCollection', features: [] }
}

export function clearDrawn(): void {
  drawn?.clearLayers()
  useMapStore.getState().setDrawnCount(0)
}

/** Unduh gambar sebagai berkas GeoJSON. Mengembalikan jumlah fitur yang diekspor, 0 bila tidak ada. */
export function downloadDrawn(): number {
  const data = drawnGeoJson()
  if (data.features.length === 0) return 0
  const blob = new Blob([JSON.stringify(data)], { type: 'application/geo+json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = exportFileName()
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
  return data.features.length
}
