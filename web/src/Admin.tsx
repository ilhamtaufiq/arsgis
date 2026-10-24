import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { fetchMe, fetchNetworks, type Role } from './api'
import { AuthPanel } from './Auth'

type AdminUser = { id: number; username: string; role: Role; active: boolean; created_at: string }
type AdminImport = { id: number; nama_file: string; status: string; network_id: number | null; dibuat_oleh: string; jumlah_fitur: number | null; waktu: string }

async function call<T>(method: string, url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    headers: body ? { 'content-type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) {
    const err = (await res.json().catch(() => ({}))) as { error?: string }
    throw new Error(err.error ?? `Permintaan gagal (${res.status})`)
  }
  return res.json() as Promise<T>
}

type Tab = 'pengguna' | 'jaringan' | 'impor'

/** Halaman admin: kelola pengguna, jaringan, dan memantau impor. Hanya untuk admin. */
export function AdminPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const [tab, setTab] = useState<Tab>('pengguna')
  const user = me.data ?? null

  return (
    <div className="dash-page">
      <header className="dash-page-head">
        <a className="back" href="/">← Peta</a>
        <div className="dash-title">
          <h1>Admin</h1>
          <a className="back" href="/dashboard">Dashboard jaringan</a>
        </div>
        <div className="dash-auth">
          <AuthPanel user={user} />
        </div>
      </header>

      {me.isPending ? null : user?.role !== 'admin' ? (
        <section className="dash-block">
          <p className="dash-note">
            {user ? 'Halaman ini hanya untuk admin.' : 'Masuk dengan akun admin untuk membuka halaman ini.'}
          </p>
        </section>
      ) : (
        <>
          <nav className="tabs" role="tablist">
            {(['pengguna', 'jaringan', 'impor'] as Tab[]).map((t) => (
              <button key={t} type="button" role="tab" aria-selected={tab === t} className={tab === t ? 'is-active' : ''} onClick={() => setTab(t)}>
                {t === 'pengguna' ? 'Pengguna' : t === 'jaringan' ? 'Jaringan' : 'Impor'}
              </button>
            ))}
          </nav>
          {tab === 'pengguna' && <UsersTab currentUser={user.username} />}
          {tab === 'jaringan' && <NetworksTab />}
          {tab === 'impor' && <ImportsTab />}
        </>
      )}
    </div>
  )
}

function UsersTab({ currentUser }: { currentUser: string }) {
  const queryClient = useQueryClient()
  const users = useQuery({
    queryKey: ['admin-users'],
    queryFn: () => call<{ data: AdminUser[] }>('GET', '/api/v1/admin/users').then((r) => r.data),
  })
  const [error, setError] = useState<string | null>(null)
  const [form, setForm] = useState({ username: '', password: '', role: 'viewer' as Role })

  const update = useMutation({
    mutationFn: (args: { id: number; patch: { role?: Role; active?: boolean } }) =>
      call('PATCH', `/api/v1/admin/users/${args.id}`, args.patch),
    onSuccess: () => {
      setError(null)
      queryClient.invalidateQueries({ queryKey: ['admin-users'] })
    },
    onError: (e) => setError((e as Error).message),
  })

  const create = useMutation({
    mutationFn: () => call('POST', '/api/v1/admin/users', form),
    onSuccess: () => {
      setError(null)
      setForm({ username: '', password: '', role: 'viewer' })
      queryClient.invalidateQueries({ queryKey: ['admin-users'] })
    },
    onError: (e) => setError((e as Error).message),
  })

  function handleCreate(e: FormEvent) {
    e.preventDefault()
    create.mutate()
  }

  return (
    <>
      <section className="dash-block">
        <h3 className="dash-section">Buat pengguna</h3>
        <form className="admin-form" onSubmit={handleCreate}>
          <input placeholder="Username" aria-label="Username baru" value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} required />
          <input type="password" placeholder="Password (min. 10)" aria-label="Password baru" value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} required />
          <select aria-label="Peran baru" value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value as Role })}>
            <option value="viewer">viewer</option>
            <option value="editor">editor</option>
            <option value="admin">admin</option>
          </select>
          <button type="submit" className="admin-primary" disabled={create.isPending}>Buat</button>
        </form>
      </section>

      {error && <p className="modal-error">{error}</p>}

      <section className="dash-block">
        <h3 className="dash-section">Daftar pengguna</h3>
        {users.data && (
          <div className="dash-table-wrap">
            <table className="dash-table">
              <thead>
                <tr>
                  <th>Username</th>
                  <th>Peran</th>
                  <th>Status</th>
                  <th>Dibuat</th>
                </tr>
              </thead>
              <tbody>
                {users.data.map((u) => (
                  <tr key={u.id}>
                    <td>{u.username}{u.username === currentUser && <small className="you"> (Anda)</small>}</td>
                    <td>
                      <select aria-label={`Peran ${u.username}`} value={u.role} onChange={(e) => update.mutate({ id: u.id, patch: { role: e.target.value as Role } })}>
                        <option value="viewer">viewer</option>
                        <option value="editor">editor</option>
                        <option value="admin">admin</option>
                      </select>
                    </td>
                    <td>
                      <button type="button" className={`pill ${u.active ? 'on' : 'off'}`} onClick={() => update.mutate({ id: u.id, patch: { active: !u.active } })}>
                        {u.active ? 'Aktif' : 'Nonaktif'}
                      </button>
                    </td>
                    <td>{u.created_at}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  )
}

function NetworksTab() {
  const queryClient = useQueryClient()
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const [nama, setNama] = useState('')
  const [deskripsi, setDeskripsi] = useState('')
  const [error, setError] = useState<string | null>(null)

  const create = useMutation({
    mutationFn: () => call('POST', '/api/v1/admin/networks', { nama, deskripsi: deskripsi || null }),
    onSuccess: () => {
      setError(null)
      setNama('')
      setDeskripsi('')
      queryClient.invalidateQueries({ queryKey: ['networks'] })
    },
    onError: (e) => setError((e as Error).message),
  })

  return (
    <>
      <section className="dash-block">
        <h3 className="dash-section">Buat jaringan</h3>
        <form className="admin-form" onSubmit={(e) => { e.preventDefault(); create.mutate() }}>
          <input placeholder="Nama jaringan" aria-label="Nama jaringan" value={nama} onChange={(e) => setNama(e.target.value)} required />
          <input placeholder="Deskripsi (opsional)" aria-label="Deskripsi jaringan" value={deskripsi} onChange={(e) => setDeskripsi(e.target.value)} />
          <button type="submit" className="admin-primary" disabled={create.isPending}>Buat</button>
        </form>
        {error && <p className="modal-error">{error}</p>}
      </section>

      <section className="dash-block">
        <h3 className="dash-section">Daftar jaringan</h3>
        <ul className="dash-list">
          {networks.data?.map((n) => (
            <li key={n.id}>
              <span>{n.nama}</span>
              <small className="muted">{n.deskripsi ?? ''}</small>
            </li>
          ))}
        </ul>
      </section>
    </>
  )
}

function ImportsTab() {
  const imports = useQuery({
    queryKey: ['admin-imports'],
    queryFn: () => call<{ data: AdminImport[] }>('GET', '/api/v1/admin/imports').then((r) => r.data),
  })

  return (
    <section className="dash-block">
      <h3 className="dash-section">Semua impor</h3>
      {imports.data?.length === 0 && <p className="dash-note">Belum ada impor.</p>}
      {imports.data && imports.data.length > 0 && (
        <div className="dash-table-wrap">
          <table className="dash-table">
            <thead>
              <tr>
                <th>Berkas</th>
                <th>Oleh</th>
                <th>Fitur</th>
                <th>Status</th>
                <th>Waktu</th>
              </tr>
            </thead>
            <tbody>
              {imports.data.map((i) => (
                <tr key={i.id}>
                  <td>{i.nama_file}</td>
                  <td>{i.dibuat_oleh}</td>
                  <td>{i.jumlah_fitur ?? '–'}</td>
                  <td>{i.status === 'committed' ? 'Disimpan' : 'Menunggu'}</td>
                  <td>{i.waktu}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
