import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    svelte(),
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      // Icons live in public/ (Task 1); include them in the precache too.
      includeAssets: ['favicon.svg', 'apple-touch-icon.png'],
      manifest: {
        name: 'Octopus',
        short_name: 'Octopus',
        description: 'Self-hosted hub for your freelance business — projects, tasks, contacts, calendar, notes.',
        display: 'standalone',
        start_url: '/',
        scope: '/',
        theme_color: '#06070a',
        background_color: '#06070a',
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
          { src: '/icon-512-maskable.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,svg,png,woff2}'],
        navigateFallback: '/index.html',
        navigateFallbackDenylist: [/^\/api\//],  // never serve the shell for API requests
        // no runtimeCaching: API is never cached
      },
    }),
  ],
  // Build straight into the dir Axum serves (../static at repo root).
  build: { outDir: '../static', emptyOutDir: true },
  server: {
    port: 5173,
    proxy: { '/api': 'http://localhost:8090' }, // Rust API in local dev
  },
  // vitest runs components in jsdom, not SSR; force the browser build of .svelte packages.
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  test: { environment: 'jsdom' },
})
