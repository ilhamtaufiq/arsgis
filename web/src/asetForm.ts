import type { Geometry } from 'geojson'
import { ASET_LABEL } from './asetStyle'
import type { FeatureProperties, FeatureType } from './types'

// Logika form aset tanpa React, supaya validasi dan bentuk payload bisa dites langsung.
// Aturan wajib di sini hanya salinan untuk umpan balik cepat. Backend tetap otoritas (422).

export const LABEL_JENIS: Record<FeatureType, string> = {
  pipa: 'Pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
  tapping: 'Tapping pipa',
  mata_air: 'Mata air',
  ...ASET_LABEL,
}

/** Tiga belas jenis fitur. Daftar ini dipakai form aset, jadi jenis baru harus ditambah di sini juga. */
export const JENIS_SEMUA: FeatureType[] = [
  'pipa',
  'valve',
  'reservoir',
  'sr',
  'bronkaptering',
  'tapping',
  'bak_penahan_tekanan',
  'menara_air',
  'hidran_umum',
  'intake',
  'mata_air',
  'pompa',
  'meter_induk',
]

export const MATERIAL = ['PVC', 'HDPE', 'GIP', 'DCI', 'Steel', 'ACP', 'Lainnya'] as const

export const KONDISI: { value: string; label: string }[] = [
  { value: 'baik', label: 'Baik' },
  { value: 'rusak_ringan', label: 'Rusak ringan' },
  { value: 'rusak_berat', label: 'Rusak berat' },
  { value: 'tidak_berfungsi', label: 'Tidak berfungsi' },
]

export type Field =
  | 'nama'
  | 'diameter_mm'
  | 'material'
  | 'tahun_pasang'
  | 'kondisi'
  | 'status'
  | 'kapasitas_m3'
  | 'debit_lps'
  | 'elevasi_m'
  | 'tahun_bangun'
  | 'sumber_dana'

/** Semua isian form dalam bentuk teks, supaya input kosong dan angka setengah jadi bisa ditampung. */
export type NilaiForm = Record<Field, string>

export const LABEL_FIELD: Record<Field, string> = {
  nama: 'Nama',
  diameter_mm: 'Diameter (mm)',
  material: 'Material',
  tahun_pasang: 'Tahun pasang',
  kondisi: 'Kondisi',
  status: 'Status',
  kapasitas_m3: 'Kapasitas (m³)',
  debit_lps: 'Debit (L/detik)',
  elevasi_m: 'Elevasi (m)',
  tahun_bangun: 'Tahun bangun',
  sumber_dana: 'Sumber dana',
}

/** Atribut kapasitas atau debit untuk tiap jenis aset. Jenis yang tidak tercantum tidak punya keduanya. */
export const ATRIBUT_KAPASITAS: Partial<Record<FeatureType, 'kapasitas_m3' | 'debit_lps'>> = {
  reservoir: 'kapasitas_m3',
  bak_penahan_tekanan: 'kapasitas_m3',
  menara_air: 'kapasitas_m3',
  bronkaptering: 'debit_lps',
  intake: 'debit_lps',
  pompa: 'debit_lps',
  meter_induk: 'debit_lps',
}

const NUMERIK_BULAT: Field[] = ['diameter_mm', 'tahun_pasang', 'tahun_bangun']
const NUMERIK: Field[] = ['kapasitas_m3', 'debit_lps', 'elevasi_m']
const TAHUN_MIN = 1900

export const NILAI_KOSONG: NilaiForm = {
  nama: '',
  diameter_mm: '',
  material: '',
  tahun_pasang: '',
  kondisi: '',
  status: '',
  kapasitas_m3: '',
  debit_lps: '',
  elevasi_m: '',
  tahun_bangun: '',
  sumber_dana: '',
}

/** Pipa digambar sebagai garis; semua jenis lain adalah titik. */
export function geometriUntuk(jenis: FeatureType): 'LineString' | 'Point' {
  return jenis === 'pipa' ? 'LineString' : 'Point'
}

/** Isian yang tampil untuk jenis ini, dalam urutan tampilan. */
export function fieldUntuk(jenis: FeatureType): Field[] {
  if (jenis === 'pipa') {
    return ['nama', 'diameter_mm', 'material', 'tahun_pasang', 'kondisi', 'status', 'sumber_dana']
  }
  const kapasitas = ATRIBUT_KAPASITAS[jenis]
  return [
    'nama',
    ...(kapasitas ? [kapasitas] : []),
    'tahun_bangun',
    'tahun_pasang',
    'elevasi_m',
    'kondisi',
    'status',
    'sumber_dana',
  ]
}

/** Isian yang wajib diisi. Pipa wajib diameter dan material; aset lain wajib nama dan kapasitas atau debit bila ada. */
export function wajibUntuk(jenis: FeatureType): Field[] {
  if (jenis === 'pipa') return ['diameter_mm', 'material']
  const kapasitas = ATRIBUT_KAPASITAS[jenis]
  return ['nama', ...(kapasitas ? [kapasitas] : [])]
}

/** Bilangan dengan koma desimal dibolehkan. `nilai` null berarti kosong. */
export type BacaAngka = { ok: true; nilai: number | null } | { ok: false }

export function bacaAngka(teks: string): BacaAngka {
  const t = teks.trim()
  if (t === '') return { ok: true, nilai: null }
  if (!/^-?\d+([.,]\d+)?$/.test(t)) return { ok: false }
  return { ok: true, nilai: Number(t.replace(',', '.')) }
}

/** Pesan galat per isian. Kosong berarti form valid. `tahunIni` diberi dari luar agar tes deterministik. */
export function validasiAset(
  jenis: FeatureType,
  nilai: NilaiForm,
  tahunIni: number = new Date().getFullYear(),
): Partial<Record<Field, string>> {
  const galat: Partial<Record<Field, string>> = {}

  for (const f of wajibUntuk(jenis)) {
    if (nilai[f].trim() === '') galat[f] = `${LABEL_FIELD[f]} wajib diisi.`
  }

  if (nilai.nama.trim().length > 100) galat.nama = 'Nama maksimal 100 karakter.'
  if (nilai.status.trim().length > 50) galat.status = 'Status maksimal 50 karakter.'
  if (nilai.sumber_dana.trim().length > 100) galat.sumber_dana = 'Sumber dana maksimal 100 karakter.'

  if (nilai.material && !(MATERIAL as readonly string[]).includes(nilai.material)) {
    galat.material = 'Material tidak dikenal.'
  }

  const diameter = bacaAngka(nilai.diameter_mm)
  if (!diameter.ok) galat.diameter_mm = 'Diameter harus berupa angka.'
  else if (diameter.nilai !== null && (!Number.isInteger(diameter.nilai) || diameter.nilai <= 0 || diameter.nilai > 3000)) {
    galat.diameter_mm = 'Diameter berupa bilangan bulat 1 sampai 3000 mm.'
  }

  for (const f of ['tahun_pasang', 'tahun_bangun'] as const) {
    const t = bacaAngka(nilai[f])
    if (!t.ok) galat[f] = 'Tahun harus berupa angka.'
    else if (t.nilai !== null && (!Number.isInteger(t.nilai) || t.nilai < TAHUN_MIN || t.nilai > tahunIni)) {
      galat[f] = `Tahun antara ${TAHUN_MIN} dan ${tahunIni}.`
    }
  }

  for (const f of ['kapasitas_m3', 'debit_lps'] as const) {
    const a = bacaAngka(nilai[f])
    if (!a.ok) galat[f] = `${LABEL_FIELD[f]} harus berupa angka.`
    else if (a.nilai !== null && a.nilai < 0) galat[f] = `${LABEL_FIELD[f]} tidak boleh negatif.`
  }

  const elevasi = bacaAngka(nilai.elevasi_m)
  if (!elevasi.ok) galat.elevasi_m = 'Elevasi harus berupa angka.'

  return galat
}

/** Nilai yang dikirim ke backend untuk satu isian. Null berarti kosong, atau dihapus saat mengubah. */
export function nilaiKirim(f: Field, teks: string): string | number | null {
  const t = teks.trim()
  if (NUMERIK_BULAT.includes(f) || NUMERIK.includes(f)) {
    const a = bacaAngka(t)
    if (!a.ok || a.nilai === null) return null
    return NUMERIK_BULAT.includes(f) ? Math.trunc(a.nilai) : a.nilai
  }
  return t === '' ? null : t
}

/** Isi form dari atribut fitur yang sudah ada. */
export function nilaiDari(p: FeatureProperties): NilaiForm {
  const teks = (v: unknown) => (v === null || v === undefined ? '' : String(v))
  return {
    nama: teks(p.nama),
    diameter_mm: teks(p.diameter_mm),
    material: teks(p.material),
    tahun_pasang: teks(p.tahun_pasang),
    kondisi: teks(p.kondisi),
    status: teks(p.status),
    kapasitas_m3: teks(p.kapasitas_m3),
    debit_lps: teks(p.debit_lps),
    elevasi_m: teks(p.elevasi_m),
    tahun_bangun: teks(p.tahun_bangun),
    sumber_dana: teks(p.sumber_dana),
  }
}

export type PayloadBuat = {
  network_id: number
  feature_type: FeatureType
  geometry: Geometry
} & Partial<Record<Field, string | number>>

/** Badan POST /api/v1/features. Isian kosong tidak dikirim. */
export function buatPayload(jenis: FeatureType, geometry: Geometry, nilai: NilaiForm, networkId: number): PayloadBuat {
  const body: PayloadBuat = { network_id: networkId, feature_type: jenis, geometry }
  for (const f of fieldUntuk(jenis)) {
    const v = nilaiKirim(f, nilai[f])
    if (v !== null) body[f] = v
  }
  return body
}

export type PayloadUbah = Partial<Record<Field | 'geometry', string | number | null | Geometry>>

/** Badan PATCH: hanya isian yang berbeda dari nilai awal. Isian yang dikosongkan dikirim sebagai null. */
export function ubahPayload(jenis: FeatureType, awal: NilaiForm, akhir: NilaiForm): PayloadUbah {
  const body: PayloadUbah = {}
  for (const f of fieldUntuk(jenis)) {
    if (nilaiKirim(f, awal[f]) !== nilaiKirim(f, akhir[f])) body[f] = nilaiKirim(f, akhir[f])
  }
  return body
}

/** Badan PATCH untuk pemindahan: hanya geometri. */
export function pindahPayload(geometry: Geometry): PayloadUbah {
  return { geometry }
}

export const LABEL_KOLOM: Record<string, string> = {
  nama: 'Nama',
  diameter_mm: 'Diameter (mm)',
  material: 'Material',
  tahun_pasang: 'Tahun pasang',
  kondisi: 'Kondisi',
  status: 'Status',
  kapasitas_m3: 'Kapasitas (m³)',
  debit_lps: 'Debit (L/detik)',
  elevasi_m: 'Elevasi (m)',
  tahun_bangun: 'Tahun bangun',
  sumber_dana: 'Sumber dana',
  fungsi: 'Fungsi',
  geometry: 'Lokasi',
}

export type SelisihAtribut = { kolom: string; dari: string; ke: string }

function tampilNilai(v: unknown): string {
  if (v === null || v === undefined || v === '') return '–'
  if (typeof v === 'object') return 'lokasi'
  return String(v)
}

/** Kolom yang berbeda antara dua keadaan, untuk ditampilkan di riwayat. */
export function selisihAtribut(
  sebelum: Record<string, unknown> | null,
  sesudah: Record<string, unknown> | null,
): SelisihAtribut[] {
  const a = sebelum ?? {}
  const b = sesudah ?? {}
  const kunci = [...new Set([...Object.keys(a), ...Object.keys(b)])]
  const hasil: SelisihAtribut[] = []
  for (const k of kunci) {
    if (JSON.stringify(a[k] ?? null) === JSON.stringify(b[k] ?? null)) continue
    hasil.push({ kolom: LABEL_KOLOM[k] ?? k, dari: tampilNilai(a[k]), ke: tampilNilai(b[k]) })
  }
  return hasil
}
