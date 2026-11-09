import { useId, useMemo, useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query'
import { History, Pencil, X } from 'lucide-react'
import { fetchNetworks } from './api'
import { toastGalat, toastSukses } from './components/shared/toast'
import {
  adalahFitur,
  AsetApiError,
  buatAset,
  EntriRiwayat,
  fetchRiwayat,
  hapusAset,
  pindahAset,
  ubahAset,
} from './asetApi'
import {
  buatPayload,
  fieldUntuk,
  geometriUntuk,
  KONDISI,
  LABEL_FIELD,
  LABEL_JENIS,
  MATERIAL,
  NILAI_KOSONG,
  nilaiDari,
  selisihAtribut,
  ubahPayload,
  validasiAset,
  wajibUntuk,
  type Field,
  type NilaiForm,
} from './asetForm'
import { geometriPindah } from './asetEditing'
import { useMapStore } from './store'
import type { PipeFeature } from './types'
import './aset.css'

/** Segarkan data yang bergantung pada fitur: peta, dan riwayat bila sudah terbuka. */
function segarkan(qc: QueryClient) {
  qc.invalidateQueries({ queryKey: ['features'] })
  qc.invalidateQueries({ queryKey: ['riwayat'] })
}

const NUMERIK = new Set<Field>(['diameter_mm', 'tahun_pasang', 'tahun_bangun', 'kapasitas_m3', 'debit_lps', 'elevasi_m'])

/** Form atribut untuk fitur baru (dari klik di peta) atau fitur yang sudah ada. Tampil di tepi kiri. */
export function AsetFormPanel() {
  const formulir = useMapStore((s) => s.formulir)
  const networkId = useMapStore((s) => s.networkId)
  if (!formulir) return null
  const kunci = formulir.kind === 'ubah' ? `ubah-${formulir.feature.id}` : `buat-${formulir.jenis}-${Date.now()}`
  return <FormulirIsi key={kunci} formulir={formulir} networkId={networkId} />
}

function FormulirIsi({
  formulir,
  networkId,
}: {
  formulir: NonNullable<ReturnType<typeof useMapStore.getState>['formulir']>
  networkId: number | null
}) {
  const qc = useQueryClient()
  const setFormulir = useMapStore((s) => s.setFormulir)
  const select = useMapStore((s) => s.select)
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const judulId = useId()

  const jenis = formulir.kind === 'buat' ? formulir.jenis : formulir.feature.properties.feature_type
  const unitId = formulir.kind === 'buat' ? networkId : formulir.feature.properties.network_id
  const namaUnit = networks.data?.find((n) => n.id === unitId)?.nama ?? (unitId === null ? 'belum dipilih' : `#${unitId}`)

  const awal = useMemo<NilaiForm>(
    () => (formulir.kind === 'ubah' ? nilaiDari(formulir.feature.properties) : NILAI_KOSONG),
    [formulir],
  )
  const [nilai, setNilai] = useState<NilaiForm>(awal)
  const [coba, setCoba] = useState(false)
  const [galat, setGalat] = useState<string | null>(null)
  const galatIsian = useMemo(() => validasiAset(jenis, nilai), [jenis, nilai])
  const wajib = new Set(wajibUntuk(jenis))

  const simpan = useMutation({
    mutationFn: () => {
      if (formulir.kind === 'buat') {
        if (networkId === null) throw new AsetApiError(400, 'Pilih unit SPAM tujuan di header dulu.')
        return buatAset(buatPayload(jenis, formulir.geometry, nilai, networkId))
      }
      return ubahAset(formulir.feature.id, ubahPayload(jenis, awal, nilai))
    },
    onSuccess: (data) => {
      segarkan(qc)
      toastSukses(formulir.kind === 'buat' ? 'aset.tambah' : 'aset.ubah')
      setFormulir(null)
      if (adalahFitur(data)) select(data)
    },
    onError: (err) => toastGalat(err),
  })

  function kirim(e: FormEvent) {
    e.preventDefault()
    setCoba(true)
    setGalat(null)
    if (Object.keys(galatIsian).length > 0) return
    if (formulir.kind === 'ubah' && Object.keys(ubahPayload(jenis, awal, nilai)).length === 0) {
      setGalat('Tidak ada perubahan untuk disimpan.')
      return
    }
    simpan.mutate()
  }

  function batal() {
    setFormulir(null)
  }

  const judul = formulir.kind === 'buat' ? `Tambah ${LABEL_JENIS[jenis]}` : `Ubah ${LABEL_JENIS[jenis]}`
  const titikInfo =
    formulir.kind === 'buat'
      ? formulir.geometry.type === 'LineString'
        ? `Jalur pipa dari ${formulir.geometry.coordinates.length} titik.`
        : formulir.geometry.type === 'Point'
          ? `Lokasi ${formulir.geometry.coordinates[1].toFixed(5)}, ${formulir.geometry.coordinates[0].toFixed(5)}.`
          : null
      : null

  return (
    <aside className="side-panel aset-form-panel" aria-labelledby={judulId}>
      <header className="panel-head">
        <Pencil size={16} aria-hidden="true" />
        <h2 id={judulId}>{judul}</h2>
        <button type="button" aria-label="Tutup form" onClick={batal}>
          <X size={16} aria-hidden="true" />
        </button>
      </header>
      <form className="panel-body aset-form" onSubmit={kirim} noValidate>
        <p className="panel-muted">Unit SPAM: {namaUnit}</p>
        {titikInfo && <p className="panel-muted">{titikInfo}</p>}
        <p className="panel-muted">Isian bertanda * wajib diisi.</p>

        {fieldUntuk(jenis).map((f) => {
          const pesan = coba ? galatIsian[f] : undefined
          const id = `${judulId}-${f}`
          return (
            <div className="aset-field" key={f}>
              <label htmlFor={id}>
                {LABEL_FIELD[f]}
                {wajib.has(f) && <span aria-hidden="true"> *</span>}
              </label>
              <IsianField
                id={id}
                field={f}
                value={nilai[f]}
                invalid={Boolean(pesan)}
                kondisiSekarang={awal.kondisi}
                onChange={(v) => setNilai((n) => ({ ...n, [f]: v }))}
              />
              {pesan && <span className="aset-galat">{pesan}</span>}
            </div>
          )
        })}

        {galat && <p role="alert" className="panel-error">{galat}</p>}

        <div className="btn-row">
          <button type="submit" className="tool-btn primary" disabled={simpan.isPending}>
            {simpan.isPending ? 'Menyimpan…' : 'Simpan'}
          </button>
          <button type="button" className="tool-btn" onClick={batal} disabled={simpan.isPending}>
            Batal
          </button>
        </div>
      </form>
    </aside>
  )
}

function IsianField({
  id,
  field,
  value,
  invalid,
  kondisiSekarang,
  onChange,
}: {
  id: string
  field: Field
  value: string
  invalid: boolean
  kondisiSekarang: string
  onChange: (v: string) => void
}) {
  const common = {
    id,
    value,
    'aria-invalid': invalid || undefined,
    onChange: (e: { target: { value: string } }) => onChange(e.target.value),
  }
  if (field === 'material') {
    return (
      <select {...common}>
        <option value="">Belum diisi</option>
        {MATERIAL.map((m) => (
          <option key={m} value={m}>{m}</option>
        ))}
      </select>
    )
  }
  if (field === 'kondisi') {
    // Data lama bisa memakai nilai di luar daftar (mis. "perlu cek"). Nilai itu tetap ditampilkan agar tidak hilang diam-diam.
    const pilihan = KONDISI.some((k) => k.value === kondisiSekarang) || !kondisiSekarang
      ? KONDISI
      : [...KONDISI, { value: kondisiSekarang, label: kondisiSekarang }]
    return (
      <select {...common}>
        <option value="">Belum diisi</option>
        {pilihan.map((k) => (
          <option key={k.value} value={k.value}>{k.label}</option>
        ))}
      </select>
    )
  }
  return (
    <input
      {...common}
      type="text"
      inputMode={NUMERIK.has(field) ? 'decimal' : undefined}
      maxLength={field === 'nama' ? 100 : undefined}
    />
  )
}

/** Bilah kecil di bawah peta: langkah menambah pipa, dan simpan atau batal saat memindahkan aset. */
export function AsetModeBar() {
  const editor = useMapStore((s) => s.editor)
  const setEditor = useMapStore((s) => s.setEditor)
  const setFormulir = useMapStore((s) => s.setFormulir)
  const select = useMapStore((s) => s.select)
  const qc = useQueryClient()
  const [galat, setGalat] = useState<string | null>(null)

  const pindah = useMutation({
    mutationFn: (id: number) => {
      const geometri = geometriPindah()
      if (!geometri) throw new AsetApiError(400, 'Posisi belum terbaca. Coba seret lagi.')
      return pindahAset(id, geometri)
    },
    onSuccess: (data) => {
      segarkan(qc)
      toastSukses('aset.pindah')
      setEditor(null)
      if (adalahFitur(data)) select(data)
    },
    onError: (err) => toastGalat(err),
  })

  if (!editor) return null

  if (editor.kind === 'tambah') {
    const titik = geometriUntuk(editor.jenis) === 'Point'
    const label = LABEL_JENIS[editor.kind === 'tambah' ? editor.jenis : 'pipa']
    return (
      <div className="aset-bar" role="status">
        <span>
          {titik
            ? `Klik di peta untuk meletakkan ${label}. Esc untuk batal.`
            : `Klik titik-titik jalur ${label.toLowerCase()} (${editor.titik.length} titik). Pilih Selesai bila sudah. Esc untuk batal.`}
        </span>
        <div className="aset-bar-aksi">
          {!titik && (
            <button
              type="button"
              className="tool-btn primary"
              disabled={editor.titik.length < 2}
              onClick={() => {
                setFormulir({ kind: 'buat', jenis: editor.jenis, geometry: { type: 'LineString', coordinates: editor.titik } })
                setEditor(null)
              }}
            >
              Selesai
            </button>
          )}
          <button type="button" className="tool-btn" onClick={() => setEditor(null)}>Batal</button>
        </div>
      </div>
    )
  }

  const titik = editor.feature.geometry.type === 'Point'
  return (
    <div className="aset-bar" role="status">
      <span>
        {titik
          ? 'Seret titik ke lokasi baru, lalu simpan posisi.'
          : 'Seret titik ujung atau simpul garis ke posisi baru, lalu simpan posisi.'}
      </span>
      {galat && <span role="alert" className="aset-galat">{galat}</span>}
      <div className="aset-bar-aksi">
        <button
          type="button"
          className="tool-btn primary"
          disabled={pindah.isPending}
          onClick={() => {
            setGalat(null)
            pindah.mutate(editor.feature.id)
          }}
        >
          {pindah.isPending ? 'Menyimpan…' : 'Simpan posisi'}
        </button>
        <button type="button" className="tool-btn" disabled={pindah.isPending} onClick={() => setEditor(null)}>Batal</button>
      </div>
    </div>
  )
}

const AKSI: Record<string, string> = { buat: 'Dibuat', ubah: 'Diubah', pindah: 'Dipindah', hapus: 'Dihapus' }

function waktu(s: string): string {
  const d = new Date(s)
  return Number.isNaN(d.getTime()) ? s : d.toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' })
}

/** Aksi untuk aset yang dipilih: ubah atribut, pindah, hapus, dan riwayat. Hanya tampil untuk editor dan admin. */
export function AsetAksi({ feature }: { feature: PipeFeature }) {
  const qc = useQueryClient()
  const editor = useMapStore((s) => s.editor)
  const setEditor = useMapStore((s) => s.setEditor)
  const setFormulir = useMapStore((s) => s.setFormulir)
  const select = useMapStore((s) => s.select)
  const [konfirmasi, setKonfirmasi] = useState(false)
  const [galat, setGalat] = useState<string | null>(null)
  const [riwayatBuka, setRiwayatBuka] = useState(false)
  const sibuk = editor !== null

  const hapus = useMutation({
    mutationFn: () => hapusAset(feature.id),
    onSuccess: () => {
      segarkan(qc)
      toastSukses('aset.hapus')
      select(null)
    },
    onError: (err) => {
      setKonfirmasi(false)
      toastGalat(err)
    },
  })

  const riwayat = useQuery({
    queryKey: ['riwayat', feature.id],
    queryFn: () => fetchRiwayat(feature.id),
    enabled: riwayatBuka,
  })

  return (
    <section className="aset-aksi" aria-label="Aksi aset">
      {galat && <p role="alert" className="panel-error">{galat}</p>}
      <div className="aset-aksi-btn">
        <button type="button" className="tool-btn" disabled={sibuk} onClick={() => { setGalat(null); setFormulir({ kind: 'ubah', feature }) }}>
          Ubah atribut
        </button>
        <button type="button" className="tool-btn" disabled={sibuk} onClick={() => { setGalat(null); setEditor({ kind: 'pindah', feature }) }}>
          Pindah
        </button>
        {!konfirmasi ? (
          <button type="button" className="tool-btn" disabled={sibuk} onClick={() => setKonfirmasi(true)}>Hapus</button>
        ) : (
          <div role="alertdialog" aria-label="Konfirmasi hapus" className="aset-konfirmasi">
            <p>Hapus {feature.properties.nama ?? LABEL_JENIS[feature.properties.feature_type]} dari peta? Riwayat tetap tersimpan.</p>
            <div className="btn-row">
              <button type="button" className="tool-btn primary" disabled={hapus.isPending} onClick={() => hapus.mutate()}>
                {hapus.isPending ? 'Menghapus…' : 'Ya, hapus'}
              </button>
              <button type="button" className="tool-btn" disabled={hapus.isPending} onClick={() => setKonfirmasi(false)}>Batal</button>
            </div>
          </div>
        )}
      </div>

      <details className="aset-riwayat" onToggle={(e) => setRiwayatBuka(e.currentTarget.open)}>
        <summary><History size={14} aria-hidden="true" /> Riwayat perubahan</summary>
        <RiwayatList
          pending={riwayatBuka && riwayat.isPending}
          error={riwayat.isError ? (riwayat.error as Error).message : null}
          data={riwayat.data}
        />
      </details>
    </section>
  )
}

function RiwayatList({ pending, error, data }: { pending: boolean; error: string | null; data: EntriRiwayat[] | undefined }) {
  if (pending) return <p className="panel-muted">Memuat riwayat…</p>
  if (error) return <p role="alert" className="panel-error">{error}</p>
  if (!data || data.length === 0) return <p className="panel-muted">Belum ada riwayat.</p>
  return (
    <ol className="aset-riwayat-list">
      {data.map((e, i) => {
        const selisih = selisihAtribut(e.sebelum, e.sesudah)
        return (
          <li key={`${e.diubah_pada}-${i}`}>
            <strong>{AKSI[e.aksi] ?? e.aksi}</strong> oleh {e.diubah_oleh}
            <span className="panel-muted"> · {waktu(e.diubah_pada)}</span>
            {selisih.length > 0 && (
              <ul>
                {selisih.map((d) => (
                  <li key={d.kolom}>{d.kolom}: {d.dari} → {d.ke}</li>
                ))}
              </ul>
            )}
          </li>
        )
      })}
    </ol>
  )
}
