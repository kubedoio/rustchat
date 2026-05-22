// Presence Repository - Data access for user presence

// Note: Most presence operations are WebSocket-based
// This repository is minimal as presence is real-time

export const presenceRepository = {
  // Presence is primarily WebSocket-driven
  // API endpoints would be for bulk operations or initial load

  async getBulkPresence(): Promise<Map<string, string>> {
    // If there's a bulk presence API, use it here
    // For now, return empty (WebSocket handles real-time updates)
    return new Map()
  },
}
