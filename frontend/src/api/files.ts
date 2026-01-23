import api from './client'

export interface FileInfo {
    id: string
    name: string
    key: string
    mime_type: string
    size: number
    uploader_id: string
    channel_id?: string
    created_at: string
}

export interface FileUploadResponse {
    id: string
    name: string
    mime_type: string
    size: number
    url: string
    thumbnail_url?: string
}

export interface PresignedUploadUrl {
    upload_url: string
    file_key: string
    expires_in: number
}

export const filesApi = {
    upload: (file: File, channelId?: string, onProgress?: (progressEvent: any) => void) => {
        const formData = new FormData()
        formData.append('file', file)
        return api.post<FileUploadResponse>('/files', formData, {
            params: channelId ? { channel_id: channelId } : undefined,
            headers: { 'Content-Type': 'multipart/form-data' },
            onUploadProgress: onProgress,
        })
    },
    getPresignedUrl: (filename: string, contentType: string, channelId?: string) =>
        api.post<PresignedUploadUrl>('/files/presign', {
            filename,
            content_type: contentType,
            channel_id: channelId,
        }),
    get: (id: string) => api.get<FileInfo>(`/files/${id}`),
    getDownloadUrl: (id: string) => api.get<{ url: string; filename: string }>(`/files/${id}/download`),
    delete: (id: string) => api.delete(`/files/${id}`),
}
