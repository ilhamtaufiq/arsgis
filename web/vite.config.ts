import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Pengembangan: API Rust berjalan di 8080, dan Vite meneruskan /api dan /healthz ke sana.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/healthz': 'http://127.0.0.1:8080',
    },
  },
})
