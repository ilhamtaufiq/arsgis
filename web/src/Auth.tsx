import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { login, logout, type User } from './api'

type Props = {
  user: User | null
}

/** Tombol masuk, atau nama dan peran pengguna dengan tombol keluar. Form login tampil sebagai modal. */
export function AuthPanel({ user }: Props) {
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const usernameRef = useRef<HTMLInputElement>(null)

  // Setelah masuk atau keluar, data peta dan sesi dimuat ulang karena hasilnya bergantung pada peran.
  const refresh = () => Promise.all([queryClient.invalidateQueries({ queryKey: ['me'] }), queryClient.invalidateQueries({ queryKey: ['features'] })])

  function close() {
    setOpen(false)
    setError(null)
  }

  useEffect(() => {
    if (!open) return
    usernameRef.current?.focus()
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open])

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

  return (
    <>
      <div className="auth">
        <button type="button" className="auth-primary" onClick={() => setOpen(true)}>Masuk</button>
      </div>

      {open && (
        <div className="modal-backdrop" onMouseDown={close}>
          <form
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="login-title"
            onMouseDown={(e) => e.stopPropagation()}
            onSubmit={handleSubmit}
          >
            <div className="modal-head">
              <h2 id="login-title">Masuk ke arsgis</h2>
              <button type="button" className="modal-close" aria-label="Tutup" onClick={close}>×</button>
            </div>
            <p className="modal-sub">Data sambungan rumah hanya untuk editor dan admin.</p>

            <label className="field">
              <span>Username</span>
              <input
                ref={usernameRef}
                autoComplete="username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
              />
            </label>
            <label className="field">
              <span>Password</span>
              <input
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </label>

            {error && <p className="modal-error" role="alert">{error}</p>}

            <button type="submit" className="modal-submit" disabled={busy}>
              {busy ? 'Memproses…' : 'Masuk'}
            </button>
          </form>
        </div>
      )}
    </>
  )
}
