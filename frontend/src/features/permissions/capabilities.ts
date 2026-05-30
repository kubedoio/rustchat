import { computed, ref, watchEffect } from 'vue'
import { channelsApi } from '../../api/channels'
import { useAuthStore } from '../../features/auth/stores/authStore'
import { useTeamStore } from '@/features/teams/stores/teamStore'

const TEAM_MANAGER_ROLES = new Set(['system_admin', 'org_admin', 'team_admin', 'admin'])

const CHANNEL_MANAGER_ROLES = new Set(['system_admin', 'org_admin', 'team_admin', 'admin'])

const CHANNEL_CREATOR_ROLES = new Set([
  'system_admin',
  'org_admin',
  'team_admin',
  'admin',
  'member',
])

const TEAM_ADMIN_MEMBERSHIP_ROLES = new Set(['admin', 'owner'])

interface ChannelPermissionSnapshot {
  membershipRole: string | null
  loaded: boolean
  loading: boolean
}

const channelPermissionCache = ref<Record<string, ChannelPermissionSnapshot>>({})

function setChannelPermissionSnapshot(channelId: string, snapshot: ChannelPermissionSnapshot) {
  channelPermissionCache.value = {
    ...channelPermissionCache.value,
    [channelId]: snapshot,
  }
}

export function canManageTeams(role?: string | null) {
  return !!(role && TEAM_MANAGER_ROLES.has(role))
}

export function canCreateTeam(role?: string | null) {
  return canManageTeams(role)
}

export function canCreateChannel(role?: string | null) {
  return !!(role && CHANNEL_CREATOR_ROLES.has(role))
}

export function canManageTeam(options: {
  currentUserRole?: string | null
  currentTeamMembershipRole?: string | null
}) {
  const { currentUserRole, currentTeamMembershipRole } = options

  if (canManageTeams(currentUserRole)) {
    return true
  }

  return !!(currentTeamMembershipRole && TEAM_ADMIN_MEMBERSHIP_ROLES.has(currentTeamMembershipRole))
}

export function canManageChannel(options: {
  currentUserRole?: string | null
  currentUserId?: string | null
  creatorId?: string | null
  membershipRole?: string | null
}) {
  const { currentUserRole, currentUserId, creatorId, membershipRole } = options

  if (currentUserRole && CHANNEL_MANAGER_ROLES.has(currentUserRole)) {
    return true
  }

  if (currentUserId && creatorId && currentUserId === creatorId) {
    return true
  }

  return membershipRole === 'admin' || membershipRole === 'channel_admin'
}

export function clearChannelPermissionCache() {
  channelPermissionCache.value = {}
}

export function useCurrentTeamManagementPermission(teamIdSource: () => string | null | undefined) {
  const authStore = useAuthStore()
  const teamStore = useTeamStore()

  const currentTeamMembershipRole = computed(() => {
    const teamId = teamIdSource()
    if (!teamId || teamStore.currentTeamId !== teamId || !authStore.user?.id) {
      return null
    }

    const membership = teamStore.members.find(member => member.user_id === authStore.user?.id)
    return membership?.role ?? null
  })

  const canManageCurrentTeam = computed(() =>
    canManageTeam({
      currentUserRole: authStore.user?.role,
      currentTeamMembershipRole: currentTeamMembershipRole.value,
    })
  )

  return {
    canManageTeam: canManageCurrentTeam,
    currentTeamMembershipRole,
  }
}

async function ensureChannelPermissionLoaded(channelId: string, userId: string) {
  const existing = channelPermissionCache.value[channelId]
  if (existing?.loaded || existing?.loading) {
    return
  }

  setChannelPermissionSnapshot(channelId, {
    membershipRole: existing?.membershipRole ?? null,
    loaded: false,
    loading: true,
  })

  try {
    const response = await channelsApi.getMembers(channelId)
    const members = Array.isArray(response.data) ? response.data : []
    const currentMember = members.find(member => member.user_id === userId)

    setChannelPermissionSnapshot(channelId, {
      membershipRole: currentMember?.role ?? null,
      loaded: true,
      loading: false,
    })
  } catch {
    setChannelPermissionSnapshot(channelId, {
      membershipRole: null,
      loaded: false,
      loading: false,
    })
  }
}

export function useChannelManagementPermission(
  channelIdSource: () => string | null | undefined,
  creatorIdSource: () => string | null | undefined = () => undefined
) {
  const authStore = useAuthStore()

  const canManageCurrentChannel = computed(() => {
    const channelId = channelIdSource()
    if (!channelId) {
      return false
    }

    const snapshot = channelPermissionCache.value[channelId]

    return canManageChannel({
      currentUserRole: authStore.user?.role,
      currentUserId: authStore.user?.id,
      creatorId: creatorIdSource() ?? null,
      membershipRole: snapshot?.membershipRole ?? null,
    })
  })

  const isLoading = computed(() => {
    const channelId = channelIdSource()
    return !!(channelId && channelPermissionCache.value[channelId]?.loading)
  })

  watchEffect(() => {
    const channelId = channelIdSource()
    const currentUserId = authStore.user?.id
    const currentUserRole = authStore.user?.role
    const creatorId = creatorIdSource()

    if (!channelId || !currentUserId) {
      return
    }

    if (canManageChannel({ currentUserRole, currentUserId, creatorId })) {
      return
    }

    void ensureChannelPermissionLoaded(channelId, currentUserId)
  })

  return {
    canManageChannel: canManageCurrentChannel,
    isLoading,
  }
}
