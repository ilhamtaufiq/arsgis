import './preflight.css'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchMe, fetchNetworks } from './api'
import { AuthPanel } from './Auth'
import { fetchDashboard } from './dashboardApi'
import { AppSidebar } from '@/components/app-sidebar'
import { FeaturesChart } from '@/components/features-chart'
import { ImportsTable } from '@/components/imports-table'
import { SectionCards } from '@/components/section-cards'
import { SiteHeader } from '@/components/site-header'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'

// Label dan warna jenis fitur, sama dengan warna di peta.
const JENIS: Record<string, { label: string; color: string }> = {
  pipa: { label: 'Pipa', color: '#1d4ed8' },
  valve: { label: 'Valve', color: '#111111' },
  reservoir: { label: 'Reservoir', color: '#0f766e' },
  sr: { label: 'Sambungan rumah', color: '#a16207' },
  bronkaptering: { label: 'Bronkaptering', color: '#0891b2' },
}

/** Halaman dashboard jaringan, dibuka di /dashboard. */
export function DashboardPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  const [chosen, setChosen] = useState<number | null>(null)
  const networkId = chosen ?? networks.data?.[0]?.id ?? null

  const data = useQuery({
    // Hasilnya bergantung pada peran (sambungan rumah dan impor), jadi peran ikut kunci cache.
    queryKey: ['dashboard', networkId, me.data?.role ?? 'anon'],
    queryFn: () => fetchDashboard(networkId!),
    enabled: networkId !== null && !me.isPending,
  })

  const fitur = data.data?.fitur ?? []
  const totalFitur = fitur.reduce((sum, f) => sum + f.jumlah, 0)
  const totalSr = fitur.find((f) => f.feature_type === 'sr')?.jumlah ?? 0
  const totalKm = (data.data?.pipa ?? []).reduce((sum, p) => sum + p.km, 0)
  const bisaSr = data.data?.bisa_melihat_sr ?? false

  const chartData = fitur.map((f) => ({
    label: JENIS[f.feature_type]?.label ?? f.feature_type,
    jumlah: f.jumlah,
    color: JENIS[f.feature_type]?.color ?? '#6b7280',
  }))

  const networkSelect =
    networks.data && networks.data.length > 1 ? (
      <Select value={String(networkId ?? '')} onValueChange={(v) => setChosen(Number(v))}>
        <SelectTrigger size="sm" className="w-56" aria-label="Pilih jaringan">
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
    ) : null

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <SiteHeader
          title={`Dashboard · ${data.data?.jaringan ?? 'memuat…'}`}
          actions={
            <>
              {networkSelect}
              <AuthPanel user={me.data ?? null} />
            </>
          }
        />
        <main className="flex flex-1 flex-col gap-4 p-4 md:p-6">
          {data.isError && (
            <p className="rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700">{(data.error as Error).message}</p>
          )}

          <SectionCards totalFitur={totalFitur} totalKm={totalKm} jenis={fitur.length} bisaSr={bisaSr} totalSr={totalSr} />

          <div className="grid gap-4 lg:grid-cols-3">
            <div className="lg:col-span-2">
              <FeaturesChart data={chartData} />
            </div>
            <Card>
              <CardHeader>
                <CardTitle>Pipa per fungsi</CardTitle>
                <CardDescription>Panjang dan jumlah ruas</CardDescription>
              </CardHeader>
              <CardContent>
                <ul className="flex flex-col gap-4 text-sm">
                  {(data.data?.pipa ?? []).map((p) => (
                    <li key={p.fungsi} className="flex items-center justify-between">
                      <span className="font-medium capitalize">{p.fungsi}</span>
                      <span className="tabular-nums text-muted-foreground">
                        {p.km.toLocaleString('id-ID', { maximumFractionDigits: 2 })} km · {p.jumlah} ruas
                      </span>
                    </li>
                  ))}
                  {data.data && data.data.pipa.length === 0 && <li className="text-muted-foreground">Belum ada pipa.</li>}
                </ul>
              </CardContent>
            </Card>
          </div>

          {bisaSr && <ImportsTable rows={data.data?.impor_terakhir ?? []} />}
        </main>
      </SidebarInset>
    </SidebarProvider>
  )
}
