import { ref } from 'vue'

const toastRef = ref<any>(null)

export function useToast() {
    function success(title: string, message?: string) {
        toastRef.value?.add({ type: 'success', title, message })
    }
    function error(title: string, message?: string) {
        toastRef.value?.add({ type: 'error', title, message })
    }
    function info(title: string, message?: string) {
        toastRef.value?.add({ type: 'info', title, message })
    }

    // Function to register the real toast instance
    function register(instance: any) {
        toastRef.value = instance
    }

    return { success, error, info, register }
}
