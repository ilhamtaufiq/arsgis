import { useEffect, useRef } from 'react'
import L from 'leaflet'
import type { FeatureCollection } from 'geojson'
import 'leaflet/dist/leaflet.css'
import { leafletLayer } from 'protomaps-leaflet'
import { cakupanColor, type DesaGeo, type DesaProps } from './cakupanApi'
import { FEATURE_META } from './featureIcons'
import { markerIcon } from './featureMarker'
import { attachAsetEditing } from './asetEditing'
import { attachDrawing } from './drawing'
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
  /** Desa unit yang dipilih. Peta dipusatkan ke batas desa ini saat unit berganti. */
  focusDesaId?: number | null
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

/** Teks tooltip fitur: nama (atau jenis bila tanpa nama) dan jenisnya. Nama dari input pengguna, di-escape. */
function tooltipFitur(p: PipeFeature['properties']): string {
  const esc = (t: string) => t.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`)
  const jenis = FEATURE_META[p.feature_type].label
  const nama = p.nama ? esc(p.nama) : esc(jenis)
  return p.nama ? `<strong>${nama}</strong><br/><small>${esc(jenis)}</small>` : `<strong>${nama}</strong>`
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

export function MapView({ features, desa, showCakupan, focusDesaId, onBboxChange }: Props) {
  const container = useRef<HTMLDivElement>(null)
  const mapRef = useRef<L.Map | null>(null)
  const layerRef = useRef<L.GeoJSON | null>(null)
  // Lapisan klik tak terlihat di atas pipa: area sentuh pipa jauh lebih lebar dari garisnya.
  const hitRef = useRef<L.GeoJSON | null>(null)
  const desaLayerRef = useRef<L.GeoJSON | null>(null)
  const focus = useMapStore((s) => s.focus)
  const onBboxChangeRef = useRef(onBboxChange)
  onBboxChangeRef.current = onBboxChange

  useEffect(() => {
    if (!container.current) return
    const map = L.map(container.current, { center: [-6.82, 107.14], zoom: 13, zoomControl: false })
    L.control.zoom({ position: 'bottomright', zoomInTitle: 'Perbesar', zoomOutTitle: 'Perkecil' }).addTo(map)
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

    // Klik pada fitur tidak boleh ikut dianggap klik kosong di peta, dan tidak boleh menutup pilihan lain.
    const pilihFitur = (feature: PipeFeature, e: L.LeafletMouseEvent) => {
      // Saat menambah pipa, klik pada pipa yang ada dipakai sebagai titik jalur baru, jadi diteruskan ke peta.
      if (useMapStore.getState().editor?.kind === 'tambah') return
      L.DomEvent.stopPropagation(e)
      useMapStore.getState().select(feature)
    }

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
      pointToLayer: (feature, latlng) => {
        const p = (feature as PipeFeature).properties
        return L.marker(latlng, { icon: markerIcon(p.feature_type, p.terlayani), keyboard: true })
      },
      onEachFeature: (feature, l) => {
        const p = (feature as PipeFeature).properties
        l.bindTooltip(tooltipFitur(p), { sticky: true, direction: 'top', opacity: 0.95 })
        l.on('click', (e) => pilihFitur(feature as PipeFeature, e))
      },
    }).addTo(map)

    // Pipa tipis sulit dikenai. Lapisan ini memberi area sentuh selebar 14 px, hanya untuk garis.
    const hit = L.geoJSON(undefined, {
      filter: (feature) => feature.geometry.type === 'LineString',
      style: { weight: 14, opacity: 0, lineCap: 'round', lineJoin: 'round' },
      onEachFeature: (feature, l) => {
        const p = (feature as PipeFeature).properties
        l.bindTooltip(tooltipFitur(p), { sticky: true, direction: 'top', opacity: 0.95 })
        l.on('click', (e) => pilihFitur(feature as PipeFeature, e))
      },
    }).addTo(map)

    map.on('click', () => {
      useMapStore.getState().select(null)
      useMapStore.getState().selectDesa(null)
    })
    map.on('moveend', () => onBboxChangeRef.current(toBbox(map)))
    onBboxChangeRef.current(toBbox(map))

    const detachDrawing = attachDrawing(map)
    const detachEditing = attachAsetEditing(map)

    mapRef.current = map
    layerRef.current = layer
    hitRef.current = hit
    desaLayerRef.current = desaLayer
    return () => {
      detachEditing()
      detachDrawing()
      map.remove()
      mapRef.current = null
      layerRef.current = null
      hitRef.current = null
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
      hitRef.current?.bringToFront()
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
    hitRef.current?.bringToFront()
  }, [desa])

  // Unit dipilih: pusatkan ke batas desanya. Dilakukan sekali per pilihan, bukan saat poligon dimuat ulang.
  useEffect(() => {
    const map = mapRef.current
    if (!map || focusDesaId == null || !desa) return
    const poligon = desa.features.find((f) => f.properties.id === focusDesaId)
    if (!poligon) return
    const bounds = L.geoJSON(poligon).getBounds()
    if (bounds.isValid()) map.fitBounds(bounds, { maxZoom: 16, padding: [40, 40] })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusDesaId])

  useEffect(() => {
    const map = mapRef.current
    if (!map || !focus) return
    // Titik tunggal menghasilkan batas nol, jadi dibatasi zoom maksimum agar tidak terlalu dekat.
    map.fitBounds(L.geoJSON(focus).getBounds(), { maxZoom: 17, padding: [60, 60] })
  }, [focus])

  useEffect(() => {
    const layer = layerRef.current
    const hit = hitRef.current
    if (!layer || !hit) return
    layer.clearLayers()
    hit.clearLayers()
    const collection: FeatureCollection = { type: 'FeatureCollection', features }
    layer.addData(collection)
    hit.addData(collection)
    hit.bringToFront()
  }, [features])

  return <div ref={container} className="map" />
}
