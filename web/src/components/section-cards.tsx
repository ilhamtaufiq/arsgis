import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const nf = (n: number, digits = 0) => n.toLocaleString('id-ID', { maximumFractionDigits: digits })

type Props = {
  totalFitur: number
  totalKm: number
  jenis: number
  bisaSr: boolean
  totalSr: number
}

/** Empat kartu ringkasan, data dari endpoint dashboard. */
export function SectionCards({ totalFitur, totalKm, jenis, bisaSr, totalSr }: Props) {
  return (
    <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <Card>
        <CardHeader>
          <CardDescription>Total fitur</CardDescription>
          <CardTitle className="text-3xl tabular-nums">{nf(totalFitur)}</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">Tersebar di {jenis} jenis</CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>Panjang pipa</CardDescription>
          <CardTitle className="text-3xl tabular-nums">{nf(totalKm, 2)} km</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">Transmisi dan distribusi</CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>Sambungan rumah</CardDescription>
          <CardTitle className="text-3xl tabular-nums">{bisaSr ? nf(totalSr) : '–'}</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">
          {bisaSr ? 'Data pelanggan' : 'Hanya untuk editor dan admin'}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>Jenis fitur</CardDescription>
          <CardTitle className="text-3xl tabular-nums">{jenis}</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">Pipa, valve, reservoir, dan lainnya</CardContent>
      </Card>
    </div>
  )
}
