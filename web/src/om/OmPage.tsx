import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchMe, fetchNetworks } from '../api'
import { GangguanPanel } from './GangguanPanel'
import { JatuhTempoPanel } from './JatuhTempoPanel'
import { RehabPanel } from './RehabPanel'
import { bolehCatat } from './omLogic'
import './om.css'

type Tab = 'gangguan' | 'jatuh-tempo' | 'rehab'

const TAB: { id: Tab; label: string }[] = [
  { id: 'gangguan', label: 'Laporan gangguan' },
  { id: 'jatuh-tempo', label: 'Jatuh tempo perawatan' },
  { id: 'rehab', label: 'Prioritas rehabilitasi' },
]

/** Halaman operasi dan pemeliharaan. Hanya untuk pengguna yang sudah masuk (dibungkus LoginGate). */
export function OmPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const units = (networks.data ?? []).filter((n) => n.unit_spam_id !== null)
  const dariUrl = Number(new URLSearchParams(window.location.search).get('unit'))
  const [pilihan, setPilihan] = useState<number | null>(null)
  const [tab, setTab] = useState<Tab>('gangguan')

  const networkId = pilihan ?? (units.some((u) => u.id === dariUrl) ? dariUrl : (units[0]?.id ?? null))
  const bisaCatat = bolehCatat(me.data?.role)

  return (
    <main className="om-halaman">
      <header className="om-header">
        <div>
          <p className="om-kicker">Operasi dan pemeliharaan</p>
          <h1>Gangguan, perawatan, dan rehabilitasi</h1>
        </div>
        <label className="om-field">
          Unit SPAM
          <select value={networkId ?? ''} onChange={(e) => setPilihan(Number(e.target.value))} disabled={units.length === 0}>
            {units.map((u) => (
              <option key={u.id} value={u.id}>
                {u.nama}
              </option>
            ))}
          </select>
        </label>
      </header>

      {networks.isError && <p role="alert" className="om-error">{(networks.error as Error).message}</p>}
      {networkId === null && networks.isSuccess && <p className="om-muted">Belum ada unit SPAM.</p>}

      {networkId !== null && (
        <>
          <nav className="om-tab" aria-label="Bagian operasi dan pemeliharaan">
            {TAB.map((t) => (
              <button key={t.id} type="button" aria-pressed={tab === t.id} onClick={() => setTab(t.id)}>
                {t.label}
              </button>
            ))}
          </nav>
          {tab === 'gangguan' && <GangguanPanel networkId={networkId} bisaCatat={bisaCatat} />}
          {tab === 'jatuh-tempo' && <JatuhTempoPanel networkId={networkId} bisaCatat={bisaCatat} />}
          {tab === 'rehab' && <RehabPanel networkId={networkId} />}
        </>
      )}
    </main>
  )
}
