import type { ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchMe } from './api'
import { AuthPanel } from './Auth'

type Props = {
  /** Judul halaman yang ditampilkan di kartu saat belum masuk. */
  judul: string
  children: ReactNode
}

/** Membungkus halaman yang butuh login. Tampilan saat belum masuk sama dengan gerbang peta sebelumnya. */
export function LoginGate({ judul, children }: Props) {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })

  if (me.isPending) return null
  if (me.data) return <>{children}</>

  return (
    <div className="gate">
      <div className="gate-card">
        <p className="gate-title">{judul}</p>
        <p className="gate-sub">Masuk untuk membuka halaman ini.</p>
        <AuthPanel user={null} inline />
      </div>
    </div>
  )
}
