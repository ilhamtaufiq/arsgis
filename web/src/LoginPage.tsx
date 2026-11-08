import { useState, type FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { login } from './api'

// Hanya path di dalam aplikasi yang boleh dituju setelah masuk, supaya tidak bisa diarahkan ke situs lain.
function aman(next: string | null): string {
  return next && next.startsWith('/') && !next.startsWith('//') ? next : '/'
}

/** Halaman masuk. Setelah berhasil, pengguna dikembalikan ke halaman asal lewat ?next=. */
export function LoginPage() {
  const queryClient = useQueryClient()
  const next = aman(new URLSearchParams(window.location.search).get('next'))
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      await login(username.trim(), password)
      await queryClient.invalidateQueries({ queryKey: ['me'] })
      window.location.replace(next)
    } catch (err) {
      setError((err as Error).message)
      setPassword('')
      setBusy(false)
    }
  }

  return (
    <div className="gate">
      <form className="gate-card" onSubmit={handleSubmit}>
        <p className="gate-title">Masuk ke arsgis</p>
        <p className="gate-sub">Data sambungan rumah hanya untuk editor dan admin.</p>

        <label className="field">
          <span>Username</span>
          <input autoFocus autoComplete="username" value={username} onChange={(e) => setUsername(e.target.value)} required />
        </label>
        <label className="field">
          <span>Password</span>
          <input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} required />
        </label>

        {error && <p className="modal-error" role="alert">{error}</p>}

        <button type="submit" className="modal-submit" disabled={busy}>
          {busy ? 'Memproses…' : 'Masuk'}
        </button>
        <a href={next} className="gate-back">Kembali</a>
      </form>
    </div>
  )
}
