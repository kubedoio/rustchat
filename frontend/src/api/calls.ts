import axios from './client'

export interface Call {
    id: string
    channel_id: string
    type: string
    started_at: string
    ended_at?: string
    owner_id: string
}

export interface CallParticipant {
    call_id: string
    user_id: string
    role: string
    joined_at: string
    left_at?: string
    muted: boolean
    raised_hand: boolean
}

export interface CallSession {
    call: Call
    participants: CallParticipant[]
}

export interface CreateMeetingResponse {
    meeting_url: string
    mode: 'new_tab' | 'embed_iframe'
}

export default {
    createMeeting(scope: 'channel' | 'dm', channelId?: string, dmUserId?: string) {
        return axios.post<CreateMeetingResponse>('/video/meetings', {
            scope,
            channel_id: channelId,
            dm_user_id: dmUserId
        })
    },

    createCall(channelId: string, type: string = 'audio') {
        return axios.post<Call>('/calls', { channel_id: channelId, type })
    },

    getCall(id: string) {
        return axios.get<CallSession>(`/calls/${id}`)
    },

    joinCall(id: string) {
        return axios.post<CallParticipant>(`/calls/${id}/join`)
    },

    leaveCall(id: string) {
        return axios.post(`/calls/${id}/leave`)
    },

    endCall(id: string) {
        return axios.post<Call>(`/calls/${id}/end`)
    }
}
