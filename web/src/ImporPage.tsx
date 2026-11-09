import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { canSeeCustomerData, fetchMe, fetchNetworks } from './api'
import { commitImport, uploadKml, type StagedImport } from './importApi'
import { pesanDuplikat, pesanGalatSimpan } from './importLogic'
import { fetchPekerjaan } from './pekerjaanApi'
import { LABEL_JENIS } from './asetForm'
import { ImporDaftar } from './ImporDaftar'
import type { FeatureType } from './types'
import './aset.css'

const JENIS_TITIK: { value: string; label: string }[] = [
  { value: 'sr', label: 'Sambungan rumah (sr)' },
  { value: 'valve', label: 'Valve' },
  { value: 'reservoir', label: 'Reservoir' },
  { value: 'bronkaptering', label: 'Bronkaptering' },
]

/** Halaman impor KML atau KMZ untuk editor dan admin: pilih unit dan paket, unggah, pratinjau laporan, lalu simpan. Di bawahnya ada riwayat impor dan aset terhapus. */
export function ImporPage() {
  const qc = useQueryClient()
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const allowed = canSeeCustomerData(me.data ?? null)
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const pekerjaan = useQuery({ queryKey: ['pekerjaan'], queryFn: fetchPekerjaan, enabled: allowed })
  const [networkId, setNetworkId] = useState<number | null>(null)
  const [pekerjaanId, setPekerjaanId] = useState('')
  const [file, setFile] = useState<File | null>(null)
  const [titik, setTitik] = useState('sr')
  const [staged, setStaged] = useState<StagedImport | null>(null)
  const [izinkanGanda, setIzinkanGanda] = useState(false)
  const [pesan, setPesan] = useState<string | null>(null)
  const [galat, setGalat] = useState<string | null>(null)
  const [sibuk, setSibuk] = useState(false)

  if (me.isPending) return null
  if (!allowed) {
    return (
      <div className="impor">
        <div className="impor-card">
          <h1>Impor KML</h1>
          <p role="alert" className="panel-error">Halaman ini hanya untuk editor dan admin.</p>
          <a href="/peta">Kembali ke peta</a>
        </div>
      </div>
    )
  }

  const units = (networks.data ?? []).filter((n) => n.unit_spam_id !== null)

  async function unggah() {
    if (!file || networkId === null) return
    setSibuk(true)
    setGalat(null)
    setPesan(null)
    setIzinkanGanda(false)
    try {
      setStaged(await uploadKml(file, titik, networkId, pekerjaanId === '' ? null : Number(pekerjaanId)))
    } catch (err) {
      setStaged(null)
      setGalat((err as Error).message)
    } finally {
      setSibuk(false)
    }
  }

  async function simpan() {
    if (!staged || networkId === null) return
    setSibuk(true)
    setGalat(null)
    try {
      const { ditulis, dilewati } = await commitImport(staged.id, networkId, izinkanGanda)
      setPesan(`${ditulis} fitur disimpan${dilewati > 0 ? `, ${dilewati} dilewati karena di luar batas desa` : ''}.`)
      setStaged(null)
      setFile(null)
      setIzinkanGanda(false)
      qc.invalidateQueries({ queryKey: ['impor-daftar'] })
      qc.invalidateQueries({ queryKey: ['features'] })
    } catch (err) {
      setGalat(pesanGalatSimpan(err))
    } finally {
      setSibuk(false)
    }
  }

  return (
    <div className="impor">
      <div className="impor-card">
        <h1>Impor KML</h1>
        <p>Unggah berkas KML atau KMZ, periksa laporan, lalu simpan ke unit SPAM yang dipilih.</p>

        <div className="impor-field">
          <label htmlFor="impor-unit">Unit SPAM tujuan</label>
          <select id="impor-unit" value={networkId ?? ''} onChange={(e) => { setNetworkId(e.target.value === '' ? null : Number(e.target.value)); setStaged(null) }}>
            <option value="">Pilih unit</option>
            {units.map((n) => <option key={n.id} value={n.id}>{n.nama}</option>)}
          </select>
        </div>
        <div className="impor-field">
          <label htmlFor="impor-pekerjaan">Paket pekerjaan (opsional)</label>
          <select
            id="impor-pekerjaan"
            value={pekerjaanId}
            disabled={pekerjaan.isPending}
            onChange={(e) => { setPekerjaanId(e.target.value); setStaged(null) }}
          >
            <option value="">Tanpa paket</option>
            {(pekerjaan.data ?? []).map((p) => <option key={p.id} value={p.id}>{p.nama_paket} ({p.tahun})</option>)}
          </select>
          {pekerjaan.isError && <small className="impor-muted">Daftar paket belum bisa dimuat. Impor tetap bisa dilakukan tanpa paket.</small>}
        </div>
        <div className="impor-field">
          <label htmlFor="impor-titik">Jenis titik</label>
          <select id="impor-titik" value={titik} onChange={(e) => setTitik(e.target.value)}>
            {JENIS_TITIK.map((t) => <option key={t.value} value={t.value}>{t.label}</option>)}
          </select>
        </div>
        <div className="impor-field">
          <label htmlFor="impor-berkas">Berkas KML atau KMZ</label>
          <input
            id="impor-berkas"
            type="file"
            accept=".kml,.kmz,application/vnd.google-earth.kml+xml,application/vnd.google-earth.kmz"
            onChange={(e) => { setFile(e.target.files?.[0] ?? null); setStaged(null) }}
          />
        </div>
        <div className="impor-aksi">
          <button type="button" className="impor-btn primary" onClick={unggah} disabled={!file || networkId === null || sibuk}>
            {sibuk && !staged ? 'Memeriksa…' : 'Unggah dan periksa'}
          </button>
          <a className="impor-btn" href="/peta" style={{ display: 'inline-flex', alignItems: 'center' }}>Kembali ke peta</a>
        </div>

        {staged && (
          <div className="impor-report" aria-label="Pratinjau impor">
            {staged.duplikat && (
              <div className="impor-peringatan" role="alert">
                <p>{pesanDuplikat(staged.duplikat)}</p>
                <label className="impor-cek">
                  <input type="checkbox" checked={izinkanGanda} onChange={(e) => setIzinkanGanda(e.target.checked)} />
                  Impor lagi walaupun ganda
                </label>
              </div>
            )}
            <p>
              <strong>{staged.nama_file}</strong>: {staged.laporan.jumlah_fitur} fitur siap disimpan
              {staged.laporan.bukan_fitur > 0 && `, ${staged.laporan.bukan_fitur} gambar dilewati`}.
            </p>
            {staged.laporan.dalam_batas !== undefined && (
              <p>
                Di dalam batas desa: <strong>{staged.laporan.dalam_batas}</strong>
                {(staged.laporan.luar_batas ?? 0) > 0 && <> · di luar: <strong>{staged.laporan.luar_batas}</strong> (tidak disimpan)</>}
                {(staged.laporan.tanpa_batas ?? 0) > 0 && <> · desa belum punya batas: {staged.laporan.tanpa_batas}</>}
              </p>
            )}
            <ul>
              {Object.entries(staged.laporan.per_jenis).map(([jenis, n]) => (
                <li key={jenis}>{LABEL_JENIS[jenis as FeatureType] ?? jenis}: {n}</li>
              ))}
            </ul>
            {staged.laporan.dilewati.length > 0 && (
              <details>
                <summary>{staged.laporan.dilewati.length} item dilewati</summary>
                <ul>
                  {staged.laporan.dilewati.map((s, i) => <li key={i}>{s.nama ?? '(tanpa nama)'}: {s.alasan}</li>)}
                </ul>
              </details>
            )}
            <div className="impor-aksi">
              <button type="button" className="impor-btn primary" onClick={simpan} disabled={sibuk}>
                {sibuk ? 'Menyimpan…' : 'Simpan ke jaringan'}
              </button>
            </div>
          </div>
        )}

        {galat && <p role="alert" className="panel-error">{galat}</p>}
        {pesan && <p role="status">{pesan}</p>}
      </div>

      <ImporDaftar />
    </div>
  )
}
