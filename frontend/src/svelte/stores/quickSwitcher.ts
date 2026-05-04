import { writable } from 'svelte/store'

export interface QuickSwitcherState {
    open: boolean
}

const initialState: QuickSwitcherState = {
    open: false,
}

function createQuickSwitcherStore() {
    const { subscribe, update } = writable<QuickSwitcherState>(initialState)

    function toggle() {
        update((state) => ({ ...state, open: !state.open }))
    }

    function open() {
        update((state) => ({ ...state, open: true }))
    }

    function close() {
        update((state) => ({ ...state, open: false }))
    }

    if (typeof window !== 'undefined') {
        window.addEventListener('keydown', (e: KeyboardEvent) => {
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault()
                toggle()
            }
        })
    }

    return {
        subscribe,
        toggle,
        open,
        close,
    }
}

export const quickSwitcherStore = createQuickSwitcherStore()
