import { ref, onUnmounted } from 'vue'
import { useAuthStore } from '../stores/auth'

export function useSocket() {
    const auth = useAuthStore()
    // const messageStore = useMessageStore()
    const socket = ref<WebSocket | null>(null)
    const isConnected = ref(false)

    function connect() {
        if (!auth.token || socket.value) return

        const wsUrl = import.meta.env.VITE_WS_URL || 'ws://localhost:3000/api/v1/ws'
        socket.value = new WebSocket(`${wsUrl}?token=${auth.token}`)

        socket.value.onopen = () => {
            isConnected.value = true
            console.log('WS Connected')
        }

        socket.value.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data)
                handleMessage(data)
            } catch (e) {
                console.error('WS Parse Error', e)
            }
        }

        socket.value.onclose = () => {
            isConnected.value = false
            socket.value = null
            // Reconnect logic would go here
        }
    }

    function handleMessage(data: any) {
        // Dispatch to stores based on event type
        if (data.type === 'new_post') {
            // Ideally we would parse this into the Message interface
            // messageStore.handleNewMessage(data.payload)
        }
    }

    function disconnect() {
        socket.value?.close()
        socket.value = null
    }

    onUnmounted(() => {
        disconnect()
    })

    return { connect, disconnect, isConnected }
}
