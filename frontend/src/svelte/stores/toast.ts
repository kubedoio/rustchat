import { writable } from 'svelte/store'

export interface Toast {
  id: number
  type: 'success' | 'error' | 'info'
  title: string
  message?: string
  duration?: number
}

const internal = writable<Toast[]>([])
let nextId = 0

function add(toast: Omit<Toast, 'id'>): number {
  const id = nextId++
  const newToast = { ...toast, id }
  internal.update((toasts) => [...toasts, newToast])

  if (toast.duration !== 0) {
    setTimeout(() => remove(id), toast.duration || 5000)
  }
  return id
}

function remove(id: number) {
  internal.update((toasts) => toasts.filter((t) => t.id !== id))
}

function success(title: string, message?: string): number {
  return add({ type: 'success', title, message })
}

function error(title: string, message?: string): number {
  return add({ type: 'error', title, message })
}

function info(title: string, message?: string): number {
  return add({ type: 'info', title, message })
}

export const toastStore = {
  subscribe: internal.subscribe,
  add,
  remove,
  success,
  error,
  info,
}
