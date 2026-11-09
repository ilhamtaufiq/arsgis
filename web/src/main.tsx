import { lazy, StrictMode, Suspense, type ReactNode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Toaster } from 'sonner'
import { LoginGate } from './LoginGate'
import { AppShell } from './shell/AppShell'
import { HalamanTidakDitemukan } from './shell/NotFound'
import { normalisasiPath } from './shell/menu'
import { usePathname } from './shell/navigasi'
import './index.css'
import './styles.css'

// Halaman dimuat lazy: reset Tailwind (preflight) hanya berlaku di halaman yang memakainya.
const LandingPage = lazy(() => import('./LandingPage').then((m) => ({ default: m.LandingPage })))
const LoginPage = lazy(() => import('./LoginPage').then((m) => ({ default: m.LoginPage })))
const MapPage = lazy(() => import('./App').then((m) => ({ default: m.App })))
const DashboardPage = lazy(() => import('./Dashboard').then((m) => ({ default: m.DashboardPage })))
const AdminPage = lazy(() => import('./Admin').then((m) => ({ default: m.AdminPage })))
const ImporPage = lazy(() => import('./ImporPage').then((m) => ({ default: m.ImporPage })))
const DesaDetailPage = lazy(() => import('./DesaDetailPage').then((m) => ({ default: m.DesaDetailPage })))
const PelangganPage = lazy(() => import('./pelanggan/PelangganPage').then((m) => ({ default: m.PelangganPage })))
const OmPage = lazy(() => import('./om/OmPage').then((m) => ({ default: m.OmPage })))

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, retry: 1 } },
})

/** Halaman yang memakai shell bersama. Dashboard dan Admin sudah memasang sidebar dan header sendiri. */
function berGerbang(judul: string, halaman: ReactNode, shell?: { title: string; penuhLayar?: boolean; aksi?: ReactNode }) {
  return (
    <LoginGate judul={judul}>
      {shell ? <AppShell title={shell.title} penuhLayar={shell.penuhLayar} aksi={shell.aksi}>{halaman}</AppShell> : halaman}
    </LoginGate>
  )
}

/** Routing sederhana berbasis path. Pindah halaman lewat `navigate` tanpa muat ulang. */
function Rute() {
  const path = normalisasiPath(usePathname())

  if (path === '/') return <LandingPage />
  if (path === '/masuk') return <LoginPage />
  // Peta berlayar penuh tanpa sidebar. Navigasi ke halaman lain lewat tombol Dashboard di peta.
  if (path === '/peta') return berGerbang('Peta jaringan', <MapPage />)
  if (path === '/dashboard') return berGerbang('Dashboard', <DashboardPage />)
  if (path === '/pelanggan') return berGerbang('Pelanggan dan cakupan', <PelangganPage />, { title: 'Pelanggan' })
  if (path === '/impor') return berGerbang('Impor data', <ImporPage />, { title: 'Impor data' })
  if (path === '/om') return berGerbang('Operasi dan pemeliharaan', <OmPage />, { title: 'O&M' })
  if (path === '/admin') return berGerbang('Admin', <AdminPage />)
  if (/^\/desa\/\d+$/.test(path)) {
    // Halaman detail punya header dan tombol masuk sendiri, jadi shell tidak menambah tombol masuk.
    return (
      <AppShell title="Detail desa" aksi={<></>}>
        <DesaDetailPage id={Number(path.split('/')[2])} />
      </AppShell>
    )
  }
  return <HalamanTidakDitemukan />
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={null}>
        <Rute />
      </Suspense>
      <Toaster position="bottom-right" closeButton duration={4000} />
    </QueryClientProvider>
  </StrictMode>,
)
