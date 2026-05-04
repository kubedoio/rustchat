export function getBrowserStorage(): Storage | null {
    if (typeof window === 'undefined') {
        return null
    }

    return window.localStorage
}

export function readLocalStorage(key: string, fallback = ''): string {
    return getBrowserStorage()?.getItem(key) ?? fallback
}

export function writeLocalStorage(key: string, value: string): void {
    getBrowserStorage()?.setItem(key, value)
}

export function removeLocalStorage(key: string): void {
    getBrowserStorage()?.removeItem(key)
}

export function createJsonStorage<T>(key: string, fallback: T) {
    return {
        read(): T {
            const raw = readLocalStorage(key, '')
            if (!raw) {
                return fallback
            }

            try {
                return JSON.parse(raw) as T
            } catch {
                return fallback
            }
        },
        write(value: T): void {
            writeLocalStorage(key, JSON.stringify(value))
        },
        remove(): void {
            removeLocalStorage(key)
        },
    }
}
