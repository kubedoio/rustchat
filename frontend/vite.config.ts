import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath, URL } from 'node:url'

// https://vite.dev/config/
export default defineConfig({
  test: {
    exclude: ['e2e/**', 'tests/**', 'node_modules/**']
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '$svelte': fileURLToPath(new URL('./src/svelte', import.meta.url)),
    }
  },
  plugins: [svelte()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id) {
            return
          }

          if (id.includes('node_modules')) {
            if (
              id.includes('/svelte/') ||
              id.includes('/@sveltejs/')
            ) {
              return 'vendor-framework'
            }

            if (
              id.includes('/highlight.js/') ||
              id.includes('/marked/') ||
              id.includes('/dompurify/')
            ) {
              return 'vendor-markdown'
            }

            return 'vendor'
          }

        },
      },
    },
  },
})
