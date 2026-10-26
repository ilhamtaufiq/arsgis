import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

type Status = 'lengkap' | 'selisih_sr' | 'belum_dipetakan' | 'tanpa_batas'

type Row = {
  id: number
  nama: string
  kecamatan: string
  punya_batas: boolean
  punya_pokmas: boolean
  pokmas_konflik: boolean
  sr_tercatat: number
  sr_peta: number
  pipa_km: number
  unit_count: number
  status: Status
}

type Data = {
  ringkasan: {
    jumlah_desa: number
    per_status: Partial<Record<Status, number>>
    sr_tercatat: number
    sr_peta: number
    pipa_km: number
    tanpa_pokmas: number
    pokmas_konflik: number
  }
  data: Row[]
}

const STATUS: Record<Status, { label: string; variant: 'default' | 'secondary' | 'destructive' | 'outline'; hint: string }> = {
  lengkap: { label: 'Lengkap', variant: 'default', hint: 'SR di peta sama dengan SR tercatat' },
  selisih_sr: { label: 'Selisih SR', variant: 'destructive', hint: 'SR di peta berbeda dengan SR tercatat' },
  belum_dipetakan: { label: 'Belum dipetakan', variant: 'outline', hint: 'Belum ada fitur jaringan' },
  tanpa_batas: { label: 'Tanpa batas', variant: 'secondary', hint: 'Desa belum punya poligon batas' },
}

const nf = (n: number, d = 0) => n.toLocaleString('id-ID', { maximumFractionDigits: d })

/** Kelengkapan data per desa untuk editor dan admin: SR tercatat dibanding SR di peta. */
export function KelengkapanCard() {
  const [status, setStatus] = useState<'semua' | Status>('semua')
  const data = useQuery({
    queryKey: ['kelengkapan'],
    queryFn: async (): Promise<Data> => {
      const res = await fetch('/api/v1/dashboard/kelengkapan')
      if (!res.ok) throw new Error(`Kelengkapan gagal dimuat (${res.status})`)
      return (await res.json()) as Data
    },
    staleTime: 60_000,
  })

  const rows = useMemo(
    () => (data.data?.data ?? []).filter((r) => status === 'semua' || r.status === status),
    [data.data, status],
  )
  const ring = data.data?.ringkasan

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between gap-4">
        <div className="grid gap-1">
          <CardTitle className="text-base">Kelengkapan data per desa</CardTitle>
          <CardDescription>
            Membandingkan SR tercatat di Arumanis dengan SR yang sudah dipetakan di arsgis.
          </CardDescription>
        </div>
        <Select value={status} onValueChange={(v) => setStatus(v as 'semua' | Status)}>
          <SelectTrigger size="sm" className="w-44" aria-label="Filter status">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="semua">Semua status</SelectItem>
            {(Object.keys(STATUS) as Status[]).map((s) => (
              <SelectItem key={s} value={s}>
                {STATUS[s].label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {data.isError && <p className="text-sm text-red-700">{(data.error as Error).message}</p>}

        {ring && (
          <div className="grid gap-3 sm:grid-cols-4">
            {(Object.keys(STATUS) as Status[]).map((s) => (
              <div key={s} className="rounded-xl border p-3" title={STATUS[s].hint}>
                <p className="text-xs text-muted-foreground">{STATUS[s].label}</p>
                <p className="text-xl font-semibold tabular-nums">{nf(ring.per_status[s] ?? 0)} desa</p>
              </div>
            ))}
            <div className="rounded-xl border p-3 sm:col-span-4 text-sm text-muted-foreground">
              SR tercatat <strong className="text-foreground">{nf(ring.sr_tercatat)}</strong> · SR di peta{' '}
              <strong className="text-foreground">{nf(ring.sr_peta)}</strong> · pipa{' '}
              <strong className="text-foreground">{nf(ring.pipa_km, 2)} km</strong> · desa tanpa profil Pokmas{' '}
              <strong className="text-foreground">{nf(ring.tanpa_pokmas)}</strong>
              {ring.pokmas_konflik > 0 && (
                <>
                  {' '}· Pokmas berbeda antar unit <strong className="text-foreground">{nf(ring.pokmas_konflik)}</strong>
                </>
              )}
            </div>
          </div>
        )}

        <div className="max-h-[480px] overflow-auto rounded-xl border">
          <Table>
            <TableHeader className="sticky top-0 bg-card">
              <TableRow>
                <TableHead>Desa</TableHead>
                <TableHead>Kecamatan</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">SR tercatat</TableHead>
                <TableHead className="text-right">SR di peta</TableHead>
                <TableHead className="text-right">Pipa (km)</TableHead>
                <TableHead>Pokmas</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((r) => (
                <TableRow key={r.id}>
                  <TableCell className="font-medium">{r.nama}</TableCell>
                  <TableCell className="text-muted-foreground">{r.kecamatan}</TableCell>
                  <TableCell>
                    <Badge variant={STATUS[r.status].variant} title={STATUS[r.status].hint}>
                      {STATUS[r.status].label}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-right tabular-nums">{nf(r.sr_tercatat)}</TableCell>
                  <TableCell className="text-right tabular-nums">{nf(r.sr_peta)}</TableCell>
                  <TableCell className="text-right tabular-nums">{nf(r.pipa_km, 2)}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {!r.punya_pokmas ? 'Belum ada' : r.pokmas_konflik ? 'Berbeda antar unit' : 'Terisi'}
                  </TableCell>
                </TableRow>
              ))}
              {data.data && rows.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} className="py-6 text-center text-muted-foreground">
                    Tidak ada desa dengan status ini.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  )
}
