import { lazy, StrictMode, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { App } from './App'
import './index.css'
import './styles.css'

// Halaman dashboard dan admin dimuat lazy: reset Tailwind (preflight) hanya berlaku di sana.
const DashboardPage = lazy(() => import('./Dashboard').then((m) => ({ default: m.DashboardPage })))
const AdminPage = lazy(() => import('./Admin').then((m) => ({ default: m.AdminPage })))

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, retry: 1 } },
})

// Routing sederhana: /dashboard dan /admin adalah halaman sendiri, path lain adalah peta.
const path = window.location.pathname.replace(/\/+$/, '')

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={null}>
        {path === '/dashboard' ? <DashboardPage /> : path === '/admin' ? <AdminPage /> : <App />}
      </Suspense>
    </QueryClientProvider>
  </StrictMode>,
)
