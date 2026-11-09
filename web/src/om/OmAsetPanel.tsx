import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { fetchMe } from '../api'
import { toastGalat, toastSukses } from '../components/shared/toast'

import {
  catatJadwalSelesai,
  omAset,
  simpanRehab,
  tambahJadwal,
  tambahPerbaikan,
  type StatusRehab,
} from './omApi'
import {
  bolehCatat,
  formatRupiah,
  formatTanggal,
  LABEL_JENIS_GANGGUAN,
  LABEL_JENIS_PERAWATAN,
  LABEL_KONDISI,
  LABEL_STATUS_GANGGUAN,
  LABEL_STATUS_REHAB,
  PILIHAN_REHAB,
  validasiRehab,
} from './omLogic'
import './om.css'

type Props = { featureId: number; kondisi: string | null; bisaCatat?: boolean }

const JENIS_PERAWATAN = ['pembersihan_bak', 'cek_valve', 'kalibrasi_meter', 'lainnya'] as const

function hariIniWib(): string {
  // Tanggal operasional memakai WIB (UTC+7), sama dengan server.
  return new Date(Date.now() + 7 * 60 * 60 * 1000).toISOString().slice(0, 10)
}

/** Bagian O&M di panel detail aset: rehabilitasi, perbaikan, jadwal perawatan, laporan, dan riwayat.
 * Panel ini hanya tampil untuk pengguna yang sudah masuk. Pengunjung tidak meminta data O&M. */
export function OmAsetPanel({ featureId, kondisi, bisaCatat: bisaCatatProp }: Props) {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const sudahMasuk = Boolean(me.data)
  const bisaCatat = bisaCatatProp ?? bolehCatat(me.data?.role)
  const om = useQuery({
    queryKey: ['om', 'aset', featureId],
    queryFn: () => omAset(featureId),
    enabled: sudahMasuk,
  })

  if (!sudahMasuk) return null

  return (
    <section className="om-aset" aria-label="Operasi dan pemeliharaan aset">
      <h3>Pemeliharaan</h3>
      {om.isPending && <p className="om-muted">Memuat data O&M…</p>}
      {om.isError && <p role="alert" className="om-error">{(om.error as Error).message}</p>}
      {om.data && (
        <>
          <RehabBagian featureId={featureId} kondisi={kondisi} rehab={om.data.rehab} bisaCatat={bisaCatat} />
          <PerbaikanBagian featureId={featureId} perbaikan={om.data.perbaikan} bisaCatat={bisaCatat} />
          <JadwalBagian featureId={featureId} jadwal={om.data.jadwal} bisaCatat={bisaCatat} />
          <GangguanBagian gangguan={om.data.gangguan} />
          <details className="om-riwayat">
            <summary>Riwayat O&amp;M ({om.data.riwayat.length})</summary>
            {om.data.riwayat.length === 0 && <p className="om-muted">Belum ada riwayat.</p>}
            <ol className="om-riwayat-list">
              {om.data.riwayat.map((r, i) => (
                <li key={`${r.sumber}-${r.sumber_id}-${r.diubah_pada}-${i}`}>
                  <span>{formatTanggal(r.diubah_pada.slice(0, 10))}</span> {r.diubah_oleh} · {r.sumber} {r.aksi}
                </li>
              ))}
            </ol>
          </details>
        </>
      )}
    </section>
  )
}

function useSegarkanOm(featureId: number) {
  const qc = useQueryClient()
  return () => {
    qc.invalidateQueries({ queryKey: ['om', 'aset', featureId] })
    qc.invalidateQueries({ queryKey: ['om', 'jatuh-tempo'] })
    qc.invalidateQueries({ queryKey: ['om', 'rehab-prioritas'] })
  }
}

function RehabBagian({
  featureId,
  kondisi,
  rehab,
  bisaCatat,
}: {
  featureId: number
  kondisi: string | null
  rehab: { status_rehab: StatusRehab; prioritas: number | null; catatan: string | null } | null
  bisaCatat: boolean
}) {
  const segarkan = useSegarkanOm(featureId)
  const [buka, setBuka] = useState(false)
  const [status, setStatus] = useState<StatusRehab>(rehab?.status_rehab ?? 'perlu')
  const [prioritas, setPrioritas] = useState<string>(rehab?.prioritas?.toString() ?? '')
  const [catatan, setCatatan] = useState(rehab?.catatan ?? '')
  const [galat, setGalat] = useState<string | null>(null)

  const simpan = useMutation({
    mutationFn: () =>
      simpanRehab(featureId, {
        status_rehab: status,
        prioritas: prioritas === '' ? null : Number(prioritas),
        catatan: catatan.trim() || null,
      }),
    onSuccess: () => {
      setBuka(false)
      toastSukses('rehab.simpan')
      segarkan()
    },
    onError: (err) => toastGalat(err),
  })

  function kirim(e: FormEvent) {
    e.preventDefault()
    const p = prioritas === '' ? null : Number(prioritas)
    const salah = validasiRehab(status, p, kondisi)
    setGalat(salah)
    if (!salah) simpan.mutate()
  }

  return (
    <div className="om-bagian">
      <p>
        <strong>Rehabilitasi:</strong>{' '}
        {LABEL_STATUS_REHAB[rehab?.status_rehab ?? 'belum_ditandai']}
        {rehab?.prioritas != null && ` · prioritas ${rehab.prioritas}`}
        {kondisi && ` · kondisi ${LABEL_KONDISI[kondisi] ?? kondisi}`}
      </p>
      {rehab?.catatan && <p className="om-muted">{rehab.catatan}</p>}
      {bisaCatat && !buka && (
        <button type="button" onClick={() => setBuka(true)}>
          Ubah penandaan
        </button>
      )}
      {bisaCatat && buka && (
        <form className="om-form" onSubmit={kirim} aria-label="Penandaan rehabilitasi">
          <label className="om-field">
            Status
            <select value={status} onChange={(e) => setStatus(e.target.value as StatusRehab)}>
              {PILIHAN_REHAB.map((s) => (
                <option key={s} value={s}>
                  {LABEL_STATUS_REHAB[s]}
                </option>
              ))}
            </select>
          </label>
          <label className="om-field">
            Angka prioritas (1 paling mendesak)
            <select value={prioritas} onChange={(e) => setPrioritas(e.target.value)}>
              <option value="">Tanpa angka</option>
              {Array.from({ length: 10 }, (_, i) => i + 1).map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
          <label className="om-field">
            Catatan
            <textarea rows={2} value={catatan} onChange={(e) => setCatatan(e.target.value)} />
          </label>
          {galat && <p role="alert" className="om-error">{galat}</p>}
          <div className="om-baris">
            <button type="submit" disabled={simpan.isPending}>
              Simpan
            </button>
            <button type="button" onClick={() => setBuka(false)}>
              Batal
            </button>
          </div>
        </form>
      )}
    </div>
  )
}

function PerbaikanBagian({
  featureId,
  perbaikan,
  bisaCatat,
}: {
  featureId: number
  perbaikan: { id: number; tanggal: string; tindakan: string; teknisi: string; biaya: number | null }[]
  bisaCatat: boolean
}) {
  const segarkan = useSegarkanOm(featureId)
  const [buka, setBuka] = useState(false)
  const [tanggal, setTanggal] = useState(hariIniWib())
  const [tindakan, setTindakan] = useState('')
  const [teknisi, setTeknisi] = useState('')
  const [biaya, setBiaya] = useState('')
  const [galat, setGalat] = useState<string | null>(null)

  const simpan = useMutation({
    mutationFn: () =>
      tambahPerbaikan(featureId, {
        tanggal,
        tindakan: tindakan.trim(),
        teknisi: teknisi.trim(),
        biaya: biaya === '' ? null : Number(biaya),
      }),
    onError: (err) => toastGalat(err),
    onSuccess: () => {
      toastSukses('perbaikan.catat')
      setBuka(false)
      setTindakan('')
      setTeknisi('')
      setBiaya('')
      segarkan()
    },
  })

  function kirim(e: FormEvent) {
    e.preventDefault()
    if (!tindakan.trim()) return setGalat('Tindakan wajib diisi.')
    if (!teknisi.trim()) return setGalat('Nama teknisi wajib diisi.')
    if (biaya !== '' && (!Number.isFinite(Number(biaya)) || Number(biaya) < 0)) return setGalat('Biaya tidak boleh negatif.')
    setGalat(null)
    simpan.mutate()
  }

  return (
    <div className="om-bagian">
      <h4>Riwayat perbaikan</h4>
      {perbaikan.length === 0 && <p className="om-muted">Belum ada perbaikan tercatat.</p>}
      <ul className="om-list-ringkas">
        {perbaikan.slice(0, 5).map((p) => (
          <li key={p.id}>
            <span>{formatTanggal(p.tanggal)}</span> {p.tindakan} · {p.teknisi}
            {p.biaya !== null && ` · ${formatRupiah(p.biaya)}`}
          </li>
        ))}
      </ul>
      {bisaCatat && !buka && (
        <button type="button" onClick={() => setBuka(true)}>
          Catat perbaikan
        </button>
      )}
      {bisaCatat && buka && (
        <form className="om-form" onSubmit={kirim} aria-label="Catat perbaikan">
          <label className="om-field">
            Tanggal
            <input type="date" value={tanggal} max={hariIniWib()} onChange={(e) => setTanggal(e.target.value)} />
          </label>
          <label className="om-field">
            Tindakan
            <input value={tindakan} maxLength={500} onChange={(e) => setTindakan(e.target.value)} />
          </label>
          <label className="om-field">
            Teknisi
            <input value={teknisi} maxLength={100} onChange={(e) => setTeknisi(e.target.value)} />
          </label>
          <label className="om-field">
            Biaya (Rp, opsional)
            <input type="number" min={0} step="any" value={biaya} onChange={(e) => setBiaya(e.target.value)} />
          </label>
          {galat && <p role="alert" className="om-error">{galat}</p>}
          <div className="om-baris">
            <button type="submit" disabled={simpan.isPending}>
              Simpan
            </button>
            <button type="button" onClick={() => setBuka(false)}>
              Batal
            </button>
          </div>
        </form>
      )}
    </div>
  )
}

function JadwalBagian({
  featureId,
  jadwal,
  bisaCatat,
}: {
  featureId: number
  jadwal: { id: number; jenis: string; interval_hari: number; tanggal_berikutnya: string; aktif: boolean }[]
  bisaCatat: boolean
}) {
  const segarkan = useSegarkanOm(featureId)
  const [buka, setBuka] = useState(false)
  const [jenis, setJenis] = useState<(typeof JENIS_PERAWATAN)[number]>('pembersihan_bak')
  const [intervalTeks, setIntervalTeks] = useState('90')
  const [pertama, setPertama] = useState('')
  const [galat, setGalat] = useState<string | null>(null)

  const tambah = useMutation({
    mutationFn: () =>
      tambahJadwal(featureId, {
        jenis,
        interval_hari: Number(intervalTeks),
        tanggal_berikutnya: pertama || null,
      }),
    onSuccess: () => {
      toastSukses('jadwal.tambah')
      setBuka(false)
      segarkan()
    },
    onError: (err) => toastGalat(err),
  })
  const selesai = useMutation({
    mutationFn: (id: number) => catatJadwalSelesai(id),
    onSuccess: () => {
      toastSukses('jadwal.selesai')
      segarkan()
    },
    onError: (err) => toastGalat(err),
  })

  function kirim(e: FormEvent) {
    e.preventDefault()
    const n = Number(intervalTeks)
    if (!Number.isInteger(n) || n < 1 || n > 3650) return setGalat('Interval harus 1 sampai 3650 hari.')
    setGalat(null)
    tambah.mutate()
  }

  return (
    <div className="om-bagian">
      <h4>Jadwal perawatan</h4>
      {jadwal.length === 0 && <p className="om-muted">Belum ada jadwal.</p>}
      <ul className="om-list-ringkas">
        {jadwal.map((j) => (
          <li key={j.id}>
            {LABEL_JENIS_PERAWATAN[j.jenis] ?? j.jenis}, tiap {j.interval_hari} hari · berikutnya{' '}
            {formatTanggal(j.tanggal_berikutnya)}
            {bisaCatat && (
              <button type="button" className="om-kecil" onClick={() => selesai.mutate(j.id)} disabled={selesai.isPending}>
                Catat selesai
              </button>
            )}
          </li>
        ))}
      </ul>
      {bisaCatat && !buka && (
        <button type="button" onClick={() => setBuka(true)}>
          Tambah jadwal
        </button>
      )}
      {bisaCatat && buka && (
        <form className="om-form" onSubmit={kirim} aria-label="Tambah jadwal perawatan">
          <label className="om-field">
            Jenis perawatan
            <select value={jenis} onChange={(e) => setJenis(e.target.value as (typeof JENIS_PERAWATAN)[number])}>
              {JENIS_PERAWATAN.map((j) => (
                <option key={j} value={j}>
                  {LABEL_JENIS_PERAWATAN[j]}
                </option>
              ))}
            </select>
          </label>
          <label className="om-field">
            Interval (hari)
            <input type="number" min={1} max={3650} value={intervalTeks} onChange={(e) => setIntervalTeks(e.target.value)} />
          </label>
          <label className="om-field">
            Tanggal pertama (opsional, kosong berarti hari ini)
            <input type="date" value={pertama} onChange={(e) => setPertama(e.target.value)} />
          </label>
          {galat && <p role="alert" className="om-error">{galat}</p>}
              <div className="om-baris">
            <button type="submit" disabled={tambah.isPending}>
              Simpan
            </button>
            <button type="button" onClick={() => setBuka(false)}>
              Batal
            </button>
          </div>
        </form>
      )}
    </div>
  )
}

function GangguanBagian({
  gangguan,
}: {
  gangguan: { id: number; jenis: string; status: string; dilaporkan_pada: string }[]
}) {
  return (
    <div className="om-bagian">
      <h4>Laporan gangguan</h4>
      {gangguan.length === 0 && <p className="om-muted">Belum ada laporan untuk aset ini.</p>}
      <ul className="om-list-ringkas">
        {gangguan.slice(0, 5).map((g) => (
          <li key={g.id}>
            <span>{formatTanggal(g.dilaporkan_pada.slice(0, 10))}</span>{' '}
            {LABEL_JENIS_GANGGUAN[g.jenis as keyof typeof LABEL_JENIS_GANGGUAN] ?? g.jenis} ·{' '}
            {LABEL_STATUS_GANGGUAN[g.status as keyof typeof LABEL_STATUS_GANGGUAN] ?? g.status}
          </li>
        ))}
      </ul>
    </div>
  )
}
