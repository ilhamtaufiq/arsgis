import L from 'leaflet'
import '@geoman-io/leaflet-geoman-free'
import type { FeatureCollection, Geometry, LineString, MultiLineString } from 'geojson'
import { geometriUntuk } from './asetForm'
import { ASET_COLOR, isAsetBaru } from './asetStyle'
import { useMapStore, type ModeEdit } from './store'
import type { PipeFeature } from './types'

// Mode edit di peta. Satu peta aktif pada satu waktu, jadi state modul cukup (sama seperti drawing.ts).
// Klik dan seret diterjemahkan ke geometri di sini. Penyimpanan dilakukan komponen React lewat API.

let map: L.Map | null = null
let previewLayer: L.LayerGroup | null = null
let pindahLayer: L.GeoJSON | null = null

const PIPA_STYLE: L.PathOptions = { color: '#1d4ed8', weight: 4, lineCap: 'round', lineJoin: 'round' }
const PREVIEW_STYLE: L.PathOptions = { color: '#1d4ed8', weight: 3, dashArray: '6 6' }

function warnaTitik(jenis: PipeFeature['properties']['feature_type']): string {
  return isAsetBaru(jenis) ? ASET_COLOR[jenis] : '#0f766e'
}

function ikonSeret(jenis: PipeFeature['properties']['feature_type']): L.DivIcon {
  return L.divIcon({
    className: 'aset-seret',
    iconSize: [22, 22],
    iconAnchor: [11, 11],
    html: `<span style="display:block;width:16px;height:16px;border-radius:50%;background:${warnaTitik(jenis)};border:3px solid #fff;box-shadow:0 0 0 2px #111"></span>`,
  })
}

/** Pasang mode edit ke peta. Mengembalikan fungsi pelepas. */
export function attachAsetEditing(m: L.Map): () => void {
  map = m
  previewLayer = L.layerGroup().addTo(m)

  const unsubscribe = useMapStore.subscribe((s, prev) => {
    if (s.editor !== prev.editor) gantiMode(s.editor, prev.editor)
  })

  const onClick = (e: L.LeafletMouseEvent) => {
    const store = useMapStore.getState()
    const ed = store.editor
    if (ed?.kind !== 'tambah') return
    const ll: [number, number] = [e.latlng.lng, e.latlng.lat]
    if (geometriUntuk(ed.jenis) === 'Point') {
      // Titik langsung membuka form, tanpa langkah tambahan.
      store.setEditor(null)
      store.setFormulir({ kind: 'buat', jenis: ed.jenis, geometry: { type: 'Point', coordinates: ll } })
      return
    }
    store.setEditor({ ...ed, titik: [...ed.titik, ll] })
  }

  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && useMapStore.getState().editor) useMapStore.getState().setEditor(null)
  }

  m.on('click', onClick)
  document.addEventListener('keydown', onKey)

  return () => {
    unsubscribe()
    m.off('click', onClick)
    document.removeEventListener('keydown', onKey)
    stopPindah()
    previewLayer?.remove()
    previewLayer = null
    m.getContainer().style.cursor = ''
    map = null
  }
}

function gantiMode(next: ModeEdit, prev: ModeEdit) {
  if (!map) return

  if (prev?.kind === 'pindah' && next?.kind !== 'pindah') stopPindah()
  if (next?.kind === 'pindah' && (prev?.kind !== 'pindah' || prev.feature.id !== next.feature.id)) {
    startPindah(next.feature)
  }

  map.getContainer().style.cursor = next?.kind === 'tambah' ? 'crosshair' : ''

  previewLayer?.clearLayers()
  if (next?.kind === 'tambah' && next.titik.length > 0) {
    const latlngs = next.titik.map(([lng, lat]) => L.latLng(lat, lng))
    previewLayer?.addLayer(L.polyline(latlngs, PREVIEW_STYLE))
    for (const ll of latlngs) {
      previewLayer?.addLayer(L.circleMarker(ll, { radius: 4, color: '#1d4ed8', weight: 2, fillColor: '#ffffff', fillOpacity: 1 }))
    }
  }
}

function startPindah(feature: PipeFeature) {
  stopPindah()
  if (!map) return
  const jenis = feature.properties.feature_type
  const titik = feature.geometry.type === 'Point'
  pindahLayer = L.geoJSON(feature.geometry, {
    style: () => PIPA_STYLE,
    pointToLayer: (_f, ll) => L.marker(ll, { draggable: true, icon: ikonSeret(jenis), zIndexOffset: 1000 }),
  }).addTo(map)
  if (!titik) {
    // Garis diedit dengan geoman: titik ujung dan simpul bisa diseret.
    pindahLayer.eachLayer((l) => {
      const pm = (l as unknown as { pm?: { enable: () => void } }).pm
      pm?.enable()
    })
  }
}

function stopPindah() {
  pindahLayer?.remove()
  pindahLayer = null
}

/** Geometri hasil seretan saat ini, atau null bila mode pindah belum aktif. */
export function geometriPindah(): Geometry | null {
  if (!pindahLayer) return null
  const fc = pindahLayer.toGeoJSON() as FeatureCollection
  const fitur = fc.features
  if (fitur.length === 0) return null
  if (fitur.length === 1) return fitur[0].geometry
  // Beberapa bagian garis digabung kembali menjadi MultiLineString.
  const bagian = fitur
    .map((f) => f.geometry)
    .filter((g): g is LineString => g.type === 'LineString')
    .map((g) => g.coordinates)
  const hasil: MultiLineString = { type: 'MultiLineString', coordinates: bagian }
  return hasil
}
