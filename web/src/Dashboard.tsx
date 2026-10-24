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
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'

const JENIS_LABEL: Record<string, string> = {
  pipa: 'Pipa',
  valve: 'Valve',
  reservoir: 'Reservoir',
  sr: 'Sambungan rumah',
  bronkaptering: 'Bronkaptering',
}

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
  const totalFitur = fitur.reduce((sum, f) => sum + f.jumlah, 0)
  const totalSr = fitur.find((f) => f.feature_type === 'sr')?.jumlah ?? 0
  const totalKm = (data.data?.pipa ?? []).reduce((sum, p) => sum + p.km, 0)
  const bisaSr = data.data?.bisa_melihat_sr ?? false

  const chartData = fitur.map((f) => ({ label: JENIS_LABEL[f.feature_type] ?? f.feature_type, jumlah: f.jumlah }))

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
          network={data.data?.jaringan ?? 'memuat…'}
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
            <div className="rounded-xl border bg-card p-4 text-sm">
              <p className="mb-3 font-medium">Pipa per fungsi</p>
              <ul className="flex flex-col gap-3">
                {(data.data?.pipa ?? []).map((p) => (
                  <li key={p.fungsi} className="flex items-center justify-between">
                    <span className="capitalize">{p.fungsi}</span>
                    <span className="tabular-nums text-muted-foreground">
                      {p.km.toLocaleString('id-ID', { maximumFractionDigits: 2 })} km · {p.jumlah} ruas
                    </span>
                  </li>
                ))}
                {data.data && data.data.pipa.length === 0 && <li className="text-muted-foreground">Belum ada pipa.</li>}
              </ul>
            </div>
          </div>

          {bisaSr && <ImportsTable rows={data.data?.impor_terakhir ?? []} />}
        </main>
      </SidebarInset>
    </SidebarProvider>
  )
}
