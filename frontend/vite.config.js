import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  // Build straight into the dir Axum serves (../static at repo root).
  build: { outDir: '../static', emptyOutDir: true },
  server: {
    port: 5173,
    proxy: { '/api': 'http://localhost:8090' }, // Rust API in local dev
  },
  test: { environment: 'jsdom' },
})
