import { useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { commitImport, uploadKml, type StagedImport } from './importApi'

const POINT_TYPES = [
  { value: 'sr', label: 'Sambungan rumah (sr)' },
  { value: 'valve', label: 'Valve' },
  { value: 'reservoir', label: 'Reservoir' },
  { value: 'bronkaptering', label: 'Bronkaptering' },
]

const LABEL: Record<string, string> = {
  pipa: 'Pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
}

type Props = {
  networkId: number | null
  open: boolean
  onClose: () => void
}

/** Impor KML untuk editor dan admin: unggah, periksa laporan, lalu simpan ke jaringan. */
export function ImportPanel({ networkId, open, onClose }: Props) {
  const queryClient = useQueryClient()
  const [file, setFile] = useState<File | null>(null)
  const [titik, setTitik] = useState('sr')
  const [staged, setStaged] = useState<StagedImport | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  async function handleUpload() {
    if (!file) return
    setBusy(true)
    setMessage(null)
    try {
      if (networkId === null) throw new Error('Pilih unit SPAM tujuan di header dulu.')
      setStaged(await uploadKml(file, titik, networkId))
    } catch (err) {
      setStaged(null)
      setMessage((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  async function handleCommit() {
    if (!staged || networkId === null) return
    setBusy(true)
    setMessage(null)
    try {
      const { ditulis, dilewati } = await commitImport(staged.id, networkId)
      setMessage(`${ditulis} fitur disimpan${dilewati > 0 ? `, ${dilewati} dilewati karena di luar batas desa` : ''}.`)
      setStaged(null)
      setFile(null)
      await queryClient.invalidateQueries({ queryKey: ['features'] })
    } catch (err) {
      setMessage((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  if (!open) return null

  return (
    <div className="import">
      <div className="import-row">
        <input type="file" accept=".kml,application/vnd.google-earth.kml+xml" onChange={(e) => setFile(e.target.files?.[0] ?? null)} />
        <select aria-label="Jenis titik" value={titik} onChange={(e) => setTitik(e.target.value)}>
          {POINT_TYPES.map((t) => (
            <option key={t.value} value={t.value}>{t.label}</option>
          ))}
        </select>
        <button type="button" onClick={handleUpload} disabled={!file || busy}>
          {busy && !staged ? 'Memeriksa…' : 'Unggah & periksa'}
        </button>
        <button type="button" onClick={() => { onClose(); setStaged(null); setMessage(null) }}>Tutup</button>
      </div>

      {staged && (
        <div className="import-report">
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
              <li key={jenis}>{LABEL[jenis] ?? jenis}: {n}</li>
            ))}
          </ul>
          {staged.laporan.dilewati.length > 0 && (
            <details>
              <summary>{staged.laporan.dilewati.length} item dilewati</summary>
              <ul>
                {staged.laporan.dilewati.map((s, i) => (
                  <li key={i}>{s.nama ?? '(tanpa nama)'}: {s.alasan}</li>
                ))}
              </ul>
            </details>
          )}
          <button type="button" onClick={handleCommit} disabled={busy || networkId === null}>
            {busy ? 'Menyimpan…' : 'Simpan ke jaringan'}
          </button>
        </div>
      )}

      {message && <p className="import-message" role="status">{message}</p>}
    </div>
  )
}
