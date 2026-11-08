import './preflight.css'
import { useQuery } from '@tanstack/react-query'
import { fetchMe } from './api'
import { AuthPanel } from './Auth'
import { fetchDesaDetail } from './desaDetailApi'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const nf = (n: number) => n.toLocaleString('id-ID')

export function DesaDetailPage({ id }: { id: number }) {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const detail = useQuery({
    queryKey: ['desa-detail', id],
    queryFn: () => fetchDesaDetail(id),
    enabled: Number.isFinite(id) && id > 0,
  })

  const d = detail.data
  return (
    <div className="min-h-screen bg-muted/30 p-4 md:p-6">
      <header className="mb-4 flex items-center justify-between gap-3">
        <a href="/peta" className="text-sm text-muted-foreground hover:underline">← Kembali ke peta</a>
        <AuthPanel user={me.data ?? null} />
      </header>

      {detail.isLoading && <p className="text-sm text-muted-foreground">Memuat detail desa…</p>}
      {detail.isError && (
        <Card><CardContent className="p-6 text-sm text-destructive">{(detail.error as Error).message}</CardContent></Card>
      )}

      {d && (
        <div className="grid gap-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-2xl">{d.summary.nama}</CardTitle>
              <CardDescription>
                Kecamatan {d.summary.kecamatan}
                {d.summary.kode_bps ? ` · kode BPS ${d.summary.kode_bps}` : ''}
              </CardDescription>
            </CardHeader>
            <CardContent className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <Stat label="Cakupan SPM" value={d.summary.cakupan_persen === null ? 'Tanpa target' : `${d.summary.cakupan_persen}%`} />
              <Stat label="Target" value={`${nf(d.summary.target_kk)} KK`} />
              <Stat label="JP (perpipaan)" value={`${nf(d.summary.jp_kk)} KK`} />
              <Stat label="BJP" value={`${nf(d.summary.bjp_kk)} KK`} />
              <Stat label="Sambungan rumah" value={nf(d.summary.sr)} />
              <Stat label="Jiwa terlayani" value={nf(d.summary.jiwa)} />
              <Stat label="Unit SPAM" value={nf(d.summary.unit_count)} />
              <Stat label="Batas desa" value={d.summary.punya_batas ? 'Ada' : 'Belum ada'} />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Unit SPAM</CardTitle>
              <CardDescription>{d.units.length} unit aktif di desa ini</CardDescription>
            </CardHeader>
            <CardContent>
              {d.units.length === 0 ? (
                <p className="text-sm text-muted-foreground">Belum ada unit SPAM di desa ini.</p>
              ) : (
                <div className="overflow-x-auto">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Nama unit</TableHead>
                        <TableHead>Sistem layanan</TableHead>
                        <TableHead>Sumber dana</TableHead>
                        <TableHead className="text-right">SR</TableHead>
                        <TableHead className="text-right">KK</TableHead>
                        <TableHead className="text-right">Jiwa</TableHead>
                        <TableHead className="text-right">BJP KK</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {d.units.map((u) => (
                        <TableRow key={u.id}>
                          <TableCell className="font-medium">
                            {u.nama ?? `Unit #${u.id}`}
                            {u.is_simspam && <Badge variant="secondary" className="ml-2">SIMSPAM</Badge>}
                          </TableCell>
                          <TableCell>{u.sistem_layanan ?? '–'}</TableCell>
                          <TableCell>{u.sumber_dana ?? '–'}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.sr)}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.kk)}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.jiwa)}</TableCell>
                          <TableCell className="text-right tabular-nums">{nf(u.bjp_kk)}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Pokmas</CardTitle>
              <CardDescription>Kelompok masyarakat pengelola SPAM di desa ini</CardDescription>
            </CardHeader>
            <CardContent>
              {!d.pokmas ? (
                <p className="text-sm text-muted-foreground">Profil Pokmas belum terisi.</p>
              ) : (
                <div className="grid gap-3 md:grid-cols-2">
                  <Field label="Nama Pokmas" value={d.pokmas.nama} />
                  <Field label="Perdes" value={d.pokmas.perdes} />
                  {d.pokmas.konflik && (
                    <div className="md:col-span-2">
                      <Badge variant="destructive">Ada perbedaan nama Pokmas antar unit</Badge>
                    </div>
                  )}
                  {d.pokmas.kepala !== undefined ? (
                    <>
                      <Field label="Kepala" value={d.pokmas.kepala ?? null} />
                      <Field label="Bendahara" value={d.pokmas.bendahara ?? null} />
                      <Field label="Sekretaris" value={d.pokmas.sekretaris ?? null} />
                    </>
                  ) : (
                    <p className="text-xs text-muted-foreground md:col-span-2">
                      Nama pengurus hanya terlihat untuk editor dan admin.
                    </p>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border bg-background p-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-lg font-semibold tabular-nums">{value}</p>
    </div>
  )
}

function Field({ label, value }: { label: string; value: string | null }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-sm">{value ?? '–'}</p>
    </div>
  )
}
