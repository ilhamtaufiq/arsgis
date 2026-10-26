import './preflight.css'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchMe, fetchNetworks } from './api'
import { AuthPanel } from './Auth'
import { fetchDashboard } from './dashboardApi'
import { AppSidebar } from '@/components/app-sidebar'
import { FeaturesChart } from '@/components/features-chart'
import { ImportsTable } from '@/components/imports-table'
import { KelengkapanCard } from './KelengkapanCard'
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

/**
 * Halaman dashboard. Statistik unit baru tampil setelah pengguna memilih unit SPAM.
 * Kelengkapan per desa tidak bergantung pada unit dan tampil untuk editor dan admin.
 */
export function DashboardPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  // Hanya baris jaringan yang merupakan unit SPAM (punya desa dan unit).
  const units = (networks.data ?? []).filter((n) => n.unit_spam_id !== null)
  const [unitId, setUnitId] = useState<number | null>(null)

  const data = useQuery({
    // Hasilnya bergantung pada peran (sambungan rumah dan impor), jadi peran ikut kunci cache.
    queryKey: ['dashboard', unitId, me.data?.role ?? 'anon'],
    queryFn: () => fetchDashboard(unitId!),
    enabled: unitId !== null && !me.isPending,
  })

  const fitur = data.data?.fitur ?? []
  const totalFitur = fitur.reduce((sum, f) => sum + f.jumlah, 0)
  const totalSr = fitur.find((f) => f.feature_type === 'sr')?.jumlah ?? 0
  const totalKm = (data.data?.pipa ?? []).reduce((sum, p) => sum + p.km, 0)
  const bisaSr = data.data?.bisa_melihat_sr ?? false
  const adaUnit = unitId !== null
  const bisaKelengkapan = me.data?.role === 'editor' || me.data?.role === 'admin'

  const chartData = fitur.map((f) => ({
    label: JENIS[f.feature_type]?.label ?? f.feature_type,
    jumlah: f.jumlah,
    color: JENIS[f.feature_type]?.color ?? '#6b7280',
  }))

  const unitPicker = (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Pilih unit SPAM</CardTitle>
        <CardDescription>
          {units.length} unit SPAM. Pilih satu untuk melihat jaringan, capaian, dan impornya.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Select value={unitId === null ? '' : String(unitId)} onValueChange={(v) => setUnitId(Number(v))}>
          <SelectTrigger className="w-full max-w-xl" aria-label="Pilih unit SPAM">
            <SelectValue placeholder="Pilih desa · unit SPAM…" />
          </SelectTrigger>
          <SelectContent className="max-h-80">
            {units.map((u) => (
              <SelectItem key={u.id} value={String(u.id)}>
                {u.nama}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </CardContent>
    </Card>
  )

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <SiteHeader
          title={adaUnit && data.data ? `Dashboard · ${data.data.jaringan}` : 'Dashboard'}
          actions={<AuthPanel user={me.data ?? null} />}
        />
        <main className="flex flex-1 flex-col gap-4 p-4 md:p-6">
          {unitPicker}

          {data.isError && (
            <p className="rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700">{(data.error as Error).message}</p>
          )}

          {adaUnit && (
            <>
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
            </>
          )}

          {bisaKelengkapan && <KelengkapanCard />}
        </main>
      </SidebarInset>
    </SidebarProvider>
  )
}
