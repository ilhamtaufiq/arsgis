import L from 'leaflet'
import { ikonHtml } from './featureIcons'
import type { FeatureType } from './types'

/** Ikon titik untuk peta. Area klik sebesar badge, jadi lebih mudah dikenai daripada lingkaran kecil. */
export function markerIcon(type: FeatureType, terlayani?: boolean | null): L.DivIcon {
  return L.divIcon({
    className: 'fi-marker',
    html: ikonHtml(type, terlayani, 26),
    iconSize: [26, 26],
    iconAnchor: [13, 13],
  })
}
