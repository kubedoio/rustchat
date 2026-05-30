import { useMediaQuery } from '@vueuse/core'
import { computed } from 'vue'

export function useBreakpoints() {
  const isMobile = useMediaQuery('(max-width: 767px)')
  const isTablet = useMediaQuery('(min-width: 768px) and (max-width: 1023px)')
  const isDesktop = useMediaQuery('(min-width: 1024px)')

  const isMobileOrTablet = computed(() => isMobile.value || isTablet.value)

  return {
    isMobile,
    isTablet,
    isDesktop,
    isMobileOrTablet,
  }
}
