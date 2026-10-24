import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { TrendingUp } from 'lucide-react'

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
    <div className="grid gap-4 *:data-[slot=card]:shadow-xs sm:grid-cols-2 xl:grid-cols-4">
      <Card data-slot="card">
        <CardHeader>
          <CardDescription>Total fitur</CardDescription>
          <CardTitle className="text-2xl tabular-nums">{nf(totalFitur)}</CardTitle>
          <CardFooter className="text-sm text-muted-foreground">
            <Badge variant="secondary">
              <TrendingUp /> {jenis} jenis
            </Badge>
          </CardFooter>
        </CardHeader>
      </Card>
      <Card data-slot="card">
        <CardHeader>
          <CardDescription>Panjang pipa</CardDescription>
          <CardTitle className="text-2xl tabular-nums">{nf(totalKm, 2)} km</CardTitle>
        </CardHeader>
      </Card>
      <Card data-slot="card">
        <CardHeader>
          <CardDescription>Sambungan rumah</CardDescription>
          <CardTitle className="text-2xl tabular-nums">{bisaSr ? nf(totalSr) : '–'}</CardTitle>
          {!bisaSr && <CardFooter className="text-sm text-muted-foreground">Hanya editor dan admin</CardFooter>}
        </CardHeader>
      </Card>
      <Card data-slot="card">
        <CardHeader>
          <CardDescription>Jenis fitur</CardDescription>
          <CardTitle className="text-2xl tabular-nums">{jenis}</CardTitle>
        </CardHeader>
      </Card>
    </div>
  )
}
