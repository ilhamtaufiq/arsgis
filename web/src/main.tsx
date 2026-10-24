import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { App } from './App'
import { AdminPage } from './Admin'
import { DashboardPage } from './Dashboard'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, retry: 1 } },
})

// Routing sederhana: /dashboard dan /admin adalah halaman sendiri, path lain adalah peta.
const path = window.location.pathname.replace(/\/+$/, '')

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      {path === '/dashboard' ? <DashboardPage /> : path === '/admin' ? <AdminPage /> : <App />}
    </QueryClientProvider>
  </StrictMode>,
)
