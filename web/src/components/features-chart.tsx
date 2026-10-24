import { Bar, BarChart, CartesianGrid, Cell, XAxis, YAxis } from 'recharts'
import { ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig } from '@/components/ui/chart'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const config = {
  jumlah: { label: 'Jumlah', color: 'var(--primary)' },
} satisfies ChartConfig

type Item = { label: string; jumlah: number; color: string }

/** Grafik batang jumlah fitur per jenis. */
export function FeaturesChart({ data }: { data: Item[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Fitur per jenis</CardTitle>
        <CardDescription>Jumlah fitur aktif di jaringan yang dipilih</CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={config} className="h-64 w-full">
          <BarChart data={data} margin={{ left: 0, right: 8, top: 8 }}>
            <CartesianGrid vertical={false} />
            <XAxis dataKey="label" tickLine={false} axisLine={false} tickMargin={8} />
            <YAxis tickLine={false} axisLine={false} width={40} />
            <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
            <Bar dataKey="jumlah" fill="var(--color-jumlah)" radius={6}>
              {data.map((d) => (
                <Cell key={d.label} fill={d.color} />
              ))}
            </Bar>
          </BarChart>
        </ChartContainer>
      </CardContent>
    </Card>
  )
}
