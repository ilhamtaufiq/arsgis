import { useEffect, useRef } from 'react'
import maplibregl from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'
import type { StyleSpecification } from 'maplibre-gl'
import { useMapStore } from './store'
import type { Bbox, PipeFeature } from './types'

// Basemap sementara: latar polos tanpa tile pihak ketiga. PMTiles self-hosted menyusul (lihat rencana).
const STYLE: StyleSpecification = {
  version: 8,
  sources: {},
  layers: [{ id: 'background', type: 'background', paint: { 'background-color': '#eef1f4' } }],
}

const SOURCE = 'jaringan'

type Props = {
  features: PipeFeature[]
  onBboxChange: (bbox: Bbox) => void
}

function toBbox(map: maplibregl.Map): Bbox {
  const b = map.getBounds()
  return { west: b.getWest(), south: b.getSouth(), east: b.getEast(), north: b.getNorth() }
}

export function MapView({ features, onBboxChange }: Props) {
  const container = useRef<HTMLDivElement>(null)
  const mapRef = useRef<maplibregl.Map | null>(null)
  const onBboxChangeRef = useRef(onBboxChange)
  onBboxChangeRef.current = onBboxChange

  useEffect(() => {
    if (!container.current) return
    const map = new maplibregl.Map({
      container: container.current,
      style: STYLE,
      center: [107.14, -6.82],
      zoom: 13,
      attributionControl: false,
    })
    map.addControl(new maplibregl.NavigationControl({ showCompass: false }), 'top-right')
    mapRef.current = map

    map.on('load', () => {
      map.addSource(SOURCE, { type: 'geojson', data: { type: 'FeatureCollection', features: [] } })

      map.addLayer({
        id: 'pipa',
        type: 'line',
        source: SOURCE,
        filter: ['==', ['get', 'feature_type'], 'pipa'],
        layout: { 'line-cap': 'round', 'line-join': 'round' },
        paint: {
          'line-color': ['match', ['get', 'kondisi'], 'perlu cek', '#d9603b', '#1d4ed8'],
          'line-width': ['interpolate', ['linear'], ['coalesce', ['get', 'diameter_mm'], 50], 15, 1.5, 150, 6],
        },
      })
      map.addLayer({
        id: 'valve',
        type: 'circle',
        source: SOURCE,
        filter: ['==', ['get', 'feature_type'], 'valve'],
        paint: { 'circle-radius': 6, 'circle-color': '#ffffff', 'circle-stroke-color': '#111111', 'circle-stroke-width': 2 },
      })
      map.addLayer({
        id: 'reservoir',
        type: 'circle',
        source: SOURCE,
        filter: ['==', ['get', 'feature_type'], 'reservoir'],
        paint: { 'circle-radius': 10, 'circle-color': '#0f766e', 'circle-stroke-color': '#ffffff', 'circle-stroke-width': 2 },
      })
      map.addLayer({
        id: 'bronkaptering',
        type: 'circle',
        source: SOURCE,
        filter: ['==', ['get', 'feature_type'], 'bronkaptering'],
        paint: { 'circle-radius': 9, 'circle-color': '#0891b2', 'circle-stroke-color': '#ffffff', 'circle-stroke-width': 2 },
      })
      map.addLayer({
        id: 'sr',
        type: 'circle',
        source: SOURCE,
        filter: ['==', ['get', 'feature_type'], 'sr'],
        paint: { 'circle-radius': 4, 'circle-color': '#a16207' },
      })

      onBboxChangeRef.current(toBbox(map))
    })

    map.on('moveend', () => {
      if (map.isStyleLoaded()) onBboxChangeRef.current(toBbox(map))
    })

    const pickable = ['pipa', 'valve', 'reservoir', 'bronkaptering', 'sr']
    map.on('click', (e) => {
      const hits = map.queryRenderedFeatures(e.point, { layers: pickable })
      const hit = hits[0]
      if (!hit) {
        useMapStore.getState().select(null)
        return
      }
      useMapStore.getState().select({
        type: 'Feature',
        id: Number(hit.id ?? hit.properties?.id),
        geometry: hit.geometry,
        properties: hit.properties as PipeFeature['properties'],
      } as PipeFeature)
    })
    map.on('mouseenter', pickable[0], () => (map.getCanvas().style.cursor = 'pointer'))
    map.on('mouseleave', pickable[0], () => (map.getCanvas().style.cursor = ''))

    return () => {
      map.remove()
      mapRef.current = null
    }
  }, [])

  useEffect(() => {
    const map = mapRef.current
    if (!map || !map.isStyleLoaded()) return
    const source = map.getSource<maplibregl.GeoJSONSource>(SOURCE)
    source?.setData({ type: 'FeatureCollection', features })
  }, [features])

  return <div ref={container} className="map" />
}
