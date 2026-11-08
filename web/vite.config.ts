import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Pengembangan: API Rust berjalan di 8080 (atau port dari API_PORT, diisi dev.bat), dan Vite meneruskan
// /api, /healthz, dan /basemap ke sana.
const apiTarget = `http://127.0.0.1:${process.env.API_PORT ?? 8080}`

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(__dirname, './src') },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': apiTarget,
      '/healthz': apiTarget,
      '/basemap': apiTarget,
    },
  },
})
