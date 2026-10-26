import { useEffect, useRef } from 'react'
import L from 'leaflet'
import type { FeatureCollection } from 'geojson'
import 'leaflet/dist/leaflet.css'
import { leafletLayer } from 'protomaps-leaflet'
import { cakupanColor, type DesaGeo, type DesaProps } from './cakupanApi'
import { useMapStore } from './store'
import type { Bbox, PipeFeature } from './types'

// Basemap PMTiles self-hosted (diekstrak dari OSM, disajikan backend di /basemap).
const BASEMAP_URL = '/basemap/basemap-cianjur.pmtiles'
const BASEMAP_ATTRIBUTION =
  '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, <a href="https://protomaps.com">Protomaps</a>'

type Props = {
  features: PipeFeature[]
  /** Poligon desa dengan capaian SPM. Undefined selama belum dimuat. */
  desa?: DesaGeo
  showCakupan: boolean
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

function desaStyle(feature?: GeoJSON.Feature): L.PathOptions {
  const p = feature?.properties as DesaProps | undefined
  return {
    color: '#7c2d12',
    weight: 0.8,
    opacity: 0.5,
    fillColor: cakupanColor(p?.cakupan_persen ?? null),
    fillOpacity: 0.6,
  }
}

export function MapView({ features, desa, showCakupan, onBboxChange }: Props) {
  const container = useRef<HTMLDivElement>(null)
  const mapRef = useRef<L.Map | null>(null)
  const layerRef = useRef<L.GeoJSON | null>(null)
  const desaLayerRef = useRef<L.GeoJSON | null>(null)
  const focus = useMapStore((s) => s.focus)
  const onBboxChangeRef = useRef(onBboxChange)
  onBboxChangeRef.current = onBboxChange

  useEffect(() => {
    if (!container.current) return
    const map = L.map(container.current, { center: [-6.82, 107.14], zoom: 13, zoomControl: false })
    L.control.zoom({ position: 'bottomleft' }).addTo(map)
    leafletLayer({ url: BASEMAP_URL, flavor: 'light', lang: 'id', attribution: BASEMAP_ATTRIBUTION }).addTo(map)

    // Lapisan desa lebih dulu, supaya pipa dan titik selalu tergambar di atasnya.
    const desaLayer = L.geoJSON(undefined, {
      style: desaStyle,
      onEachFeature: (feature, l) => {
        const props = feature.properties as DesaProps
        l.bindTooltip(`${props.nama} · ${props.cakupan_persen === null ? 'tanpa target' : `${props.cakupan_persen}%`}`, {
          sticky: true,
          direction: 'top',
        })
        l.on('click', (e) => {
          L.DomEvent.stopPropagation(e)
          useMapStore.getState().selectDesa(props)
        })
        l.on('mouseover', () => (l as L.Path).setStyle({ weight: 2, opacity: 1 }))
        l.on('mouseout', () => (l as L.Path).setStyle({ weight: 0.8, opacity: 0.5 }))
      },
    })

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

    map.on('click', () => {
      useMapStore.getState().select(null)
      useMapStore.getState().selectDesa(null)
    })
    map.on('moveend', () => onBboxChangeRef.current(toBbox(map)))
    onBboxChangeRef.current(toBbox(map))

    mapRef.current = map
    layerRef.current = layer
    desaLayerRef.current = desaLayer
    return () => {
      map.remove()
      mapRef.current = null
      layerRef.current = null
      desaLayerRef.current = null
    }
  }, [])

  useEffect(() => {
    const map = mapRef.current
    const desaLayer = desaLayerRef.current
    if (!map || !desaLayer) return
    if (showCakupan) {
      if (!map.hasLayer(desaLayer)) desaLayer.addTo(map)
      // Pastikan pipa tetap di atas poligon setelah lapisan desa ditambahkan ulang.
      layerRef.current?.bringToFront()
    } else {
      desaLayer.remove()
    }
  }, [showCakupan])

  useEffect(() => {
    const desaLayer = desaLayerRef.current
    if (!desaLayer) return
    desaLayer.clearLayers()
    if (desa) desaLayer.addData(desa as FeatureCollection)
    layerRef.current?.bringToFront()
  }, [desa])

  useEffect(() => {
    const map = mapRef.current
    if (!map || !focus) return
    // Titik tunggal menghasilkan batas nol, jadi dibatasi zoom maksimum agar tidak terlalu dekat.
    map.fitBounds(L.geoJSON(focus).getBounds(), { maxZoom: 17, padding: [60, 60] })
  }, [focus])

  useEffect(() => {
    const layer = layerRef.current
    if (!layer) return
    layer.clearLayers()
    const collection: FeatureCollection = { type: 'FeatureCollection', features }
    layer.addData(collection)
  }, [features])

  return <div ref={container} className="map" />
}
