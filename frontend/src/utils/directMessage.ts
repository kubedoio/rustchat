export function getDirectMessageCounterpartyId(
  channelName?: string | null,
  currentUserId?: string | null
): string | null {
  if (!channelName || !currentUserId) {
    return null
  }

  // Support both canonical "<id1>__<id2>" and legacy/v1 "dm_<id1>_<id2>" formats
  if (channelName.includes('__')) {
    const parts = channelName.split('__')
    if (parts.length !== 2) return null
    const [id1, id2] = parts
    if (!id1 || !id2) return null
    return id1 === currentUserId ? id2 : id2 === currentUserId ? id1 : null
  }

  const parts = channelName.split('_')
  if (parts.length === 3 && parts[0] === 'dm') {
    const [, id1, id2] = parts
    if (!id1 || !id2) return null
    return id1 === currentUserId ? id2 : id2 === currentUserId ? id1 : null
  }

  return null
}
