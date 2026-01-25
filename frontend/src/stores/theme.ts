import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Theme = 'dark' | 'system'

export const useThemeStore = defineStore('theme', () => {
    const storedTheme = localStorage.getItem('theme') as Theme
    const theme = ref<Theme>((storedTheme === 'dark' || storedTheme === 'system') ? storedTheme : 'system')

    function setTheme(newTheme: Theme) {
        theme.value = newTheme
        localStorage.setItem('theme', newTheme)
        applyTheme()
    }

    function applyTheme() {
        const root = window.document.documentElement

        let effectiveTheme: 'dark' | 'light' = theme.value === 'system' 
            ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
            : 'dark'

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
