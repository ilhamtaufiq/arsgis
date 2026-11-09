import { useEffect, useRef } from 'react'
import L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import { leafletLayer } from 'protomaps-leaflet'

const BASEMAP_URL = '/basemap/basemap-cianjur.pmtiles'
const BASEMAP_ATTRIBUTION =
  '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, <a href="https://protomaps.com">Protomaps</a>'

// Pusat awal peta sama dengan MapView: Cianjur. Dipakai hanya bila belum ada titik yang dipilih.
const PUSAT_AWAL: [number, number] = [-6.82, 107.14]

interface Props {
  lon: number | null
  lat: number | null
  onPilih: (lon: number, lat: number) => void
}

/**
 * Peta kecil untuk memilih lokasi laporan dengan klik. Titik juga bisa diubah lewat kolom Lon dan Lat;
 * marker mengikuti nilai di kolom itu.
 */
export function LokasiPicker({ lon, lat, onPilih }: Props) {
  const wadah = useRef<HTMLDivElement | null>(null)
  const peta = useRef<L.Map | null>(null)
  const marker = useRef<L.CircleMarker | null>(null)
  // Simpan callback terbaru tanpa membuat ulang peta setiap render.
  const onPilihRef = useRef(onPilih)
  onPilihRef.current = onPilih

  useEffect(() => {
    if (!wadah.current || peta.current) return
    const map = L.map(wadah.current, { center: PUSAT_AWAL, zoom: 13, zoomControl: true })
    // Basemap yang sama dengan halaman peta, disajikan dari server sendiri agar tetap jalan saat sinyal lemah.
    leafletLayer({ url: BASEMAP_URL, flavor: 'light', lang: 'id', attribution: BASEMAP_ATTRIBUTION }).addTo(map)
    map.on('click', (e: L.LeafletMouseEvent) => {
      onPilihRef.current(Number(e.latlng.lng.toFixed(6)), Number(e.latlng.lat.toFixed(6)))
    })
    peta.current = map
    return () => {
      map.remove()
      peta.current = null
      marker.current = null
    }
  }, [])

  useEffect(() => {
    const map = peta.current
    if (!map) return
    if (lon === null || lat === null || !Number.isFinite(lon) || !Number.isFinite(lat)) {
      marker.current?.remove()
      marker.current = null
      return
    }
    const posisi: L.LatLngExpression = [lat, lon]
    if (marker.current) {
      marker.current.setLatLng(posisi)
    } else {
      marker.current = L.circleMarker(posisi, { radius: 8, color: '#b91c1c', weight: 2, fillOpacity: 0.7 }).addTo(map)
    }
  }, [lon, lat])

  return (
    <div className="om-picker">
      <div ref={wadah} className="om-picker-peta" role="application" aria-label="Pilih lokasi dengan klik pada peta" />
      <p className="om-picker-catatan">Klik peta untuk menaruh titik lokasi. Lon dan Lat terisi otomatis.</p>
    </div>
  )
}
