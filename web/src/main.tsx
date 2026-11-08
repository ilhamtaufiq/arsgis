import { lazy, StrictMode, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { LoginGate } from './LoginGate'
import './index.css'
import './styles.css'

// Halaman dimuat lazy: reset Tailwind (preflight) hanya berlaku di halaman yang memakainya.
const LandingPage = lazy(() => import('./LandingPage').then((m) => ({ default: m.LandingPage })))
const LoginPage = lazy(() => import('./LoginPage').then((m) => ({ default: m.LoginPage })))
const MapPage = lazy(() => import('./App').then((m) => ({ default: m.App })))
const DashboardPage = lazy(() => import('./Dashboard').then((m) => ({ default: m.DashboardPage })))
const AdminPage = lazy(() => import('./Admin').then((m) => ({ default: m.AdminPage })))

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, retry: 1 } },
})

// Routing sederhana: / landing (publik), /masuk halaman login, dan halaman lain hanya setelah masuk.
const path = window.location.pathname.replace(/\/+$/, '')

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={null}>
        {path === '/masuk' ? (
          <LoginPage />
        ) : path === '/peta' ? (
          <LoginGate judul="Peta jaringan">
            <MapPage />
          </LoginGate>
        ) : path === '/dashboard' ? (
          <LoginGate judul="Dashboard">
            <DashboardPage />
          </LoginGate>
        ) : path === '/admin' ? (
          <LoginGate judul="Admin">
            <AdminPage />
          </LoginGate>
        ) : (
          <LandingPage />
        )}
      </Suspense>
    </QueryClientProvider>
  </StrictMode>,
)
