import js from '@eslint/js'
import pluginVue from 'eslint-plugin-vue'
import ts from 'typescript-eslint'
import prettierConfig from 'eslint-config-prettier'
import prettierPlugin from 'eslint-plugin-prettier'
import vueParser from 'vue-eslint-parser'
import globals from 'globals'

export default [
  js.configs.recommended,
  ...ts.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  prettierConfig,
  {
    files: ['**/*.{js,mjs,cjs,ts}'],
    languageOptions: {
      parser: ts.parser,
      globals: {
        ...globals.browser,
        ...globals.node,
        ScrollBehavior: 'readonly',
        NotificationPermission: 'readonly',
      },
    },
    plugins: {
      prettier: prettierPlugin,
    },
    rules: {
      'prettier/prettier': [
        'error',
        {
          semi: false,
          singleQuote: true,
          tabWidth: 2,
          trailingComma: 'es5',
          printWidth: 100,
          arrowParens: 'avoid',
        },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-empty-object-type': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^(props|emit|_)',
          caughtErrors: 'none',
        },
      ],
    },
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: ts.parser,
        extraFileExtensions: ['.vue'],
        sourceType: 'module',
      },
      globals: {
        ...globals.browser,
        ...globals.node,
        ScrollBehavior: 'readonly',
        NotificationPermission: 'readonly',
      },
    },
    plugins: {
      prettier: prettierPlugin,
    },
    rules: {
      'prettier/prettier': [
        'error',
        {
          semi: false,
          singleQuote: true,
          tabWidth: 2,
          trailingComma: 'es5',
          printWidth: 100,
          arrowParens: 'avoid',
        },
      ],
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-empty-object-type': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^(props|emit|_)',
          caughtErrors: 'none',
        },
      ],
    },
  },
  {
    files: ['**/*.{test,spec}.{js,ts}', 'e2e/**/*.ts'],
    rules: {
      '@typescript-eslint/no-unused-vars': 'off',
    },
  },
  {
    ignores: ['dist/', 'node_modules/', '*.d.ts'],
  },
]
