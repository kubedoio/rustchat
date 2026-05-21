// Theme Repository - Data access for appearance preferences

import { withRetry } from '../../../core/services/retry'
import { preferencesApi, type Preference } from '../../../api/preferences'
import type { Theme, ChatFont, ChatFontSize } from '../types'

const SERVER_PREFERENCE_CATEGORY = 'rustchat_display'
const SERVER_PREFERENCE_THEME = 'theme'
const SERVER_PREFERENCE_FONT = 'font'
const SERVER_PREFERENCE_FONT_SIZE = 'font_size'

export interface AppearancePreferences {
  theme?: Theme
  font?: ChatFont
  fontSize?: ChatFontSize
}

export const themeRepository = {
  // Persist appearance to server
  async saveToServer(theme: Theme, font: ChatFont, fontSize: ChatFontSize): Promise<void> {
    const payload: Preference[] = [
      {
        user_id: 'me',
        category: SERVER_PREFERENCE_CATEGORY,
        name: SERVER_PREFERENCE_THEME,
        value: theme,
      },
      {
        user_id: 'me',
        category: SERVER_PREFERENCE_CATEGORY,
        name: SERVER_PREFERENCE_FONT,
        value: font,
      },
      {
        user_id: 'me',
        category: SERVER_PREFERENCE_CATEGORY,
        name: SERVER_PREFERENCE_FONT_SIZE,
        value: String(fontSize),
      },
    ]

    await withRetry(() => preferencesApi.updatePreferencesV4('me', payload))
  },

  // Load appearance from server
  async loadFromServer(): Promise<AppearancePreferences> {
    return withRetry(async () => {
      const { data: rows } = await preferencesApi.getMyPreferencesMmV4()

      const getValue = (name: string): string | undefined =>
        rows.find(p => p.category === SERVER_PREFERENCE_CATEGORY && p.name === name)?.value

      const rawTheme = getValue(SERVER_PREFERENCE_THEME)
      const rawFont = getValue(SERVER_PREFERENCE_FONT)
      const rawFontSize = getValue(SERVER_PREFERENCE_FONT_SIZE)

      return {
        theme: rawTheme as Theme | undefined,
        font: rawFont as ChatFont | undefined,
        fontSize: rawFontSize ? (Number(rawFontSize) as ChatFontSize) : undefined,
      }
    })
  },
}
