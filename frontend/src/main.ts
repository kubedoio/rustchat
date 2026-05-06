import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import 'highlight.js/styles/github-dark.css'
import App from './App.vue'
import router from './router'
import { useThemeStore } from './stores/theme'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(router)

useThemeStore(pinia).applyAppearance()

app.mount('#app')
