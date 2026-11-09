import { useState, type ChangeEvent, type FormEvent } from 'react'
import { LokasiPicker } from './LokasiPicker'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toastGalat, toastSukses } from '../components/shared/toast'
import {
  ambilGangguan,
  buatGangguan,
  daftarGangguan,
  type Gangguan,
  type JenisGangguan,
  type StatusGangguan,
  ubahStatusGangguan,
  unggahFoto,
  urlFoto,
} from './omApi'
import {
  formatTanggal,
  LABEL_JENIS_GANGGUAN,
  LABEL_STATUS_GANGGUAN,
  MAKS_DESKRIPSI,
  MAKS_PELAPOR,
  statusBerikut,
  validasiFoto,
  validasiLaporan,
} from './omLogic'
import './om.css'

const JENIS: JenisGangguan[] = ['pipa_bocor', 'air_mati', 'kualitas_keruh', 'lainnya']

type Props = { networkId: number; bisaCatat: boolean }

/** Daftar laporan gangguan, formulir laporan baru (editor), dan detail dengan foto serta perubahan status. */
export function GangguanPanel({ networkId, bisaCatat }: Props) {
  const [status, setStatus] = useState<StatusGangguan | ''>('')
  const [terpilih, setTerpilih] = useState<number | null>(null)
  const [formBuka, setFormBuka] = useState(false)
  const daftar = useQuery({
    queryKey: ['om', 'gangguan', networkId, status],
    queryFn: () => daftarGangguan({ networkId, status: status || undefined }),
  })

  return (
    <div className="om-dua-kolom">
      <section className="om-kolom" aria-label="Daftar laporan">
        <div className="om-toolbar">
          <label className="om-field om-inline">
            Status
            <select value={status} onChange={(e) => setStatus(e.target.value as StatusGangguan | '')}>
              <option value="">Semua</option>
              {(Object.keys(LABEL_STATUS_GANGGUAN) as StatusGangguan[]).map((s) => (
                <option key={s} value={s}>
                  {LABEL_STATUS_GANGGUAN[s]}
                </option>
              ))}
            </select>
          </label>
          {bisaCatat && (
            <button type="button" onClick={() => setFormBuka((v) => !v)} aria-expanded={formBuka}>
              {formBuka ? 'Tutup formulir' : 'Laporan baru'}
            </button>
          )}
        </div>

        {formBuka && bisaCatat && (
          <FormLaporan
            networkId={networkId}
            onSimpan={(g) => {
              setTerpilih(g.id)
              setFormBuka(false)
            }}
          />
        )}

        {daftar.isPending && <p className="om-muted">Memuat laporan…</p>}
        {daftar.isError && <p role="alert" className="om-error">{(daftar.error as Error).message}</p>}
        {daftar.data && daftar.data.length === 0 && <p className="om-muted">Belum ada laporan.</p>}
        <ul className="om-list">
          {daftar.data?.map((g) => (
            <li key={g.id}>
              <button
                type="button"
                className={`om-item ${g.id === terpilih ? 'om-item-aktif' : ''}`}
                aria-pressed={g.id === terpilih}
                onClick={() => setTerpilih(g.id)}
              >
                <span className="om-item-judul">
                  {LABEL_JENIS_GANGGUAN[g.jenis]}
                  {g.feature_id !== null && ` · aset #${g.feature_id}`}
                </span>
                <span className="om-muted">
                  {formatTanggal(g.dilaporkan_pada.slice(0, 10))} · {LABEL_STATUS_GANGGUAN[g.status]}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </section>

      <section className="om-kolom" aria-label="Detail laporan">
        {terpilih === null && <p className="om-muted">Pilih laporan untuk melihat detailnya.</p>}
        {terpilih !== null && <DetailLaporan id={terpilih} bisaCatat={bisaCatat} networkId={networkId} />}
      </section>
    </div>
  )
}

function FormLaporan({ networkId, onSimpan }: { networkId: number; onSimpan: (g: Gangguan) => void }) {
  const qc = useQueryClient()
  const [jenis, setJenis] = useState<JenisGangguan>('pipa_bocor')
  const [deskripsi, setDeskripsi] = useState('')
  const [pelapor, setPelapor] = useState('')
  const [lon, setLon] = useState('')
  const [lat, setLat] = useState('')
  const [featureId, setFeatureId] = useState('')
  const [galat, setGalat] = useState<string[]>([])

  const simpan = useMutation({
    mutationFn: buatGangguan,
    onSuccess: (g) => {
      toastSukses('gangguan.buat')
      qc.invalidateQueries({ queryKey: ['om', 'gangguan'] })
      onSimpan(g)
    },
    onError: (err) => toastGalat(err),
  })

  function kirim(e: FormEvent) {
    e.preventDefault()
    const isian = {
      jenis,
      deskripsi,
      pelapor,
      featureId: featureId ? Number(featureId) : null,
      lon: lon === '' ? null : Number(lon),
      lat: lat === '' ? null : Number(lat),
    }
    const daftarGalat = validasiLaporan(isian)
    setGalat(daftarGalat)
    if (daftarGalat.length > 0) return
    simpan.mutate({
      network_id: networkId,
      feature_id: isian.featureId,
      lon: isian.lon,
      lat: isian.lat,
      jenis,
      deskripsi: deskripsi.trim(),
      pelapor: pelapor.trim() || undefined,
    })
  }

  return (
    <form className="om-form" onSubmit={kirim} aria-label="Formulir laporan gangguan">
      <label className="om-field">
        Jenis gangguan
        <select value={jenis} onChange={(e) => setJenis(e.target.value as JenisGangguan)}>
          {JENIS.map((j) => (
            <option key={j} value={j}>
              {LABEL_JENIS_GANGGUAN[j]}
            </option>
          ))}
        </select>
      </label>
      <label className="om-field">
        Deskripsi
        <textarea value={deskripsi} maxLength={MAKS_DESKRIPSI} rows={3} onChange={(e) => setDeskripsi(e.target.value)} />
      </label>
      <label className="om-field">
        Nama pelapor (opsional)
        <input value={pelapor} maxLength={MAKS_PELAPOR} onChange={(e) => setPelapor(e.target.value)} />
      </label>
      <fieldset className="om-fieldset">
        <legend>Lokasi</legend>
        <div className="om-baris">
          <label className="om-field">
            Lon
            <input type="number" step="any" value={lon} onChange={(e) => setLon(e.target.value)} />
          </label>
          <label className="om-field">
            Lat
            <input type="number" step="any" value={lat} onChange={(e) => setLat(e.target.value)} />
          </label>
        </div>
        <LokasiPicker
          lon={lon === '' ? null : Number(lon)}
          lat={lat === '' ? null : Number(lat)}
          onPilih={(x, y) => {
            setLon(String(x))
            setLat(String(y))
          }}
        />
        <label className="om-field">
          ID aset terkait (opsional)
          <input type="number" min={1} value={featureId} onChange={(e) => setFeatureId(e.target.value)} />
        </label>
      </fieldset>
      {galat.length > 0 && (
        <ul role="alert" className="om-error">
          {galat.map((g) => (
            <li key={g}>{g}</li>
          ))}
        </ul>
      )}
      <button type="submit" disabled={simpan.isPending}>
        {simpan.isPending ? 'Menyimpan…' : 'Simpan laporan'}
      </button>
    </form>
  )
}

function DetailLaporan({ id, bisaCatat, networkId }: { id: number; bisaCatat: boolean; networkId: number }) {
  const qc = useQueryClient()
  const [pesanFoto, setPesanFoto] = useState<string | null>(null)
  const detail = useQuery({ queryKey: ['om', 'gangguan-detail', id], queryFn: () => ambilGangguan(id) })

  const segarkan = () => {
    qc.invalidateQueries({ queryKey: ['om', 'gangguan-detail', id] })
    qc.invalidateQueries({ queryKey: ['om', 'gangguan', networkId] })
  }
  const ubah = useMutation({
    mutationFn: (status: StatusGangguan) => ubahStatusGangguan(id, status),
    onSuccess: () => {
      toastSukses('gangguan.status')
      segarkan()
    },
    onError: (err) => toastGalat(err),
  })
  const foto = useMutation({
    mutationFn: (berkas: File) => unggahFoto(id, berkas),
    onSuccess: () => {
      toastSukses('gangguan.foto')
      segarkan()
    },
    onError: (err) => toastGalat(err),
  })

  if (detail.isPending) return <p className="om-muted">Memuat detail…</p>
  if (detail.isError) return <p role="alert" className="om-error">{(detail.error as Error).message}</p>
  const g = detail.data
  if (!g) return null
  const berikut = statusBerikut(g.status)

  function pilihFoto(e: ChangeEvent<HTMLInputElement>) {
    const berkas = e.target.files?.[0]
    e.target.value = ''
    if (!berkas || !g) return
    const salah = validasiFoto(berkas, g.foto.length)
    if (salah) {
      setPesanFoto(salah)
      return
    }
    setPesanFoto(null)
    foto.mutate(berkas)
  }

  return (
    <article className="om-detail">
      <h3>{LABEL_JENIS_GANGGUAN[g.jenis]}</h3>
      <dl className="om-dl">
        <div>
          <dt>Status</dt>
          <dd>{LABEL_STATUS_GANGGUAN[g.status]}</dd>
        </div>
        <div>
          <dt>Dilaporkan</dt>
          <dd>{formatTanggal(g.dilaporkan_pada.slice(0, 10))}</dd>
        </div>
        {g.pelapor !== undefined && (
          <div>
            <dt>Pelapor</dt>
            <dd>{g.pelapor}</dd>
          </div>
        )}
        <div>
          <dt>Lokasi</dt>
          <dd>
            {g.lon !== null && g.lat !== null ? `${g.lat.toFixed(5)}, ${g.lon.toFixed(5)}` : '–'}
            {g.feature_id !== null && ` · aset #${g.feature_id}`}
          </dd>
        </div>
      </dl>
      <p className="om-deskripsi">{g.deskripsi}</p>

      {bisaCatat && berikut.length > 0 && (
        <div className="om-baris">
          {berikut.map((s) => (
            <button key={s} type="button" onClick={() => ubah.mutate(s)} disabled={ubah.isPending}>
              Tandai {LABEL_STATUS_GANGGUAN[s].toLowerCase()}
            </button>
          ))}
        </div>
      )}

      <h4>Foto</h4>
      {g.foto.length === 0 && <p className="om-muted">Belum ada foto.</p>}
      <div className="om-foto-grid">
        {g.foto.map((f) => (
          <img key={f.id} src={urlFoto(f.id)} alt={`Foto laporan ${g.id}`} className="om-foto" loading="lazy" />
        ))}
      </div>
      {bisaCatat && g.foto.length < 5 && (
        <label className="om-field">
          Tambah foto (JPEG atau PNG, maks 5 MB)
          <input type="file" accept="image/jpeg,image/png" onChange={pilihFoto} disabled={foto.isPending} />
        </label>
      )}
      {pesanFoto && <p role="status" className="om-muted">{pesanFoto}</p>}
    </article>
  )
}
