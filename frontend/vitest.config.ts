import { fileURLToPath } from 'node:url'
import { mergeConfig, defineConfig, configDefaults } from 'vitest/config'
import viteConfig from './vite.config.ts'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      testTimeout: 15000,
      exclude: [...configDefaults.exclude, 'e2e/**', 'tests/**/*.spec.ts', '**/*.spec.ts'],
      root: fileURLToPath(new URL('./', import.meta.url)),
    },
  })
)
