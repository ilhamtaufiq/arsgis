import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { login, logout, type User } from './api'

type Props = {
  user: User | null
}

/** Tombol masuk, atau nama dan peran pengguna dengan tombol keluar. */
export function AuthPanel({ user }: Props) {
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  // Setelah masuk atau keluar, data peta dan sesi dimuat ulang karena hasilnya bergantung pada peran.
  const refresh = () => Promise.all([queryClient.invalidateQueries({ queryKey: ['me'] }), queryClient.invalidateQueries({ queryKey: ['features'] })])

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      await login(username.trim(), password)
      setPassword('')
      setOpen(false)
      await refresh()
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  async function handleLogout() {
    await logout()
    await refresh()
  }

  if (user) {
    return (
      <div className="auth">
        <span className="auth-who">
          {user.username} <small>({user.role})</small>
        </span>
        <button type="button" onClick={handleLogout}>Keluar</button>
      </div>
    )
  }

  if (!open) {
    return (
      <div className="auth">
        <button type="button" onClick={() => setOpen(true)}>Masuk</button>
      </div>
    )
  }

  return (
    <form className="auth auth-form" onSubmit={handleSubmit}>
      <input
        aria-label="Username"
        placeholder="Username"
        autoComplete="username"
        value={username}
        onChange={(e) => setUsername(e.target.value)}
        required
      />
      <input
        aria-label="Password"
        type="password"
        placeholder="Password"
        autoComplete="current-password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        required
      />
      <button type="submit" disabled={busy}>{busy ? 'Memproses…' : 'Masuk'}</button>
      <button type="button" onClick={() => { setOpen(false); setError(null) }}>Batal</button>
      {error && <p className="auth-error" role="alert">{error}</p>}
    </form>
  )
}
