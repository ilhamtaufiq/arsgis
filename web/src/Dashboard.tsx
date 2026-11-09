import './preflight.css'
import { useQueries, useQuery } from '@tanstack/react-query'
import { fetchMe, fetchNetworks } from './api'
import { AuthPanel } from './Auth'
import { fetchDashboard, type DashboardData } from './dashboardApi'
import { AppSidebar } from '@/components/app-sidebar'
import { FeaturesChart } from '@/components/features-chart'
import { ImportsTable } from '@/components/imports-table'
import { KelengkapanCard } from './KelengkapanCard'
import { JatuhTempoCard } from './om/JatuhTempoPanel'
import { bolehCatat } from './om/omLogic'
import { SectionCards } from '@/components/section-cards'
import { SiteHeader } from '@/components/site-header'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { ASET_COLOR, ASET_LABEL } from '@/asetStyle'

// Label dan warna jenis fitur, sama dengan warna di peta.
const JENIS: Record<string, { label: string; color: string }> = {
  pipa: { label: 'Pipa', color: '#1d4ed8' },
  valve: { label: 'Valve', color: '#111111' },
  reservoir: { label: 'Reservoir', color: '#0f766e' },
  sr: { label: 'Sambungan rumah', color: '#a16207' },
  bronkaptering: { label: 'Bronkaptering', color: '#0891b2' },
  ...Object.fromEntries(
    Object.entries(ASET_LABEL).map(([k, label]) => [k, { label, color: ASET_COLOR[k as keyof typeof ASET_COLOR] }]),
  ),
}

// Unit dibaca dari query string, misalnya /dashboard?unit=3. Tanpa parameter, statistik semua unit.
function readUnitId(): number | null {
  const id = Number(new URLSearchParams(window.location.search).get('unit'))
  return Number.isInteger(id) && id > 0 ? id : null
}

// Menjumlahkan statistik beberapa unit menjadi satu, bentuknya sama dengan satu unit.
function gabung(list: DashboardData[]) {
  const fitur = new Map<string, number>()
  const pipa = new Map<string, { jumlah: number; km: number }>()
  for (const d of list) {
    for (const f of d.fitur) fitur.set(f.feature_type, (fitur.get(f.feature_type) ?? 0) + f.jumlah)
    for (const p of d.pipa) {
      const c = pipa.get(p.fungsi) ?? { jumlah: 0, km: 0 }
      pipa.set(p.fungsi, { jumlah: c.jumlah + p.jumlah, km: c.km + p.km })
    }
  }
  return {
    fitur: [...fitur].map(([feature_type, jumlah]) => ({ feature_type, jumlah })),
    pipa: [...pipa].map(([fungsi, v]) => ({ fungsi, ...v })),
  }
}

/**
 * Halaman dashboard: statistik semua unit SPAM, atau satu unit bila ada ?unit=.
 * Kelengkapan per desa tidak bergantung pada unit dan tampil untuk editor dan admin.
 */
export function DashboardPage() {
  const me = useQuery({ queryKey: ['me'], queryFn: fetchMe })
  const networks = useQuery({ queryKey: ['networks'], queryFn: fetchNetworks })
  // Hanya baris jaringan yang merupakan unit SPAM (punya desa dan unit).
  const units = (networks.data ?? []).filter((n) => n.unit_spam_id !== null)
  const unitId = readUnitId()
  // Hasilnya bergantung pada peran (sambungan rumah dan impor), jadi peran ikut kunci cache.
  const role = me.data?.role ?? 'anon'

  const satuUnit = useQuery({
    queryKey: ['dashboard', unitId, role],
    queryFn: () => fetchDashboard(unitId!),
    enabled: unitId !== null && !me.isPending,
  })

  // Tanpa ?unit, ambil statistik tiap unit.
  const semuaUnit = useQueries({
    queries: units.map((u) => ({
      queryKey: ['dashboard', u.id, role],
      queryFn: () => fetchDashboard(u.id),
      enabled: unitId === null && !me.isPending,
    })),
  })

  const sumber: DashboardData[] =
    unitId === null
      ? semuaUnit.flatMap((q) => (q.data ? [q.data] : []))
      : satuUnit.data
        ? [satuUnit.data]
        : []
  const agregat = gabung(sumber)
  const fitur = agregat.fitur
  const totalFitur = fitur.reduce((sum, f) => sum + f.jumlah, 0)
  const totalSr = fitur.find((f) => f.feature_type === 'sr')?.jumlah ?? 0
  const totalKm = agregat.pipa.reduce((sum, p) => sum + p.km, 0)
  const bisaSr = sumber.some((d) => d.bisa_melihat_sr)
  const bisaKelengkapan = me.data?.role === 'editor' || me.data?.role === 'admin'
  const judul = unitId === null ? 'Semua unit SPAM' : (satuUnit.data?.jaringan ?? '')

  const chartData = fitur.map((f) => ({
    label: JENIS[f.feature_type]?.label ?? f.feature_type,
    jumlah: f.jumlah,
    color: JENIS[f.feature_type]?.color ?? '#6b7280',
  }))

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <SiteHeader
          title={sumber.length > 0 ? `Dashboard · ${judul}` : 'Dashboard'}
          actions={<AuthPanel user={me.data ?? null} inline />}
        />
        <main className="flex flex-1 flex-col gap-4 p-4 md:p-6">
          {satuUnit.isError && (
            <p className="rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700">{(satuUnit.error as Error).message}</p>
          )}

          <SectionCards totalFitur={totalFitur} totalKm={totalKm} jenis={fitur.length} bisaSr={bisaSr} totalSr={totalSr} />

          <div className="grid gap-4 lg:grid-cols-3">
            <div className="min-w-0 lg:col-span-2">
              <FeaturesChart data={chartData} />
            </div>
            <Card className="min-w-0">
              <CardHeader>
                <CardTitle>Pipa per fungsi</CardTitle>
                <CardDescription>Panjang dan jumlah ruas</CardDescription>
              </CardHeader>
              <CardContent>
                <ul className="flex flex-col gap-4 text-sm">
                  {agregat.pipa.map((p) => (
                    <li key={p.fungsi} className="flex items-center justify-between">
                      <span className="font-medium capitalize">{p.fungsi}</span>
                      <span className="tabular-nums text-muted-foreground">
                        {p.km.toLocaleString('id-ID', { maximumFractionDigits: 2 })} km · {p.jumlah} ruas
                      </span>
                    </li>
                  ))}
                  {sumber.length > 0 && agregat.pipa.length === 0 && <li className="text-muted-foreground">Belum ada pipa.</li>}
                </ul>
              </CardContent>
            </Card>
          </div>

          {unitId !== null && bisaSr && <ImportsTable rows={satuUnit.data?.impor_terakhir ?? []} />}

          {bisaKelengkapan && <KelengkapanCard />}
          {unitId !== null && me.data && <JatuhTempoCard networkId={unitId} bisaCatat={bolehCatat(me.data.role)} />}
        </main>
      </SidebarInset>
    </SidebarProvider>
  )
}
