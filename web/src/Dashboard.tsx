import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Map as MapIcon, Ruler, Layers } from 'lucide-react'
import { fetchMe, fetchNetworks } from './api'
import { AuthPanel } from './Auth'
import { fetchDashboard } from './dashboardApi'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const JENIS: Record<string, { label: string; color: string }> = {
  pipa: { label: 'Pipa', color: '#1d4ed8' },
  valve: { label: 'Valve', color: '#111111' },
  reservoir: { label: 'Reservoir', color: '#0f766e' },
  sr: { label: 'Sambungan rumah', color: '#a16207' },
  bronkaptering: { label: 'Bronkaptering', color: '#0891b2' },
}

const FUNGSI: Record<string, string> = { transmisi: 'Transmisi', distribusi: 'Distribusi' }

const nf = (n: number, digits = 0) => n.toLocaleString('id-ID', { maximumFractionDigits: digits })

/** Halaman dashboard jaringan, dibuka di /dashboard. */
export function DashboardPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const [chosen, setChosen] = useState<number | null>(null)
  const networkId = chosen ?? networks.data?.[0]?.id ?? null

  const data = useQuery({
    queryKey: ['dashboard', networkId],
    queryFn: () => fetchDashboard(networkId!),
    enabled: networkId !== null,
  })

  const fitur = data.data?.fitur ?? []
  const total = fitur.reduce((sum, f) => sum + f.jumlah, 0)
  const pipa = data.data?.pipa ?? []
  const totalKm = pipa.reduce((sum, p) => sum + p.km, 0)

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
            <h1 className="text-xl font-semibold tracking-tight">Dashboard jaringan</h1>
            {networks.data && networks.data.length > 1 ? (
              <Select value={String(networkId ?? '')} onValueChange={(v) => setChosen(Number(v))}>
                <SelectTrigger size="sm" className="mt-1 w-64 rounded-full bg-white" aria-label="Pilih jaringan">
                  <SelectValue placeholder="Pilih jaringan" />
                </SelectTrigger>
                <SelectContent>
                  {networks.data.map((n) => (
                    <SelectItem key={n.id} value={String(n.id)}>
                      {n.nama}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <p className="text-sm text-muted-foreground">{data.data?.jaringan ?? 'Memuat jaringan…'}</p>
            )}
          </div>
          <Button variant="outline" size="sm" asChild className="rounded-full bg-white">
            <a href="/admin">Admin</a>
          </Button>
          <AuthPanel user={me.data ?? null} />
        </header>

        {data.isError && (
          <p className="rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700">{(data.error as Error).message}</p>
        )}

        <section className="grid gap-4 sm:grid-cols-3">
          <Card className="border-primary/20 bg-accent">
            <CardHeader>
              <CardDescription className="flex items-center gap-2">
                <Layers className="size-4" /> Total fitur
              </CardDescription>
              <CardTitle className="text-3xl">{nf(total)}</CardTitle>
            </CardHeader>
          </Card>

          <Card>
            <CardHeader>
              <CardDescription className="flex items-center gap-2">
                <Ruler className="size-4" /> Panjang pipa
              </CardDescription>
              <CardTitle className="text-3xl">{nf(totalKm, 2)} km</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-1 text-sm text-muted-foreground">
              {pipa.map((p) => (
                <span key={p.fungsi}>
                  {FUNGSI[p.fungsi] ?? p.fungsi}: {nf(p.km, 2)} km · {nf(p.jumlah)} ruas
                </span>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardDescription className="flex items-center gap-2">
                <MapIcon className="size-4" /> Jenis fitur
              </CardDescription>
              <CardTitle className="text-3xl">{fitur.length}</CardTitle>
            </CardHeader>
          </Card>
        </section>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Fitur per jenis</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Jenis</TableHead>
                  <TableHead className="text-right">Jumlah</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {fitur.map((f) => (
                  <TableRow key={f.feature_type}>
                    <TableCell>
                      <span className="inline-flex items-center gap-2">
                        <span className="size-2.5 rounded-full" style={{ background: JENIS[f.feature_type]?.color ?? '#6b7280' }} />
                        {JENIS[f.feature_type]?.label ?? f.feature_type}
                      </span>
                    </TableCell>
                    <TableCell className="text-right tabular-nums">{nf(f.jumlah)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            {data.data && !data.data.bisa_melihat_sr && (
              <p className="mt-3 text-sm text-muted-foreground">Sambungan rumah hanya terlihat untuk editor dan admin.</p>
            )}
          </CardContent>
        </Card>

        {data.data?.bisa_melihat_sr && (
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Impor terakhir</CardTitle>
            </CardHeader>
            <CardContent>
              {data.data.impor_terakhir.length === 0 ? (
                <p className="text-sm text-muted-foreground">Belum ada impor.</p>
              ) : (
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
                    {data.data.impor_terakhir.map((i) => (
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
        )}
      </div>
    </div>
  )
}
