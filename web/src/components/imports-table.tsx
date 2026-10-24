import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import type { DashboardData } from '@/dashboardApi'

type Item = DashboardData['impor_terakhir'][number]

/** Tabel 10 impor terakhir. Hanya tampil untuk editor dan admin. */
export function ImportsTable({ rows }: { rows: Item[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Impor terakhir</CardTitle>
        <CardDescription>Batch KML yang diunggah ke jaringan ini</CardDescription>
      </CardHeader>
      <CardContent>
        {rows.length === 0 ? (
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
              {rows.map((i) => (
                <TableRow key={i.id}>
                  <TableCell className="max-w-56 truncate font-medium">{i.nama_file}</TableCell>
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
