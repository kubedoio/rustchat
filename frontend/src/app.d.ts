declare module '*.svelte' {
  import type { Component } from 'svelte'

  const component: Component
  export default component
}

declare module '*.vue' {
  import type { DefineComponent } from 'vue'

  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>
  export default component
}
