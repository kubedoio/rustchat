import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Theme = 'light' | 'dark' | 'system'

export const useThemeStore = defineStore('theme', () => {
    const theme = ref<Theme>((localStorage.getItem('theme') as Theme) || 'system')

    function setTheme(newTheme: Theme) {
        theme.value = newTheme
        localStorage.setItem('theme', newTheme)
        applyTheme()
    }

    function applyTheme() {
        const root = window.document.documentElement

        let effectiveTheme = theme.value
        if (effectiveTheme === 'system') {
            effectiveTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
        }

        if (effectiveTheme === 'dark') {
            root.classList.add('dark')
        } else {
            root.classList.remove('dark')
        }
    }

    // Initialize
    applyTheme()

    // Listen for system theme changes
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (theme.value === 'system') {
            applyTheme()
        }
    })

    return { theme, setTheme, applyTheme }
})
