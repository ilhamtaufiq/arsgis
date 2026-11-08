import { useQueryClient } from '@tanstack/react-query'
import { logout, type User } from './api'

type Props = {
  user: User | null
  /** Tampil polos di navbar, bukan sebagai panel mengambang di atas peta. */
  inline?: boolean
}

/** Tombol masuk, atau nama dan peran pengguna dengan tombol keluar. Masuk dilakukan di halaman /masuk. */
export function AuthPanel({ user, inline = false }: Props) {
  const className = inline ? 'auth auth-inline' : 'auth'
  const queryClient = useQueryClient()

  // Setelah keluar, data peta dan sesi dimuat ulang karena hasilnya bergantung pada peran.
  async function handleLogout() {
    await logout()
    await Promise.all([queryClient.invalidateQueries({ queryKey: ['me'] }), queryClient.invalidateQueries({ queryKey: ['features'] })])
  }

  if (user) {
    return (
      <div className={className}>
        <span className="auth-who">
          {user.username} <small>({user.role})</small>
        </span>
        <button type="button" onClick={handleLogout}>Keluar</button>
      </div>
    )
  }

  // Kembali ke halaman ini setelah masuk. Hanya path di dalam aplikasi yang dipakai.
  function goToLogin() {
    const next = window.location.pathname + window.location.search
    window.location.assign(`/masuk?next=${encodeURIComponent(next)}`)
  }

  return (
    <div className={className}>
      <button type="button" className="auth-primary" onClick={goToLogin}>Masuk</button>
    </div>
  )
}
