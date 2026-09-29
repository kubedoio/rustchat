import { fileURLToPath } from 'node:url'
import { mergeConfig, defineConfig, configDefaults } from 'vitest/config'
import viteConfig from './vite.config.ts'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      exclude: [...configDefaults.exclude, 'e2e/**', 'tests/**/*.spec.ts', '**/*.spec.ts'],
      root: fileURLToPath(new URL('./', import.meta.url)),
      coverage: {
        provider: 'v8',
        reporter: ['text', 'text-summary', 'html'],
        reportsDirectory: './coverage',
        exclude: [
          // App bootstrap: mounts the app and applies side effects; not unit-testable.
          // Currently never loaded by tests, so this changes no numbers.
          'src/main.ts',
          // Playwright e2e suite: already outside the vitest test run (test.exclude).
          'e2e/**',
          // Type declarations carry no executable code.
          '**/*.d.ts',
        ],
        thresholds: {
          // Honest baseline measured on main + #324 + #325 (2026-09-29):
          // statements 39.07%, branches 29.69%, functions 29.12%, lines 40.02%.
          // Rounded down 1-2pp to catch regressions without failing on noise.
          statements: 38,
          branches: 28,
          functions: 28,
          lines: 39,
        },
      },
    },
  })
)
