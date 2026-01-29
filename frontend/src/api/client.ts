import axios from 'axios'
import { useAuthStore } from '../stores/auth'

const client = axios.create({
    baseURL: import.meta.env.VITE_API_URL || '/api/v1',
})

client.interceptors.request.use(config => {
    const authStore = useAuthStore()
    if (authStore.token) {
        config.headers.Authorization = `Bearer ${authStore.token}`
    }
    return config
})

client.interceptors.response.use(
    response => response,
    error => {
        if (error.response?.status === 401) {
            const authStore = useAuthStore()
            authStore.logout()
        }
        return Promise.reject(error)
    }
)

export default client
