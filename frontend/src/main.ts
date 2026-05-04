import './style.css'
import 'highlight.js/styles/github-dark.css'
import App from './App.svelte'
import { mount } from 'svelte'
import { themeStore } from './svelte/stores/theme'

themeStore.applyAppearance()

mount(App, {
  target: document.getElementById('app')!,
})
