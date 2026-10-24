import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { App } from './App'
import { DashboardPage } from './Dashboard'
import './styles.css'

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, retry: 1 } },
})

// Routing sederhana: /dashboard adalah halaman sendiri, path lain adalah peta.
const onDashboard = window.location.pathname.replace(/\/+$/, '') === '/dashboard'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      {onDashboard ? <DashboardPage /> : <App />}
    </QueryClientProvider>
  </StrictMode>,
)
