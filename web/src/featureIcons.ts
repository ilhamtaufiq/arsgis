import { ASET_COLOR, ASET_LABEL, type AsetBaru } from './asetStyle'
import type { FeatureType } from './types'

/**
 * Satu sumber untuk label, warna, dan ikon tiap jenis fitur. Dipakai peta, legenda, pencarian, dan panel info
 * supaya satu jenis selalu tampil sama di semua tempat.
 *
 * Ikon digambar sebagai garis (viewBox 24, goresan 2) di dalam kotak berwarna. Tidak memakai emoji.
 */

export type FeatureMeta = {
  label: string
  color: string
  /** Isi SVG (path, circle, dll.) tanpa elemen `<svg>` pembungkus. */
  icon: string
}

const ASET_ICON: Record<AsetBaru, string> = {
  // Bak penahan tekanan: bak persegi dengan manometer di tengah.
  bak_penahan_tekanan: '<rect x="5" y="5" width="14" height="14" rx="1.5"/><path d="M9 12h6M12 9v3"/>',
  // Menara air: tangki di atas rangka.
  menara_air: '<path d="M8 4h8l1.5 5h-11z"/><path d="M9 9l-2 11M15 9l2 11M12 9v11"/>',
  // Hidran umum: badan hidran dengan dua kepala keran.
  hidran_umum: '<path d="M8 21v-9a4 4 0 0 1 8 0v9"/><path d="M5 14h3M16 14h3M7 21h10"/>',
  // Intake: sumur dengan timba.
  intake: '<path d="M6 6h12M8 6v14M16 6v14M4 20h16"/><path d="M12 9v4"/>',
  // Pompa: roda impeler dalam lingkaran.
  pompa: '<circle cx="12" cy="12" r="8"/><path d="M12 7v10M7 12h10"/>',
  // Meter induk: gauge setengah lingkaran dengan jarum.
  meter_induk: '<path d="M4 15a8 8 0 0 1 16 0"/><path d="M12 15l4-4M7 19h10"/>',
}

const ASET_LABEL_BY_TYPE: Record<AsetBaru, string> = ASET_LABEL

export const FEATURE_META: Record<FeatureType, FeatureMeta> = {
  // Pipa digambar sebagai garis di peta; ikon ini hanya untuk legenda dan daftar.
  pipa: { label: 'Pipa', color: '#1d4ed8', icon: '<path d="M3 9h18M3 15h18"/>' },
  sr: {
    label: 'Sambungan rumah',
    color: '#a16207',
    // Rumah: atap dan dinding dengan pintu.
    icon: '<path d="M4 11l8-6 8 6v8a1 1 0 0 1-1 1h-4v-5h-6v5H5a1 1 0 0 1-1-1z"/>',
  },
  valve: {
    label: 'Valve',
    color: '#111111',
    // Gate valve: dua segitiga berhadapan.
    icon: '<path d="M4 7v10l8-5 8 5V7l-8 5z"/>',
  },
  reservoir: {
    label: 'Reservoir',
    color: '#0f766e',
    // Tangki silinder dengan tutup elips.
    icon: '<ellipse cx="12" cy="6.5" rx="6.5" ry="2.5"/><path d="M5.5 6.5v11c0 1.4 2.9 2.5 6.5 2.5s6.5-1.1 6.5-2.5v-11"/>',
  },
  bronkaptering: {
    label: 'Bronkaptering',
    color: '#0891b2',
    // Tetesan air: mata air yang ditangkap.
    icon: '<path d="M12 3.5s6 6.4 6 11a6 6 0 0 1-12 0c0-4.6 6-11 6-11z"/>',
  },
  tapping: {
    label: 'Tapping pipa',
    color: '#65a30d',
    // Pipa horizontal dengan cabang ke bawah dan lingkaran kecil di ujungnya: titik sadapan air dari pipa.
    icon: '<path d="M3 8h18M12 8v8"/><circle cx="12" cy="18" r="2"/>',
  },
  mata_air: {
    label: 'Mata air',
    color: '#a21caf',
    // Tetesan kecil di atas dan gelombang di bawahnya: air yang keluar dari sumber alami.
    icon: '<path d="M12 3s-4 4.4-4 7.4a4 4 0 0 0 8 0C16 7.4 12 3 12 3z"/><path d="M4 18c1.3-1 2.7-1 4 0s2.7 1 4 0 2.7-1 4 0 2.7 1 4 0"/>',
  },
  ...Object.fromEntries(
    (Object.keys(ASET_ICON) as AsetBaru[]).map((t) => [
      t,
      { label: ASET_LABEL_BY_TYPE[t], color: ASET_COLOR[t], icon: ASET_ICON[t] },
    ]),
  ),
} as Record<FeatureType, FeatureMeta>

/**
 * Warna badge. Sambungan rumah mengikuti status cakupan: hijau sudah terlayani, kuning belum.
 * Nilai `terlayani` hanya ada untuk peran internal, jadi `null` memakai warna dasar jenis.
 */
export function warnaFitur(type: FeatureType, terlayani?: boolean | null): string {
  if (type === 'sr') {
    if (terlayani === true) return '#15803d'
    if (terlayani === false) return '#ca8a04'
  }
  return FEATURE_META[type].color
}

const BADGE_STYLE =
  'display:flex;align-items:center;justify-content:center;box-sizing:border-box;' +
  'border:2px solid #ffffff;border-radius:7px;box-shadow:0 1px 3px rgba(0,0,0,0.3);'

/** Markup badge ikon (HTML string). Ukuran `size` dalam piksel. */
export function ikonHtml(type: FeatureType, terlayani?: boolean | null, size = 26): string {
  const icon = Math.round(size * 0.56)
  return (
    `<span class="fi-badge" style="${BADGE_STYLE}width:${size}px;height:${size}px;background:${warnaFitur(type, terlayani)}">` +
    `<svg viewBox="0 0 24 24" width="${icon}" height="${icon}" fill="none" stroke="#ffffff" stroke-width="2" ` +
    `stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${FEATURE_META[type].icon}</svg>` +
    `</span>`
  )
}
