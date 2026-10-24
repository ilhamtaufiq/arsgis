import { useState, type FormEvent } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft } from 'lucide-react'
import { fetchMe, fetchNetworks, type Role } from './api'
import { AuthPanel } from './Auth'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

type AdminUser = { id: number; username: string; role: Role; active: boolean; created_at: string }
type AdminImport = { id: number; nama_file: string; status: string; network_id: number | null; dibuat_oleh: string; jumlah_fitur: number | null; waktu: string }

const ROLES: Role[] = ['viewer', 'editor', 'admin']

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

/** Halaman admin: kelola pengguna, jaringan, dan memantau impor. Hanya untuk admin. */
export function AdminPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const user = me.data ?? null

  return (
    <div className="min-h-full bg-[#EBEBEB] text-foreground">
      <div className="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-6">
        <header className="flex flex-wrap items-center gap-3">
          <Button variant="outline" size="sm" asChild className="rounded-full bg-white">
            <a href="/">
              <ArrowLeft />
              Peta
            </a>
          </Button>
          <div className="min-w-0 flex-1">
            <h1 className="text-xl font-semibold tracking-tight">Admin arsgis</h1>
            <p className="text-sm text-muted-foreground">Kelola pengguna, jaringan, dan impor</p>
          </div>
          <Button variant="outline" size="sm" asChild className="rounded-full bg-white">
            <a href="/dashboard">Dashboard</a>
          </Button>
          <AuthPanel user={user} />
        </header>

        {me.isPending ? null : user?.role !== 'admin' ? (
          <Card>
            <CardContent className="py-6 text-sm text-muted-foreground">
              {user ? 'Halaman ini hanya untuk admin.' : 'Masuk dengan akun admin untuk membuka halaman ini.'}
            </CardContent>
          </Card>
        ) : (
          <Tabs defaultValue="pengguna">
            <TabsList>
              <TabsTrigger value="pengguna">Pengguna</TabsTrigger>
              <TabsTrigger value="jaringan">Jaringan</TabsTrigger>
              <TabsTrigger value="impor">Impor</TabsTrigger>
            </TabsList>
            <TabsContent value="pengguna" className="flex flex-col gap-4">
              <UsersTab currentUser={user.username} />
            </TabsContent>
            <TabsContent value="jaringan" className="flex flex-col gap-4">
              <NetworksTab />
            </TabsContent>
            <TabsContent value="impor" className="flex flex-col gap-4">
              <ImportsTab />
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  )
}

function RoleSelect({ value, label, onChange }: { value: Role; label: string; onChange: (r: Role) => void }) {
  return (
    <Select value={value} onValueChange={(v) => onChange(v as Role)}>
      <SelectTrigger size="sm" className="w-28" aria-label={label}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {ROLES.map((r) => (
          <SelectItem key={r} value={r}>
            {r}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
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
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Buat pengguna</CardTitle>
          <CardDescription>Password minimal 10 karakter. Akun baru langsung bisa dipakai untuk masuk.</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="grid gap-3 sm:grid-cols-[1fr_1fr_auto_auto] sm:items-end" onSubmit={handleCreate}>
            <div className="grid gap-1.5">
              <Label htmlFor="new-username">Username</Label>
              <Input id="new-username" value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} required />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="new-password">Password</Label>
              <Input id="new-password" type="password" value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} required />
            </div>
            <div className="grid gap-1.5">
              <Label>Peran</Label>
              <RoleSelect value={form.role} label="Peran baru" onChange={(role) => setForm({ ...form, role })} />
            </div>
            <Button type="submit" disabled={create.isPending}>
              Buat
            </Button>
          </form>
          {error && <p className="mt-3 text-sm text-red-700">{error}</p>}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Daftar pengguna</CardTitle>
        </CardHeader>
        <CardContent>
          {users.data && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Username</TableHead>
                  <TableHead>Peran</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Dibuat</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {users.data.map((u) => (
                  <TableRow key={u.id}>
                    <TableCell className="font-medium">
                      {u.username}
                      {u.username === currentUser && <span className="ml-1.5 text-xs text-muted-foreground">(Anda)</span>}
                    </TableCell>
                    <TableCell>
                      <RoleSelect value={u.role} label={`Peran ${u.username}`} onChange={(role) => update.mutate({ id: u.id, patch: { role } })} />
                    </TableCell>
                    <TableCell>
                      <Button
                        size="sm"
                        variant={u.active ? 'secondary' : 'outline'}
                        className="rounded-full"
                        onClick={() => update.mutate({ id: u.id, patch: { active: !u.active } })}
                      >
                        {u.active ? 'Aktif' : 'Nonaktif'}
                      </Button>
                    </TableCell>
                    <TableCell className="text-muted-foreground">{u.created_at}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
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
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Buat jaringan</CardTitle>
        </CardHeader>
        <CardContent>
          <form className="grid gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end" onSubmit={(e) => { e.preventDefault(); create.mutate() }}>
            <div className="grid gap-1.5">
              <Label htmlFor="net-nama">Nama jaringan</Label>
              <Input id="net-nama" value={nama} onChange={(e) => setNama(e.target.value)} required />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="net-deskripsi">Deskripsi (opsional)</Label>
              <Input id="net-deskripsi" value={deskripsi} onChange={(e) => setDeskripsi(e.target.value)} />
            </div>
            <Button type="submit" disabled={create.isPending}>
              Buat
            </Button>
          </form>
          {error && <p className="mt-3 text-sm text-red-700">{error}</p>}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Daftar jaringan</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Nama</TableHead>
                <TableHead>Deskripsi</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {networks.data?.map((n) => (
                <TableRow key={n.id}>
                  <TableCell className="font-medium">{n.nama}</TableCell>
                  <TableCell className="text-muted-foreground">{n.deskripsi ?? '–'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </>
  )
}

function ImportsTab() {
  const imports = useQuery({
    queryKey: ['admin-imports'],
    queryFn: () => call<{ data: AdminImport[] }>('GET', '/api/v1/admin/imports').then((r) => r.data),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Semua impor</CardTitle>
        <CardDescription>Termasuk batch yang belum disimpan ke jaringan.</CardDescription>
      </CardHeader>
      <CardContent>
        {imports.data?.length === 0 && <p className="text-sm text-muted-foreground">Belum ada impor.</p>}
        {imports.data && imports.data.length > 0 && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Berkas</TableHead>
                <TableHead>Oleh</TableHead>
                <TableHead className="text-right">Fitur</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Waktu</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {imports.data.map((i) => (
                <TableRow key={i.id}>
                  <TableCell className="max-w-56 truncate">{i.nama_file}</TableCell>
                  <TableCell>{i.dibuat_oleh}</TableCell>
                  <TableCell className="text-right tabular-nums">{i.jumlah_fitur ?? '–'}</TableCell>
                  <TableCell>
                    <Badge variant={i.status === 'committed' ? 'default' : 'secondary'}>
                      {i.status === 'committed' ? 'Disimpan' : 'Menunggu'}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground">{i.waktu}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  )
}
