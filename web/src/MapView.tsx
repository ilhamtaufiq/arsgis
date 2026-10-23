import { useEffect, useRef } from 'react'
import L from 'leaflet'
import type { FeatureCollection } from 'geojson'
import 'leaflet/dist/leaflet.css'
import { useMapStore } from './store'
import type { Bbox, PipeFeature } from './types'

const OSM_TILES = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png'
const OSM_ATTRIBUTION = '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'

type Props = {
  features: PipeFeature[]
  onBboxChange: (bbox: Bbox) => void
}

function toBbox(map: L.Map): Bbox {
  const b = map.getBounds()
  return { west: b.getWest(), south: b.getSouth(), east: b.getEast(), north: b.getNorth() }
}

// Lebar pipa mengikuti diameter: 15 mm → 1.5 px, 150 mm → 6 px, di luar rentang itu dijepit.
function pipeWeight(diameterMm: number | null) {
  const d = diameterMm ?? 50
  return Math.min(6, Math.max(1.5, 1.5 + ((d - 15) * 4.5) / 135))
}

function pointStyle(type: string): L.CircleMarkerOptions {
  switch (type) {
    case 'valve':
      return { radius: 6, color: '#111111', weight: 2, fillColor: '#ffffff', fillOpacity: 1 }
    case 'reservoir':
      return { radius: 10, color: '#ffffff', weight: 2, fillColor: '#0f766e', fillOpacity: 1 }
    case 'bronkaptering':
      return { radius: 9, color: '#ffffff', weight: 2, fillColor: '#0891b2', fillOpacity: 1 }
    default:
      // Sambungan rumah (sr)
      return { radius: 4, stroke: false, fillColor: '#a16207', fillOpacity: 1 }
  }
}

export function MapView({ features, onBboxChange }: Props) {
  const container = useRef<HTMLDivElement>(null)
  const layerRef = useRef<L.GeoJSON | null>(null)
  const onBboxChangeRef = useRef(onBboxChange)
  onBboxChangeRef.current = onBboxChange

  useEffect(() => {
    if (!container.current) return
    const map = L.map(container.current, { center: [-6.82, 107.14], zoom: 13 })
    L.tileLayer(OSM_TILES, { maxZoom: 19, attribution: OSM_ATTRIBUTION }).addTo(map)

    const layer = L.geoJSON(undefined, {
      style: (feature) => {
        const p = (feature as PipeFeature).properties
        return {
          color: p.kondisi === 'perlu cek' ? '#d9603b' : '#1d4ed8',
          weight: pipeWeight(p.diameter_mm),
          lineCap: 'round',
          lineJoin: 'round',
        }
      },
      pointToLayer: (feature, latlng) => L.circleMarker(latlng, pointStyle((feature as PipeFeature).properties.feature_type)),
      onEachFeature: (feature, l) => {
        l.on('click', (e) => {
          // Klik pada fitur tidak boleh ikut dianggap klik kosong di peta.
          L.DomEvent.stopPropagation(e)
          useMapStore.getState().select(feature as PipeFeature)
        })
      },
    }).addTo(map)

    map.on('click', () => useMapStore.getState().select(null))
    map.on('moveend', () => onBboxChangeRef.current(toBbox(map)))
    onBboxChangeRef.current(toBbox(map))

    layerRef.current = layer
    return () => {
      map.remove()
      layerRef.current = null
    }
  }, [])

  useEffect(() => {
    const layer = layerRef.current
    if (!layer) return
    layer.clearLayers()
    const collection: FeatureCollection = { type: 'FeatureCollection', features }
    layer.addData(collection)
  }, [features])

  return <div ref={container} className="map" />
}
